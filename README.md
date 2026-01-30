# Offline Intelligence Library

A high-performance library for offline AI inference with context management, memory optimization, and multi-format model support.

## Overview

Offline Intelligence Library provides developers with powerful tools for running large language models locally without internet connectivity. The library offers intelligent context management, memory optimization, and hybrid search capabilities across multiple programming languages.

## Features

- **Offline AI Inference**: Run LLMs locally without internet connection
- **Context Management**: Intelligent conversation context optimization
- **Memory Search**: Hybrid semantic and keyword search across conversations
- **Multi-format Support**: Support for GGUF, GGML, ONNX, TensorRT, and Safetensors models
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Multi-language**: Native bindings for Python, Java, JavaScript/Node.js, and C++

## Supported Languages

- **Python** (PyO3 bindings)
- **Java** (JNI bindings)
- **JavaScript/Node.js** (N-API bindings)
- **C++** (C FFI bindings)
- **Rust** (Native library)

## Installation

### Rust

```bash
cargo add offline-intelligence
```

### JavaScript/Node.js

```bash
npm install offline-intelligence-sdk
```

### Java (via JitPack)

**Maven:**
```xml
<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offline-intelligence</artifactId>
    <version>v0.1.2</version>
</dependency>
```

**Gradle:**
```gradle
implementation 'com.github.OfflineIntelligence:offline-intelligence:v0.1.2'
```

### C++

Download the pre-built package from [GitHub Releases](https://github.com/OfflineIntelligence/offline-intelligence/releases):

1. Extract the zip file
2. Add `include/` directory to your compiler's include path
3. Link against `offline_intelligence_cpp.dll`

Or use CMake:
```cmake
find_package(offline-intelligence-cpp REQUIRED)
target_link_libraries(your_target offline-intelligence-cpp)
```

### Python

```bash
pip install offline-intelligence
```

## Quick Start

### Prerequisites

- Rust toolchain (latest stable)
- For language-specific bindings, see individual requirements below

### Building

#### Using Build Scripts

```bash
# Linux/macOS
./build.sh

# Windows
build.bat

# Using Make (Linux/macOS)
make all
```

#### Manual Build

```bash
# Build core library
cargo build --release

# Build specific language bindings
cd crates/python-bindings && cargo build --release
cd crates/java-bindings && cargo build --release
cd crates/js-bindings && cargo build --release
cd crates/cpp-bindings && cargo build --release
```

## Language-Specific Usage

See individual documentation files in the [docs](docs/) directory for detailed usage examples in each language.

## Configuration

Set environment variables before using the library:

```bash
export LLAMA_BIN="/path/to/llama-binary"
export MODEL_PATH="/path/to/model.gguf"
export CTX_SIZE="8192"
export BATCH_SIZE="256"
export THREADS="6"
export GPU_LAYERS="20"
```

## Documentation Structure

All language-specific documentation has been moved to the [docs/](docs/) directory:
- Core library documentation
- Individual language binding guides
- Detailed usage examples

### Core Library
- [Core Library Documentation](docs/OFFLINE_INTELLIGENCE.md)

### Language Bindings
- [Python Bindings](docs/PYTHON_BINDINGS.md)
- [Java Bindings](docs/JAVA_BINDINGS.md)
- [JavaScript Bindings](docs/JS_BINDINGS.md)

### Additional Resources
- [Complete Documentation](COMPLETE_DOCUMENTATION.md)
- [Publishing Guide](PUBLISHING.md)

## Development

### Running Tests

```bash
# Run all tests
cargo test --release

# Run tests for specific crate
cd crates/offline-intelligence && cargo test
```

### Code Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with Rust for performance and reliability
- Uses various ML frameworks for model support
- Inspired by the need for offline AI capabilities