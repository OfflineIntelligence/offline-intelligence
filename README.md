# Offline Intelligence Library

A highperformance library for offline AI inference with context management, memory optimization, and multiformat model support.

## Overview

Offline Intelligence Library provides developers with powerful tools for running large language models locally without internet connectivity. The library offers intelligent context management, memory optimization, and hybrid search capabilities across multiple programming languages.

## Features

 **Offline AI Inference**: Run LLMs locally without internet connection
 **Context Management**: Intelligent conversation context optimization
 **Memory Search**: Hybrid semantic and keyword search across conversations
 **Multiformat Support**: Support for GGUF, GGML, ONNX, TensorRT, and Safetensors models
 **Crossplatform**: Works on Windows, macOS, and Linux
 **Multilanguage**: Native bindings for Python, Java, JavaScript/Node.js, and C++

## Supported Languages

 **Python** (PyO3 bindings)
 **Java** (JNI bindings)
 **JavaScript/Node.js** (NAPI bindings)
 **C++** (C FFI bindings)
 **Rust** (Native library)

## Installation

### Rust

```bash
cargo add offlineintelligence
```

### JavaScript/Node.js

```bash
npm install offlineintelligencesdk
```

### Java (via JitPack)

**Maven:**
```xml
<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offlineintelligence</artifactId>
    <version>v0.1.2</version>
</dependency>
```

**Gradle:**
```gradle
implementation 'com.github.OfflineIntelligence:offlineintelligence:v0.1.2'
```

### C++

Download the prebuilt package from [GitHub Releases](https://github.com/OfflineIntelligence/offlineintelligence/releases):

1. Extract the zip file
2. Add `include/` directory to your compiler's include path
3. Link against `offline_intelligence_cpp.dll`

Or use CMake:
```cmake
find_package(offlineintelligencecpp REQUIRED)
target_link_libraries(your_target offlineintelligencecpp)
```

### Python

```bash
pip install offlineintelligence
```

## Quick Start

### Prerequisites

 Rust toolchain (latest stable)
 For languagespecific bindings, see individual requirements below

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
cargo build release

# Build specific language bindings
cd crates/pythonbindings && cargo build release
cd crates/javabindings && cargo build release
cd crates/jsbindings && cargo build release
cd crates/cppbindings && cargo build release
```

## LanguageSpecific Usage

See individual documentation files in the [docs](docs/) directory for detailed usage examples in each language.

## Configuration

Set environment variables before using the library:

```bash
export LLAMA_BIN="/path/to/llamabinary"
export MODEL_PATH="/path/to/model.gguf"
export CTX_SIZE="8192"
export BATCH_SIZE="256"
export THREADS="6"
export GPU_LAYERS="20"
```

## Documentation Structure

All languagespecific documentation has been moved to the [docs/](docs/) directory:
 Core library documentation
 Individual language binding guides
 Detailed usage examples

### Getting Started (Start Here)
 [Universal Getting Started Guide](docs/GETTING_STARTED.md)  Complete setup for all platforms
 [Environment Setup](docs/ENVIRONMENT_SETUP.md)  Configuration and variables
 [Model Installation](docs/MODEL_INSTALLATION.md)  AI model download and setup

### PlatformSpecific Guides
 [Rust Development](docs/RUST_GETTING_STARTED.md)  Best performance, full control
 [Python Development](docs/PYTHON_GETTING_STARTED.md)  Easy to use, great for prototyping
 [JavaScript Development](docs/JAVASCRIPT_GETTING_STARTED.md)  Web and Node.js applications
 [Java Development](docs/JAVA_GETTING_STARTED.md)  Enterprise applications
 [C++ Development](docs/CPP_GETTING_STARTED.md)  Systemlevel integration

### Additional Resources
 [Core Library Documentation](docs/OFFLINE_INTELLIGENCE.md)
 [Complete Documentation](COMPLETE_DOCUMENTATION.md)
 [Publishing Guide](PUBLISHING.md)

## Development

### Running Tests

```bash
# Run all tests
cargo test release

# Run tests for specific crate
cd crates/offlineintelligence && cargo test
```

### Code Formatting

```bash
# Format all code
cargo fmt all

# Check formatting
cargo fmt all  check
```

### Linting

```bash
# Run clippy
cargo clippy alltargets allfeatures  D warnings
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout b feature/amazingfeature`)
3. Commit your changes (`git commit m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazingfeature`)
5. Open a Pull Request

## License

This project is licensed under the Apache 2.0 License  see the [LICENSE](LICENSE) file for details.

## Acknowledgments

 Built with Rust for performance and reliability
 Uses various ML frameworks for model support
 Inspired by the need for offline AI capabilities
