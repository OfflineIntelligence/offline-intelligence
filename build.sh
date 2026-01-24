#!/usr/bin/env bash

# Offline Intelligence Library - Multi-Language Distribution Build Script
# Supports: Rust (native), Python, C++, JavaScript/Node.js, Java

set -e

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$PROJECT_ROOT/crates/offline-intelligence"
BINDINGS_DIR="$PROJECT_ROOT/bindings"
BUILD_DIR="$PROJECT_ROOT/build"
DIST_DIR="$PROJECT_ROOT/dist"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Platform detection
detect_platform() {
    case "$(uname -s)" in
        Darwin*)
            PLATFORM="macos"
            if [[ "$(uname -m)" == "arm64" ]]; then
                ARCH="aarch64"
            else
                ARCH="x86_64"
            fi
            ;;
        Linux*)
            PLATFORM="linux"
            if [[ "$(uname -m)" == "aarch64" ]] || [[ "$(uname -m)" == "arm64" ]]; then
                ARCH="aarch64"
            else
                ARCH="x86_64"
            fi
            ;;
        MINGW*|MSYS*|CYGWIN*)
            PLATFORM="windows"
            ARCH="x86_64"
            ;;
        *)
            log_error "Unsupported platform: $(uname -s)"
            exit 1
            ;;
    esac
    
    log_info "Detected platform: $PLATFORM-$ARCH"
}

# Build Rust library
build_rust() {
    log_info "Building Rust library..."
    
    cd "$CRATE_DIR"
    
    # Build release version
    cargo build --release
    
    # Build for different targets if cross-compilation is needed
    if [[ "$PLATFORM" == "linux" ]]; then
        # Linux x64 and ARM64
        cargo build --release --target x86_64-unknown-linux-gnu
        cargo build --release --target aarch64-unknown-linux-gnu
    elif [[ "$PLATFORM" == "macos" ]]; then
        # macOS Intel and Apple Silicon
        cargo build --release --target x86_64-apple-darwin
        cargo build --release --target aarch64-apple-darwin
    elif [[ "$PLATFORM" == "windows" ]]; then
        # Windows x64
        cargo build --release --target x86_64-pc-windows-msvc
    fi
    
    log_success "Rust library built successfully"
}

# Build Python bindings
build_python() {
    log_info "Building Python bindings..."
    
    cd "$BINDINGS_DIR/python"
    
    # Create build directory
    mkdir -p build
    
    # Build the extension
    python setup.py build_ext --inplace
    
    # Create wheel
    python setup.py bdist_wheel
    
    log_success "Python bindings built successfully"
}

# Build C++ bindings
build_cpp() {
    log_info "Building C++ bindings..."
    
    cd "$BINDINGS_DIR/cpp"
    
    # Create build directory
    mkdir -p build
    cd build
    
    # Configure with CMake
    cmake .. -DCMAKE_BUILD_TYPE=Release
    
    # Build
    cmake --build . --config Release
    
    # Install
    cmake --install . --prefix "$DIST_DIR/cpp"
    
    log_success "C++ bindings built successfully"
}

# Build JavaScript bindings
build_javascript() {
    log_info "Building JavaScript bindings..."
    
    cd "$BINDINGS_DIR/javascript"
    
    # Install dependencies
    npm install
    
    # Build
    npm run build
    
    log_success "JavaScript bindings built successfully"
}

# Build Java bindings
build_java() {
    log_info "Building Java bindings..."
    
    cd "$BINDINGS_DIR/java"
    
    # Build with Maven
    mvn clean compile package
    
    log_success "Java bindings built successfully"
}

# Create distribution packages
create_distribution() {
    log_info "Creating distribution packages..."
    
    # Create distribution directory
    mkdir -p "$DIST_DIR"
    
    # Copy Rust binaries
    cp -r "$CRATE_DIR/target/release" "$DIST_DIR/rust"
    
    # Copy Python wheels
    if [[ -d "$BINDINGS_DIR/python/dist" ]]; then
        cp "$BINDINGS_DIR/python/dist"/*.whl "$DIST_DIR/python/"
    fi
    
    # Copy C++ libraries
    if [[ -d "$DIST_DIR/cpp" ]]; then
        cp -r "$DIST_DIR/cpp" "$DIST_DIR/cpp-lib"
    fi
    
    # Copy JavaScript packages
    if [[ -d "$BINDINGS_DIR/javascript/build" ]]; then
        cp -r "$BINDINGS_DIR/javascript/build" "$DIST_DIR/javascript/"
    fi
    
    # Copy Java JARs
    if [[ -d "$BINDINGS_DIR/java/target" ]]; then
        cp "$BINDINGS_DIR/java/target"/*.jar "$DIST_DIR/java/"
    fi
    
    # Create package manifests
    cat > "$DIST_DIR/MANIFEST.json" << EOF
{
    "name": "offline-intelligence",
    "version": "0.1.0",
    "platforms": ["windows", "macos", "linux"],
    "architectures": ["x86_64", "aarch64"],
    "components": {
        "rust": {
            "type": "native-library",
            "path": "rust/",
            "description": "Native Rust library and executables"
        },
        "python": {
            "type": "bindings",
            "path": "python/",
            "description": "Python language bindings"
        },
        "cpp": {
            "type": "bindings",
            "path": "cpp-lib/",
            "description": "C++ language bindings"
        },
        "javascript": {
            "type": "bindings",
            "path": "javascript/",
            "description": "JavaScript/Node.js bindings"
        },
        "java": {
            "type": "bindings",
            "path": "java/",
            "description": "Java language bindings"
        }
    }
}
EOF
    
    log_success "Distribution packages created in $DIST_DIR"
}

# Main build function
main() {
    log_info "Starting Offline Intelligence Library build process..."
    
    detect_platform
    
    # Clean previous builds
    log_info "Cleaning previous builds..."
    rm -rf "$BUILD_DIR" "$DIST_DIR"
    mkdir -p "$BUILD_DIR" "$DIST_DIR"
    
    # Build components
    build_rust
    build_python
    build_cpp
    build_javascript
    build_java
    
    # Create distribution
    create_distribution
    
    log_success "Build process completed successfully!"
    log_info "Distribution available at: $DIST_DIR"
}

# Run main function
main "$@"