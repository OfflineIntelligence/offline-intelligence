#!/usr/bin/env bash

# Comprehensive publish script for all platforms
# This script publishes the library to PyPI, npm, crates.io, and prepares for Maven/JitPack

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo -e "${BLUE}=== Offline Intelligence Library - Multi-Platform Publish ===${NC}"
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

# Check Git status
if [[ -n $(git status --porcelain) ]]; then
    print_warning "Working directory has uncommitted changes"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_status "Publish cancelled"
        exit 0
    fi
fi

# Get version
VERSION=$(grep '^version = ' crates/offline-intelligence/Cargo.toml | head -1 | cut -d '"' -f 2)
print_status "Publishing version: $VERSION"

# Confirm publish
echo
print_warning "About to publish Offline Intelligence Library v$VERSION to multiple platforms"
print_warning "This will:"
print_warning "  - Create and push Git tag v$VERSION"
print_warning "  - Publish to crates.io"
print_warning "  - Publish to PyPI"
print_warning "  - Publish to npm"
print_warning "  - Prepare Maven/JitPack release"
echo
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_status "Publish cancelled"
    exit 0
fi

# Build all components
print_status "Building all components..."
./build.sh

# Create Git tag
print_status "Creating Git tag..."
git tag -a "v$VERSION" -m "Release version $VERSION"
git push origin "v$VERSION"

# Publish to crates.io
print_status "Publishing to crates.io..."
cd crates/offline-intelligence
if cargo publish --dry-run; then
    cargo publish
    print_status "Successfully published to crates.io"
else
    print_error "Failed to publish to crates.io"
fi
cd ../..

# Publish to PyPI
if command -v twine &> /dev/null; then
    print_status "Publishing to PyPI..."
    cd crates/python-bindings
    python setup.py sdist bdist_wheel
    if twine upload --repository pypi dist/*; then
        print_status "Successfully published to PyPI"
    else
        print_error "Failed to publish to PyPI"
    fi
    cd ../..
else
    print_warning "twine not found, skipping PyPI publish"
fi

# Publish to npm
if command -v npm &> /dev/null; then
    print_status "Publishing to npm..."
    cd crates/js-bindings
    if npm publish; then
        print_status "Successfully published to npm"
    else
        print_error "Failed to publish to npm"
    fi
    cd ../..
else
    print_warning "npm not found, skipping npm publish"
fi

# Prepare Maven/JitPack
print_status "Preparing Maven/JitPack release..."
cd crates/java-bindings
# Create release on GitHub will trigger JitPack build
print_status "Push changes to GitHub to trigger JitPack build"
cd ../..

# Summary
echo
print_status "=== Publish Summary ==="
print_status "Version: $VERSION"
print_status "Git tag: v$VERSION created and pushed"
print_status "crates.io: Published (if successful)"
print_status "PyPI: Published (if successful)"
print_status "npm: Published (if successful)"
print_status "Maven/JitPack: Ready for GitHub release"
echo
print_status "Next steps:"
print_status "1. Create GitHub release for tag v$VERSION"
print_status "2. JitPack will automatically build Java bindings"
print_status "3. Verify all packages are available"
print_status "4. Update documentation if needed"