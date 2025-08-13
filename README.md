# SSH Key Generator

A high-performance SSH Ed25519 key generator that searches for keys containing specific sequences in their public key representation.

## Overview

This tool generates Ed25519 SSH key pairs and searches for public keys containing a target sequence. It uses parallel processing to maximize CPU utilization and generate keys as fast as possible.

## Features

- **High Performance**: Uses multiple worker threads (3x CPU cores) for maximum throughput
- **Case Sensitivity Options**: Supports both case-sensitive and case-insensitive search
- **Real-time Progress**: Shows attempts per second, average rate, and elapsed time
- **Optimized Search**: Uses byte-level operations for fast string matching
- **Standard SSH Format**: Generates standard Ed25519 private/public key pairs

## Usage

### Basic Usage (Case-Sensitive)
```bash
./ssh-keygen <target_sequence>
```

### Case-Insensitive Search
```bash
./ssh-keygen --ci <target_sequence>
```

## Examples

```bash
# Find keys containing "hello" (case-sensitive)
./ssh-keygen hello

# Find keys containing "hello" in any case (Hello, HELLO, hELLo, etc.)
./ssh-keygen --ci hello

# Find keys containing "test123"
./ssh-keygen test123
```

## Output

The program displays:
- Target sequence and search type
- CPU cores and worker count
- Real-time progress with attempts/rate/average/elapsed time
- Final results when a match is found

### Sample Output
```
Searching for ed25519 key containing: hello (case-sensitive)
Using 28 cores, 84 workers
Attempts: 1230733000 | Rate: 373000/s | Avg: 383525/s | Elapsed: 53m20s

Match found after 1230733000 attempts!
Keys written to id_ed25519 and id_ed25519.pub
Public key: ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHelloXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX
Total attempts across all workers: 1230733000
```

## Generated Files

When a match is found, two files are created:
- `id_ed25519` - Private key (600 permissions)
- `id_ed25519.pub` - Public key (644 permissions)

## Performance

The application automatically:
- Uses all available CPU cores
- Scales worker threads to 3x CPU core count
- Processes keys in optimized batches
- Minimizes memory allocations for maximum speed

Typical performance ranges from 300,000 to 500,000+ key checks per second depending on hardware.

## Building

```bash
go build -o ssh-keygen ssh-keygen.go
```

## Requirements

- Go 1.16 or later
- Unix-like system (Linux, macOS)

## Notes

- Longer target sequences will take exponentially more time to find
- Case-insensitive searches may be slightly slower than case-sensitive
- The program uses cryptographically secure random number generation
- Generated keys are fully compatible with standard SSH implementations
