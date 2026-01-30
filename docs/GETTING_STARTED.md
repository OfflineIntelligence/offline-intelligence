# Getting Started with Offline Intelligence Library

## 🚀 Quick Start Overview

Welcome to the Offline Intelligence Library! This guide will help you set up and start using the library for offline AI inference across different programming languages.

## 📋 Prerequisites

Before you begin, ensure you have:

### Essential Components
 **AI Model File**: Download a compatible model (GGUF, GGML, ONNX, etc.)
 **LLaMA Binary**: The inference engine binary for your platform
 **Rust Toolchain**: For building from source (if needed)

### System Requirements
 **Operating System**: Windows 10+, macOS 10.15+, or Linux
 **RAM**: Minimum 8GB (16GB+ recommended for larger models)
 **Storage**: 520GB free space (depending on model size)
 **CPU**: Modern processor with AVX2 support (recommended)

## 📥 Installation by Platform

### Rust (Recommended for best performance)
```bash
cargo add offlineintelligence
```

### Python
```bash
pip install offlineintelligence
```

### JavaScript/Node.js
```bash
npm install offlineintelligencesdk
```

### Java
Add to your `pom.xml`:
```xml
<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offlineintelligence</artifactId>
    <version>v0.1.2</version>
</dependency>
```

### C++
Download from [GitHub Releases](https://github.com/OfflineIntelligence/offlineintelligence/releases)

## ⚙️ Environment Setup

### 1. Download AI Models

**Recommended Models:**
 **Llama 2 7B** (small, fast)  ~4GB
 **Llama 2 13B** (balanced)  ~8GB  
 **Llama 2 70B** (powerful)  ~40GB

**Download Sources:**
```bash
# Using huggingfacecli (recommended)
huggingfacecli download TheBloke/Llama27BGGUF llama27b.Q4_K_M.gguf

# Or download manually from:
# https://huggingface.co/TheBloke/Llama27BGGUF
```

### 2. Set Environment Variables

Create a `.env` file or set system environment variables:

```bash
# Required
export MODEL_PATH="/path/to/your/model.gguf"
export LLAMA_BIN="/path/to/llamabinary"

# Optional (performance tuning)
export CTX_SIZE="8192"
export BATCH_SIZE="256" 
export THREADS="6"
export GPU_LAYERS="20"
```

### 3. Verify Installation

Test that everything works:

```bash
# Rust
cargo run example basic_usage

# Python
python c "from offline_intelligence_py import OfflineIntelligence; print('Success!')"

# JavaScript
node e "const { OfflineIntelligence } = require('offlineintelligencesdk'); console.log('Success!')"
```

## 🛠 Configuration Guide

### Basic Configuration

```python
# Python example
from offline_intelligence_py import OfflineIntelligence, Config

config = Config(
    model_path="/path/to/model.gguf",
    llama_bin="/path/to/llamabinary",
    ctx_size=8192,
    batch_size=256,
    threads=6
)

oi = OfflineIntelligence(config)
```

### Advanced Configuration

```bash
# For GPU acceleration (if available)
export GPU_LAYERS="40"  # Number of layers to offload to GPU
export MAIN_GPU="0"     # Which GPU to use (0indexed)

# For memory optimization
export CTX_SIZE="4096"  # Reduce context size to save memory
export BATCH_SIZE="128" # Smaller batches for lower RAM usage
```

## 📖 Your Next Steps

1. **Choose your platform**: Follow the specific guide for your language
2. **Download models**: Get the AI models you want to use
3. **Configure environment**: Set up the required variables
4. **Run examples**: Test with the provided sample code
5. **Start building**: Integrate into your own projects

## 📚 PlatformSpecific Guides

 **[Rust Guide](RUST_GETTING_STARTED.md)**  Best performance, full control
 **[Python Guide](PYTHON_GETTING_STARTED.md)**  Easy to use, great for prototyping  
 **[JavaScript Guide](JAVASCRIPT_GETTING_STARTED.md)**  Web and Node.js applications
 **[Java Guide](JAVA_GETTING_STARTED.md)**  Enterprise applications
 **[C++ Guide](CPP_GETTING_STARTED.md)**  Systemlevel integration

## ❓ Need Help?

 **Documentation**: Check the [complete documentation](../COMPLETE_DOCUMENTATION.md)
 **Issues**: Report problems on [GitHub Issues](https://github.com/OfflineIntelligence/offlineintelligence/issues)
 **Examples**: Browse the [examples directory](../examples/) for sample code



**Next**: Proceed to your platformspecific guide for detailed instructions!
