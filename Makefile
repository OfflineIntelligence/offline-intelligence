# Makefile for Offline Intelligence Library
# Multi-language compilation targets

.PHONY: all clean test build-core build-python build-java build-js build-cpp help

# Default target
all: build-core build-python build-java build-js build-cpp

# Help target
help:
	@echo "Offline Intelligence Library Makefile"
	@echo "====================================="
	@echo "Available targets:"
	@echo "  all          - Build all components (default)"
	@echo "  build-core   - Build core library"
	@echo "  build-python - Build Python bindings"
	@echo "  build-java   - Build Java bindings"
	@echo "  build-js     - Build JavaScript bindings"
	@echo "  build-cpp    - Build C++ bindings"
	@echo "  test         - Run all tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  help         - Show this help message"

# Build core library
build-core:
	cargo build --release

# Build Python bindings
build-python:
	cd crates/python-bindings && cargo build --release

# Build Java bindings
build-java:
	cd crates/java-bindings && cargo build --release

# Build JavaScript bindings
build-js:
	cd crates/js-bindings && cargo build --release

# Build C++ bindings
build-cpp:
	cd crates/cpp-bindings && cargo build --release

# Run tests
test:
	cargo test --release

# Clean build artifacts
clean:
	cargo clean
	cd crates/python-bindings && cargo clean
	cd crates/java-bindings && cargo clean
	cd crates/js-bindings && cargo clean
	cd crates/cpp-bindings && cargo clean

# Install (for development)
install-dev:
	cargo install --path crates/offline-intelligence --features cli

# Build documentation
doc:
	cargo doc --release --no-deps

# Check code formatting
fmt:
	cargo fmt --all

# Run clippy lints
lint:
	cargo clippy --all-targets --all-features -- -D warnings