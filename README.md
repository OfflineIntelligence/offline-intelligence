# Offline Intelligence Library

A high-performance library for offline AI inference with context management, memory optimization, and multi-format model support.

## What This Library Does

Offline Intelligence Library empowers developers to run powerful large language models locally without internet connectivity. Unlike cloud-based AI services, this library brings enterprise-grade AI capabilities directly to your applications, ensuring data privacy, eliminating latency, and reducing operational costs.

### Core Capabilities

**🧠 Intelligent Context Management**
- Automatically optimizes conversation history to maintain relevant context
- Prevents context window overflow while preserving important information
- Dynamically manages memory usage for efficient long-running conversations

**🔍 Hybrid Memory Search**
- Combines semantic understanding with keyword matching
- Searches across conversation histories to find relevant context
- Enables contextual responses based on past interactions

**⚡ Multi-Format Model Support**
- Works with GGUF, GGML, ONNX, TensorRT, and Safetensors formats
- Supports popular models like Llama, Mistral, and custom fine-tuned variants
- Flexible model loading and switching capabilities

**🌐 Cross-Platform Compatibility**
- Native support for Windows, macOS, and Linux
- Language bindings for Python, Java, JavaScript/Node.js, C++, and Rust
- Consistent API across all platforms

## How It Works

The library combines several sophisticated techniques to deliver offline AI capabilities:

1. **Model Orchestration**: Efficiently manages model loading, memory allocation, and inference execution
2. **Context Optimization**: Uses intelligent algorithms to compress and prioritize conversation context
3. **Memory Management**: Implements caching strategies to balance performance with resource usage
4. **Format Abstraction**: Provides unified interface across different model formats and runtimes
5. **Language Integration**: Offers native bindings that feel natural in each programming language

## Quick Start

### Installation

Choose your preferred language:

**Rust (Best Performance)**
```bash
cargo add offline-intelligence
```

**Python (Easy Prototyping)**
```bash
pip install offline-intelligence
```

**JavaScript/Node.js (Web Applications)**
```bash
npm install offline-intelligence-sdk
```

**Java (Enterprise Applications)**
```xml
<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offline-intelligence</artifactId>
    <version>v0.1.2</version>
</dependency>
```

### Minimal Example

**Python:**
```python
from offline_intelligence_py import OfflineIntelligence

# Initialize library
ai = OfflineIntelligence()

# Generate response
response = ai.generate_completion("Explain quantum computing")
print(response)
```

**JavaScript:**
```javascript
const { OfflineIntelligence } = require('offline-intelligence-sdk');

const ai = new OfflineIntelligence();
const response = await ai.generateCompletion("Explain quantum computing");
console.log(response);
```

**Rust:**
```rust
use offline_intelligence::OfflineIntelligence;

let ai = OfflineIntelligence::new()?;
let response = ai.generate_completion("Explain quantum computing", None).await?;
println!("{}", response);
```

## Getting Started Guide

New to the library? Follow our comprehensive guides:

1. **[Universal Setup Guide](docs/GETTING_STARTED.md)** - Complete installation and configuration
2. **[Environment Configuration](docs/ENVIRONMENT_SETUP.md)** - Setting up models and binaries
3. **[Model Installation](docs/MODEL_INSTALLATION.md)** - Downloading and managing AI models

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

### Getting Started (Start Here)
- [Universal Getting Started Guide](docs/GETTING_STARTED.md) - Complete setup for all platforms
- [Environment Setup](docs/ENVIRONMENT_SETUP.md) - Configuration and variables
- [Model Installation](docs/MODEL_INSTALLATION.md) - AI model download and setup

### Platform-Specific Guides
- [Rust Development](docs/RUST_GETTING_STARTED.md) - Best performance, full control
- [Python Development](docs/PYTHON_GETTING_STARTED.md) - Easy to use, great for prototyping
- [JavaScript Development](docs/JAVASCRIPT_GETTING_STARTED.md) - Web and Node.js applications
- [Java Development](docs/JAVA_GETTING_STARTED.md) - Enterprise applications
- [C++ Development](docs/CPP_GETTING_STARTED.md) - System-level integration

### Additional Resources
- [Core Library Documentation](docs/OFFLINE_INTELLIGENCE.md)
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
