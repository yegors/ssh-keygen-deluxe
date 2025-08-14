#!/bin/bash

# SSH Key Generator - Go Build Script
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

# Function to show usage
show_usage() {
    echo "SSH Key Generator - Go Build Script"
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

# Function to build Go version
build_go() {
    local target_arch="$1"
    local output_name="ssh-keygen-go"
    
    # Create dist directory if it doesn't exist
    mkdir -p dist
    
    if ! check_command go; then
        echo "Error: Go is not installed. Please install Go 1.16 or later." >&2
        return 1
    fi
    
    if [ ! -d "src-go" ]; then
        echo "Error: src-go directory not found. Run this script from the project root." >&2
        return 1
    fi
    
    cd src-go
    
    if [ -n "$target_arch" ]; then
        IFS='/' read -r target_os target_cpu <<< "$target_arch"
        export GOOS="$target_os"
        export GOARCH="$target_cpu"
        output_name="ssh-keygen-go-${target_os}-${target_cpu}"
        
        # Add .exe extension for Windows
        if [ "$target_os" = "windows" ]; then
            output_name="${output_name}.exe"
        fi
    fi
    
    # Build with optimization flags
    go build -ldflags="-s -w" -o "../dist/$output_name" main.go
    
    cd ..
    
    # Make executable (Unix-like systems)
    if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "win32" ]]; then
        chmod +x "dist/$output_name"
    fi
    
    echo "Go build completed: dist/$output_name"
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
    
    if build_go "$target_arch"; then
        echo "Build completed successfully!"
    else
        echo "Build failed!" >&2
        exit 1
    fi
}

# Check if we're in the right directory
if [ ! -d "src-go" ]; then
    echo "Error: This script must be run from the project root directory" >&2
    echo "Expected structure: src-go/ directory with Go source code" >&2
    exit 1
fi

# Run main function with all arguments
main "$@"