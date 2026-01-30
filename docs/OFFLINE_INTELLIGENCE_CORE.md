# Offline Intelligence Core Library

The core Rust implementation of the Offline Intelligence Library, providing high-performance offline AI inference capabilities with advanced context management and memory optimization.

## Overview

This crate contains the fundamental implementation of the Offline Intelligence system, including:

- Core inference engine with support for multiple model formats
- Advanced context management and optimization algorithms
- Memory-efficient storage and retrieval systems
- Concurrent processing capabilities
- Built-in caching mechanisms
- Comprehensive API for conversation management

## Features

### Core Capabilities

- **Multi-Model Format Support**: GGUF, ONNX, Safetensors, GGML, CoreML, TensorRT
- **Context Optimization**: Intelligent trimming and compression of conversation history
- **Memory Management**: Efficient storage with automatic cleanup and garbage collection
- **Concurrent Processing**: Thread-safe operations with configurable thread pools
- **Caching System**: Multi-tier caching with customizable eviction policies
- **Search Functionality**: Semantic and keyword-based memory search capabilities

### Architecture Components

#### Context Engine
Manages conversation context optimization through intelligent algorithms that balance relevance preservation with memory efficiency.

#### Memory Database
SQLite-based storage system with optimized schemas for conversation history, embeddings, and metadata.

#### Cache Management
Multi-layer caching system with configurable policies for optimal performance.

#### Model Runtime
Support for multiple inference backends and model formats with unified interface.

#### Worker Threads
Asynchronous processing system for handling computationally intensive operations.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
offline-intelligence = "0.1.2"
```

Or build from source:

```bash
cd crates/offline-intelligence
cargo build --release
```

## Basic Usage

```rust
use offline_intelligence::{MemoryStore, Config, Message};

// Initialize the library
let config = Config::default();
let mut memory_store = MemoryStore::new(config);

// Add messages to conversation
let messages = vec![
    Message::new("user", "Hello, how are you?"),
    Message::new("assistant", "I'm doing well, thank you for asking!"),
];

// Store conversation
memory_store.add_session("session-123", messages).await?;

// Optimize context
let optimized = memory_store.optimize_context(
    "session-123", 
    Some("What did we discuss earlier?".to_string())
).await?;

// Search memory
let results = memory_store.search("hello", 10).await?;
```

## API Reference

### Main Components

#### MemoryStore
Primary interface for conversation management and memory operations.

#### Config
Configuration struct for customizing library behavior.

#### Message
Standard message structure for conversations.

### Key Methods

- `add_session()` - Store new conversation sessions
- `optimize_context()` - Optimize conversation context
- `search()` - Search through memory
- `generate_title()` - Automatically generate conversation titles
- `get_stats()` - Retrieve memory usage statistics

## Configuration Options

The library can be configured through the `Config` struct:

```rust
let config = Config {
    max_context_length: 4096,
    cache_size: 1000,
    cleanup_threshold: 0.8,
    // ... other options
};
```

## Performance Considerations

### Memory Usage
The library is designed to be memory-efficient with configurable limits and automatic cleanup mechanisms.

### Concurrency
Thread-safe design allows for concurrent operations across multiple sessions.

### Caching
Multi-tier caching system reduces repeated computation and improves response times.

## Model Support

### Supported Formats
- GGUF (GPT-Generated Unified Format)
- ONNX (Open Neural Network Exchange)
- Safetensors
- GGML (GGML format)
- CoreML (Apple platforms)
- TensorRT (NVIDIA platforms)

### Model Loading
Models can be loaded from local files or downloaded automatically based on configuration.

## Error Handling

The library uses `anyhow` for comprehensive error handling with detailed error messages and context.

## Testing

Run tests with:

```bash
cargo test
```

## Examples

See the `examples/` directory for complete usage examples including:

- Basic conversation management
- Context optimization scenarios
- Memory search implementations
- Performance benchmarking

## License

Apache License 2.0

## Contributing

See the main repository README for contribution guidelines.