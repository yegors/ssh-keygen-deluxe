package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/pem"
	"fmt"
	"os"
	"runtime"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"golang.org/x/crypto/ssh"
)

type Result struct {
	privateKey ed25519.PrivateKey
	publicKey  ed25519.PublicKey
	sshPubKey  string
	attempts   uint64
}

func main() {
	// Ensure Go uses all available CPU cores
	runtime.GOMAXPROCS(runtime.NumCPU())

	if len(os.Args) < 2 || len(os.Args) > 3 {
		fmt.Fprintf(os.Stderr, "Usage: %s [--ci] <target_sequence>\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  --ci: Enable case-insensitive search\n")
		os.Exit(1)
	}

	var targetSequence string
	var caseInsensitive bool

	if len(os.Args) == 3 {
		if os.Args[1] != "--ci" {
			fmt.Fprintf(os.Stderr, "Usage: %s [--ci] <target_sequence>\n", os.Args[0])
			fmt.Fprintf(os.Stderr, "  --ci: Enable case-insensitive search\n")
			os.Exit(1)
		}
		caseInsensitive = true
		targetSequence = os.Args[2]
	} else {
		targetSequence = os.Args[1]
	}

	if targetSequence == "" {
		fmt.Fprintf(os.Stderr, "Error: target sequence cannot be empty\n")
		os.Exit(1)
	}

	numWorkers := runtime.NumCPU() * 3

	searchType := "case-sensitive"
	if caseInsensitive {
		searchType = "case-insensitive"
	}
	fmt.Printf("Searching for ed25519 key containing: %s (%s)\n", targetSequence, searchType)
	fmt.Printf("Using %d cores, %d workers\n", runtime.NumCPU(), numWorkers)

	resultChan := make(chan Result, 1)
	done := make(chan struct{})

	var totalAttempts uint64
	var wg sync.WaitGroup

	// Start progress reporter
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()

		lastAttempts := uint64(0)
		startTime := time.Now()

		for {
			select {
			case <-done:
				return
			case <-ticker.C:
				current := atomic.LoadUint64(&totalAttempts)
				rate := current - lastAttempts
				elapsed := time.Since(startTime)
				avgRate := float64(current) / elapsed.Seconds()

				fmt.Printf("\rAttempts: %d | Rate: %d/s | Avg: %.0f/s | Elapsed: %s",
					current, rate, avgRate, elapsed.Truncate(time.Second))
				lastAttempts = current
			}
		}
	}()

	// Start workers
	for i := 0; i < numWorkers; i++ {
		wg.Add(1)
		go worker(i, targetSequence, caseInsensitive, &totalAttempts, resultChan, done, &wg)
	}

	// Wait for result
	result := <-resultChan
	close(done)
	wg.Wait()

	fmt.Printf("\n\nMatch found after %d attempts!\n", result.attempts)

	// Write private key
	privateKeyPEM, err := ssh.MarshalPrivateKey(result.privateKey, "")
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error marshaling private key: %v\n", err)
		os.Exit(1)
	}

	privateKeyBytes := pem.EncodeToMemory(privateKeyPEM)
	err = os.WriteFile("id_ed25519", privateKeyBytes, 0600)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error writing private key: %v\n", err)
		os.Exit(1)
	}

	// Write public key
	err = os.WriteFile("id_ed25519.pub", []byte(result.sshPubKey), 0644)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error writing public key: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Keys written to id_ed25519 and id_ed25519.pub\n")
	fmt.Printf("Public key: %s\n", strings.TrimSpace(result.sshPubKey))

	finalAttempts := atomic.LoadUint64(&totalAttempts)
	fmt.Printf("Total attempts across all workers: %d\n", finalAttempts)
}

func worker(id int, targetSequence string, caseInsensitive bool, totalAttempts *uint64, resultChan chan Result, done chan struct{}, wg *sync.WaitGroup) {
	defer wg.Done()

	attempts := uint64(0)
	batchSize := uint64(1000) // Smaller batches to reduce memory pressure

	var targetBytes []byte
	if caseInsensitive {
		targetBytes = []byte(strings.ToLower(targetSequence))
	} else {
		targetBytes = []byte(targetSequence)
	}

	for {
		// Check for shutdown signal less frequently
		select {
		case <-done:
			return
		default:
		}

		// Process a batch without checking done channel for maximum performance
		for i := uint64(0); i < batchSize; i++ {
			// Generate ed25519 keypair directly
			pubKey, privKey, err := ed25519.GenerateKey(rand.Reader)
			if err != nil {
				continue
			}

			attempts++

			// Convert to SSH format - this is the expensive operation
			sshPubKey, err := ssh.NewPublicKey(pubKey)
			if err != nil {
				continue
			}

			// Get bytes directly to avoid string allocation
			sshPubKeyBytes := ssh.MarshalAuthorizedKey(sshPubKey)

			var match bool
			if caseInsensitive {
				match = containsBytesIgnoreCase(sshPubKeyBytes, targetBytes)
			} else {
				match = containsBytes(sshPubKeyBytes, targetBytes)
			}

			if match {
				// Only convert to string when we have a match
				sshPubKeyString := string(sshPubKeyBytes)

				select {
				case resultChan <- Result{
					privateKey: privKey,
					publicKey:  pubKey,
					sshPubKey:  sshPubKeyString,
					attempts:   atomic.LoadUint64(totalAttempts) + attempts,
				}:
					return
				case <-done:
					return
				}
			}
		}

		// Update global counter after processing the batch
		atomic.AddUint64(totalAttempts, batchSize)
		attempts = 0
	}
}

// Fast case-sensitive byte slice contains check
func containsBytes(haystack, needle []byte) bool {
	if len(needle) == 0 {
		return true
	}
	if len(needle) > len(haystack) {
		return false
	}

	for i := 0; i <= len(haystack)-len(needle); i++ {
		found := true
		for j := 0; j < len(needle); j++ {
			if haystack[i+j] != needle[j] {
				found = false
				break
			}
		}
		if found {
			return true
		}
	}
	return false
}

// Fast case-insensitive byte slice contains check
func containsBytesIgnoreCase(haystack, needle []byte) bool {
	if len(needle) == 0 {
		return true
	}
	if len(needle) > len(haystack) {
		return false
	}

	for i := 0; i <= len(haystack)-len(needle); i++ {
		found := true
		for j := 0; j < len(needle); j++ {
			// Convert both bytes to lowercase for comparison
			haystackChar := toLowerCase(haystack[i+j])
			needleChar := needle[j] // Already converted to lowercase in worker
			if haystackChar != needleChar {
				found = false
				break
			}
		}
		if found {
			return true
		}
	}
	return false
}

// Fast ASCII lowercase conversion
func toLowerCase(b byte) byte {
	if b >= 'A' && b <= 'Z' {
		return b + ('a' - 'A')
	}
	return b
}
