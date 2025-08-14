# SSH Key Generator

A high-performance SSH Ed25519 key generator that searches for keys containing specific sequences in their public key representation. Available in both **Rust** and **Go** implementations.

## Performance Comparison

| Implementation | Performance | Improvement |
|---------------|-------------|-------------|
| **🦀 Rust** | **1,100,000+/s** | **177% faster** |
| 🐹 Go | ~396,000/s | Baseline |

The Rust implementation delivers **2.77x performance** over the Go version.

## What It Does

This tool generates Ed25519 SSH key pairs and searches for public keys containing a target sequence. Both implementations use parallel processing to maximize CPU utilization.

## Requirements

### Rust Implementation
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Go Implementation
```bash
# Install Go 1.16 or later
# Visit: https://golang.org/dl/
```

## Building

### Quick Build
```bash
# Rust (recommended for performance)
./build-rust.sh

# Go (stable and fast)
./build-go.sh
```

### Cross-Platform Building
```bash
# Example: Build for Linux ARM64
./build-rust.sh linux/arm64
./build-go.sh linux/arm64

# Supported architectures:
# linux/amd64, linux/arm64, darwin/amd64, darwin/arm64, windows/amd64
```

## Usage

Both implementations have identical command-line interfaces:

```bash
# Basic usage (case-sensitive)
./dist/ssh-keygen-rust hello
./dist/ssh-keygen-go hello

# Case-insensitive search
./dist/ssh-keygen-rust --ci hello
./dist/ssh-keygen-go --ci hello
```

## Output

The program displays real-time progress and results:

```
Searching for ed25519 key containing: hello (case-sensitive)
Using 28 cores, 84 workers
Attempts: 1230733000 | Rate: 1100000/s | Avg: 1101044/s | Elapsed: 18m32s

Match found after 1230733000 attempts!
Keys written to id_ed25519 and id_ed25519.pub
Public key: ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHelloXxXxXxXxXxXxXxXx...
Total attempts across all workers: 1230733000
```

## Generated Files

When a match is found, two files are created:
- **`id_ed25519`** - Private key (600 permissions)
- **`id_ed25519.pub`** - Public key (644 permissions)

## Performance Benchmarks

**Test Environment**: 28-core system, 84 workers
- **Rust**: 1,101,044 iterations/second average
- **Go**: 396,673 iterations/second average
- **Performance Ratio**: 2.77x (177% improvement)

## System Requirements

**Minimum:**
- Multi-core processor (2+ cores)
- 512MB available memory
- Linux, macOS, or Windows

**Optimal:**
- 8+ cores for maximum throughput
- 2GB+ RAM for large-scale key generation
- SSD for faster binary loading

## Notes

- Longer target sequences take exponentially more time to find
- Case-insensitive searches may be slightly slower
- Uses cryptographically secure random number generation
- Generated keys are fully compatible with standard SSH implementations
- Performance scales linearly with CPU cores

## Recommendation

Use the **Rust implementation** for maximum performance, especially for longer target sequences. Use the **Go implementation** for simpler deployment or when Go toolchain integration is preferred.
