#!/usr/bin/env bash

# Offline Intelligence - Multi-Ecosystem Deployment Script (Unix/Linux/macOS)
# Deploys to: crates.io, PyPI, npm, Maven Central

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[DEPLOY]${NC} $1"
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

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$PROJECT_ROOT/crates/offline-intelligence"
DIST_DIR="$PROJECT_ROOT/dist"
LOG_DIR="$DIST_DIR/logs"

log_info "Starting Offline Intelligence multi-ecosystem deployment..."
log_info "Repository: https://github.com/OfflineIntelligence/offline-intelligence"

# Create distribution directory
log_info "Creating distribution directory..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR" "$LOG_DIR"

# Check prerequisites
log_info "Checking deployment prerequisites..."
MISSING_TOOLS=()

command -v cargo >/dev/null 2>&1 || MISSING_TOOLS+=("cargo")
command -v python3 >/dev/null 2>&1 || MISSING_TOOLS+=("python3")
command -v npm >/dev/null 2>&1 || MISSING_TOOLS+=("npm")
command -v mvn >/dev/null 2>&1 || MISSING_TOOLS+=("maven")
python3 -c "import twine" 2>/dev/null || MISSING_TOOLS+=("twine")

if [ ${#MISSING_TOOLS[@]} -ne 0 ]; then
    log_error "Missing required deployment tools: ${MISSING_TOOLS[*]}"
    log_error "Please install missing dependencies:"
    log_error "- cargo (Rust)"
    log_error "- python3 + twine (Python)"
    log_error "- npm (JavaScript)"
    log_error "- maven (Java)"
    exit 1
fi

log_success "All deployment tools available"

# Build all components first
log_info "Building all components..."
bash "$PROJECT_ROOT/build.sh"
if [ $? -ne 0 ]; then
    log_error "Build failed, deployment aborted"
    exit 1
fi

# Deploy to Crates.io (Rust)
log_info "Deploying Rust crate to crates.io..."
cd "$CRATE_DIR"

cargo publish --dry-run --allow-dirty > "$LOG_DIR/crates_io_dry_run.log" 2>&1
if [ $? -ne 0 ]; then
    log_error "Crates.io dry run failed, check logs"
    cat "$LOG_DIR/crates_io_dry_run.log"
    exit 1
fi

# Uncomment the next line for actual deployment
# cargo publish --allow-dirty

log_success "Rust crate ready for crates.io deployment"

# Deploy Python package to PyPI
log_info "Deploying Python package to PyPI..."
cd "$PROJECT_ROOT/bindings/python"
rm -rf build dist

python3 setup.py sdist bdist_wheel > "$LOG_DIR/python_build.log" 2>&1
if [ $? -ne 0 ]; then
    log_error "Python package build failed"
    cat "$LOG_DIR/python_build.log"
    exit 1
fi

python3 -m twine check dist/* > "$LOG_DIR/python_check.log" 2>&1
if [ $? -ne 0 ]; then
    log_error "Python package validation failed"
    cat "$LOG_DIR/python_check.log"
    exit 1
fi

# Uncomment for actual PyPI deployment
# python3 -m twine upload dist/*

log_success "Python package ready for PyPI deployment"

# Deploy JavaScript package to npm
log_info "Deploying JavaScript package to npm..."
cd "$PROJECT_ROOT/bindings/javascript"

npm pack --dry-run > "$LOG_DIR/npm_pack.log" 2>&1
if [ $? -ne 0 ]; then
    log_error "npm package validation failed"
    cat "$LOG_DIR/npm_pack.log"
    exit 1
fi

# Uncomment for actual npm deployment
# npm publish

log_success "JavaScript package ready for npm deployment"

# Deploy Java package to Maven Central
log_info "Deploying Java package to Maven Central..."
cd "$PROJECT_ROOT/bindings/java"

mvn clean compile test > "$LOG_DIR/java_build.log" 2>&1
if [ $? -ne 0 ]; then
    log_error "Java package build failed"
    cat "$LOG_DIR/java_build.log"
    exit 1
fi

mvn package > "$LOG_DIR/java_package.log" 2>&1
if [ $? -ne 0 ]; then
    log_error "Java package creation failed"
    cat "$LOG_DIR/java_package.log"
    exit 1
fi

# Uncomment for actual Maven deployment
# mvn clean deploy -P release

log_success "Java package ready for Maven Central deployment"

# Create cross-platform distribution
log_info "Creating cross-platform distribution packages..."

# Copy Rust artifacts
mkdir -p "$DIST_DIR/rust"
cp "$CRATE_DIR/target/release/offline-intelligence" "$DIST_DIR/rust/" 2>/dev/null || true
cp "$CRATE_DIR/target/release/liboffline_intelligence.rlib" "$DIST_DIR/rust/" 2>/dev/null || true

# Copy Python artifacts
mkdir -p "$DIST_DIR/python"
if ls "$PROJECT_ROOT/bindings/python/dist/"* 1> /dev/null 2>&1; then
    cp "$PROJECT_ROOT/bindings/python/dist/"* "$DIST_DIR/python/"
fi

# Copy JavaScript artifacts
mkdir -p "$DIST_DIR/javascript"
npm pack > "$LOG_DIR/npm_pack_final.log" 2>&1
if ls *.tgz 1> /dev/null 2>&1; then
    mv *.tgz "$DIST_DIR/javascript/"
fi

# Copy Java artifacts
mkdir -p "$DIST_DIR/java"
if ls "$PROJECT_ROOT/bindings/java/target/"*.jar 1> /dev/null 2>&1; then
    cp "$PROJECT_ROOT/bindings/java/target/"*.jar "$DIST_DIR/java/"
fi

# Create deployment manifest
cat > "$DIST_DIR/DEPLOYMENT_MANIFEST.json" << EOF
{
  "name": "offline-intelligence",
  "version": "0.1.1",
  "repository": "https://github.com/OfflineIntelligence/offline-intelligence",
  "timestamp": "$(date)",
  "deployment_status": {
    "rust": "ready_for_crates_io",
    "python": "ready_for_pypi",
    "javascript": "ready_for_npm",
    "java": "ready_for_maven_central"
  },
  "artifacts": {
    "rust": {
      "executable": "rust/offline-intelligence",
      "library": "rust/liboffline_intelligence.rlib"
    },
    "python": {
      "wheels": "python/*.whl",
      "source": "python/*.tar.gz"
    },
    "javascript": {
      "package": "javascript/*.tgz"
    },
    "java": {
      "jars": "java/*.jar"
    }
  }
}
EOF

# Create deployment summary
cat > "$DIST_DIR/DEPLOYMENT_SUMMARY.txt" << EOF
OFFLINE INTELLIGENCE - DEPLOYMENT SUMMARY
========================================
Repository: https://github.com/OfflineIntelligence/offline-intelligence
Version: 0.1.1
Timestamp: $(date)

DEPLOYMENT STATUS:
- Rust (crates.io): READY - Run 'cargo publish' in crates/offline-intelligence
- Python (PyPI): READY - Run 'twine upload dist/*' in bindings/python
- JavaScript (npm): READY - Run 'npm publish' in bindings/javascript
- Java (Maven Central): READY - Run 'mvn clean deploy -P release' in bindings/java

DISTRIBUTION ARTIFACTS:
- Executables: $DIST_DIR/rust/
- Python packages: $DIST_DIR/python/
- JavaScript packages: $DIST_DIR/javascript/
- Java packages: $DIST_DIR/java/

LOG FILES:
- Build logs: $LOG_DIR/
- Deployment manifest: $DIST_DIR/DEPLOYMENT_MANIFEST.json
EOF

log_success "Multi-ecosystem deployment preparation completed!"
log_info "Distribution created at: $DIST_DIR"
log_info "See DEPLOYMENT_SUMMARY.txt for detailed instructions"
log_info "Manual deployment commands are commented out - uncomment to deploy"