#!/usr/bin/env bash

# Publish script for crates.io
# This script publishes the core library to crates.io

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo -e "${BLUE}=== Publishing to crates.io ===${NC}"
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

# Check if we're logged in to crates.io
print_status "Checking crates.io login status..."
if ! cargo owner --list offline-intelligence &>/dev/null; then
    print_error "Not logged in to crates.io. Please run: cargo login"
    exit 1
fi

# Check version
VERSION=$(grep '^version =' crates/offline-intelligence/Cargo.toml | head -1 | cut -d '"' -f 2)
print_status "Publishing version: $VERSION"

# Build and test
print_status "Running tests..."
cd crates/offline-intelligence
cargo test --release

# Check for unpublished changes
print_status "Checking for unpublished changes..."
if cargo package --list --allow-dirty &>/dev/null; then
    print_status "Package checks passed"
else
    print_error "Package validation failed"
    exit 1
fi

# Dry run first
print_status "Performing dry run..."
if cargo publish --dry-run; then
    print_status "Dry run successful"
else
    print_error "Dry run failed"
    exit 1
fi

# Confirm publish
echo
print_warning "About to publish offline-intelligence v$VERSION to crates.io"
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_status "Publish cancelled"
    exit 0
fi

# Publish
print_status "Publishing to crates.io..."
if cargo publish; then
    print_status "Successfully published offline-intelligence v$VERSION to crates.io!"
    echo "Package URL: https://crates.io/crates/offline-intelligence"
else
    print_error "Failed to publish to crates.io"
    exit 1
fi