#!/bin/bash

# SSH Key Generator - Rust Build Script
set -e

# Function to detect current architecture
detect_arch() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case $arch in
        x86_64|amd64)
            arch="amd64"
            ;;
        arm64|aarch64)
            arch="arm64"
            ;;
        arm*)
            arch="arm"
            ;;
        *)
            echo "Error: Unsupported architecture: $arch" >&2
            exit 1
            ;;
    esac
    
    echo "${os}/${arch}"
}

# Function to check if command exists
check_command() {
    if ! command -v "$1" &> /dev/null; then
        return 1
    fi
    return 0
}

# Function to convert arch format for Rust targets
get_rust_target() {
    local target_arch="$1"
    
    case "$target_arch" in
        linux/amd64)
            echo "x86_64-unknown-linux-gnu"
            ;;
        linux/arm64)
            echo "aarch64-unknown-linux-gnu"
            ;;
        linux/arm)
            echo "armv7-unknown-linux-gnueabihf"
            ;;
        darwin/amd64)
            echo "x86_64-apple-darwin"
            ;;
        darwin/arm64)
            echo "aarch64-apple-darwin"
            ;;
        windows/amd64)
            echo "x86_64-pc-windows-msvc"
            ;;
        windows/arm64)
            echo "aarch64-pc-windows-msvc"
            ;;
        *)
            echo ""
            ;;
    esac
}

# Function to show usage
show_usage() {
    echo "SSH Key Generator - Rust Build Script"
    echo ""
    echo "Usage: $0 [ARCHITECTURE]"
    echo ""
    echo "ARCHITECTURE (optional):"
    echo "  Auto-detected if not specified"
    echo "  linux/amd64    Linux x86_64"
    echo "  linux/arm64    Linux ARM64"
    echo "  linux/arm      Linux ARM"
    echo "  darwin/amd64   macOS x86_64"
    echo "  darwin/arm64   macOS ARM64 (Apple Silicon)"
    echo "  windows/amd64  Windows x86_64"
    echo "  windows/arm64  Windows ARM64"
    echo ""
    echo "Examples:"
    echo "  $0                    # Build for current architecture"
    echo "  $0 linux/arm64       # Build for Linux ARM64"
}

# Function to build Rust version
build_rust() {
    local target_arch="$1"
    local output_name="ssh-keygen-rust"
    
    # Create dist directory if it doesn't exist
    mkdir -p dist
    
    if ! check_command cargo; then
        echo "Error: Rust/Cargo is not installed. Please install Rust toolchain." >&2
        echo "Visit: https://rustup.rs/" >&2
        return 1
    fi
    
    if ! check_command rustc; then
        echo "Error: Rust compiler not found. Please install Rust toolchain." >&2
        return 1
    fi
    
    if [ ! -d "src-rust" ]; then
        echo "Error: src-rust directory not found. Run this script from the project root." >&2
        return 1
    fi
    
    cd src-rust
    
    # Set CARGO_TARGET_DIR to use dist for all compilation artifacts
    export CARGO_TARGET_DIR="../dist/target"
    
    if [ -n "$target_arch" ]; then
        local rust_target=$(get_rust_target "$target_arch")
        
        if [ -n "$rust_target" ]; then
            # Check if target is installed
            if ! rustup target list --installed | grep -q "$rust_target"; then
                rustup target add "$rust_target"
            fi
            
            # Build with target
            cargo build --release --target "$rust_target"
            
            # Copy binary with appropriate naming
            local binary_path="../dist/target/$rust_target/release/ssh-keygen"
            local binary_path_exe="../dist/target/$rust_target/release/ssh-keygen.exe"
            
            if [ -f "$binary_path" ]; then
                cp "$binary_path" "../dist/ssh-keygen-rust-${target_arch//\//-}"
                output_name="ssh-keygen-rust-${target_arch//\//-}"
            elif [ -f "$binary_path_exe" ]; then
                cp "$binary_path_exe" "../dist/ssh-keygen-rust-${target_arch//\//-}.exe"
                output_name="ssh-keygen-rust-${target_arch//\//-}.exe"
            else
                echo "Error: Failed to find compiled binary" >&2
                return 1
            fi
        else
            cargo build --release
            
            if [ -f "../dist/target/release/ssh-keygen" ]; then
                cp "../dist/target/release/ssh-keygen" "../dist/$output_name"
            elif [ -f "../dist/target/release/ssh-keygen.exe" ]; then
                cp "../dist/target/release/ssh-keygen.exe" "../dist/$output_name.exe"
                output_name="$output_name.exe"
            else
                echo "Error: Failed to find compiled binary" >&2
                return 1
            fi
        fi
    else
        cargo build --release
        
        if [ -f "../dist/target/release/ssh-keygen" ]; then
            cp "../dist/target/release/ssh-keygen" "../dist/$output_name"
        elif [ -f "../dist/target/release/ssh-keygen.exe" ]; then
            cp "../dist/target/release/ssh-keygen.exe" "../dist/$output_name.exe"
            output_name="$output_name.exe"
        else
            echo "Error: Failed to find compiled binary" >&2
            return 1
        fi
    fi
    
    cd ..
    
    # Make executable (Unix-like systems)
    if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "win32" && "$output_name" != *.exe ]]; then
        chmod +x "dist/$output_name"
    fi
    
    echo "Rust build completed: dist/$output_name"
    return 0
}

# Main script logic
main() {
    local target_arch="$1"
    
    case "$target_arch" in
        -h|--help|help)
            show_usage
            exit 0
            ;;
    esac
    
    if build_rust "$target_arch"; then
        echo "Build completed successfully!"
    else
        echo "Build failed!" >&2
        exit 1
    fi
}

# Check if we're in the right directory
if [ ! -d "src-rust" ]; then
    echo "Error: This script must be run from the project root directory" >&2
    echo "Expected structure: src-rust/ directory with Rust source code" >&2
    exit 1
fi

# Run main function with all arguments
main "$@"