# Offline Intelligence Python Bindings

Python bindings for the Offline Intelligence Library, providing offline AI inference capabilities with context management and memory optimization.

## Installation

```bash
pip install offline-intelligence
```

Or build from source:

```bash
pip install .
```

## Quick Start

```python
from offline_intelligence_py import OfflineIntelligence, Message, Config

# Initialize the library
oi = OfflineIntelligence()

# Load configuration
config = Config.from_env()

# Create messages
messages = [
    Message("user", "Hello, how are you?"),
    Message("assistant", "I'm doing well, thank you for asking!"),
    Message("user", "What can you help me with?")
]

# Optimize context
result = oi.optimize_context("session123", messages, "What can you help me with?")
print(f"Optimized from {result['original_count']} to {result['optimized_count']} messages")

# Search memory
search_result = oi.search("help me", session_id="session123", limit=10)
print(f"Found {search_result['total']} results")

# Generate title
title = oi.generate_title(messages)
print(f"Generated title: {title}")
```

## Features

- **Offline AI Inference**: Run LLMs locally without internet connection
- **Context Management**: Intelligent conversation context optimization
- **Memory Search**: Hybrid semantic and keyword search across conversations
- **Multi-format Support**: Support for GGUF, GGML, ONNX, TensorRT, and Safetensors models
- **Cross-platform**: Works on Windows, macOS, and Linux

## Requirements

- Python 3.8 or higher
- Rust toolchain for building from source
- Compatible LLM model file

## Configuration

The library reads configuration from environment variables:

```bash
export LLAMA_BIN="/path/to/llama-binary"
export MODEL_PATH="/path/to/model.gguf"
export CTX_SIZE="8192"
export BATCH_SIZE="256"
export THREADS="6"
export GPU_LAYERS="20"
```

## License

Apache 2.0