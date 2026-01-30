#!/usr/bin/env bash

# Build script for Offline Intelligence Library - Multi-language compilation
# This script builds the core library and all language bindings

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

echo -e "${BLUE}=== Offline Intelligence Library Build Script ===${NC}"
echo -e "${BLUE}Building core library and all language bindings${NC}"
echo

# Function to print status
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
print_status "Checking prerequisites..."

# Check Rust
if ! command -v rustc &> /dev/null; then
    print_error "Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

print_status "Rust version: $(rustc --version)"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the repository root."
    exit 1
fi

# Build core library
print_status "Building core library..."
cargo build --release

# Build Python bindings
if [ -d "crates/python-bindings" ]; then
    print_status "Building Python bindings..."
    cd crates/python-bindings
    cargo build --release
    cd ../..
else
    print_warning "Python bindings directory not found, skipping..."
fi

# Build Java bindings
if [ -d "crates/java-bindings" ]; then
    print_status "Building Java bindings..."
    cd crates/java-bindings
    cargo build --release
    cd ../..
else
    print_warning "Java bindings directory not found, skipping..."
fi

# Build JavaScript bindings
if [ -d "crates/js-bindings" ]; then
    print_status "Building JavaScript bindings..."
    cd crates/js-bindings
    cargo build --release
    cd ../..
else
    print_warning "JavaScript bindings directory not found, skipping..."
fi

# Build C++ bindings
if [ -d "crates/cpp-bindings" ]; then
    print_status "Building C++ bindings..."
    cd crates/cpp-bindings
    cargo build --release
    cd ../..
else
    print_warning "C++ bindings directory not found, skipping..."
fi

print_status "Build completed successfully!"
echo
print_status "Output files:"
echo "  Core library: target/release/liboffline_intelligence.*"
echo "  Python bindings: crates/python-bindings/target/release/liboffline_intelligence_py.*"
echo "  Java bindings: crates/java-bindings/target/release/liboffline_intelligence_java.*"
echo "  JavaScript bindings: crates/js-bindings/target/release/liboffline_intelligence_js.*"
echo "  C++ bindings: crates/cpp-bindings/target/release/liboffline_intelligence_cpp.*"
echo
print_status "To run tests:"
echo "  cargo test --release"