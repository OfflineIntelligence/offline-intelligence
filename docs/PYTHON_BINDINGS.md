# Python Bindings for Offline Intelligence

Python bindings for the Offline Intelligence Library, enabling seamless integration of offline AI capabilities into Python applications.

## Overview

This package provides Python developers with access to the full power of the Offline Intelligence Library through native Python extensions. The bindings maintain the same high-performance characteristics as the core library while providing a familiar Pythonic interface.

## Features

### Core Functionality
- Native Python integration with zero-copy data transfer
- Asynchronous operations using asyncio
- Automatic memory management
- Thread-safe operations
- Comprehensive error handling with Python exceptions

### Python-Specific Enhancements
- Type hints for better IDE support
- Context manager support for resource management
- Integration with Python logging system
- Support for Python data structures (lists, dicts, etc.)

## Installation

### From PyPI (Recommended)
```bash
pip install offline-intelligence-py
```

### From Source
```bash
# Clone the repository
git clone https://github.com/offline-intelligence/library.git
cd library

# Build and install Python bindings
cd crates/python-bindings
pip install .
```

### Development Installation
```bash
# For development with editable installation
pip install -e .
```

## System Requirements

- Python 3.8 or higher
- pip 20.0 or higher
- Rust toolchain (for building from source)
- Compatible C++ compiler (for binary extensions)

## Basic Usage

```python
from offline_intelligence import MemoryStore, Config, Message

# Initialize with default configuration
config = Config()
memory_store = MemoryStore(config)

# Create conversation messages
messages = [
    Message(role="user", content="Hello, how are you?"),
    Message(role="assistant", content="I'm doing well, thank you for asking!")
]

# Store conversation session
await memory_store.add_session("session-123", messages)

# Optimize context for follow-up questions
optimized = await memory_store.optimize_context(
    "session-123", 
    user_query="What did we discuss earlier?"
)

# Search through memory
results = await memory_store.search("hello", limit=10)
```

## Advanced Usage

### Async/Await Pattern
```python
import asyncio
from offline_intelligence import MemoryStore

async def conversation_example():
    memory_store = MemoryStore()
    
    # Stream processing example
    async for chunk in memory_store.generate_stream(messages):
        print(chunk, end="", flush=True)

# Run the async function
asyncio.run(conversation_example())
```

### Context Manager Usage
```python
from offline_intelligence import MemoryStore

# Automatic resource cleanup
with MemoryStore() as memory_store:
    # Perform operations
    result = await memory_store.process_conversation(messages)
    # Resources automatically cleaned up
```

### Configuration Customization
```python
from offline_intelligence import Config

# Custom configuration
config = Config(
    max_context_length=8192,
    cache_size=2000,
    cleanup_threshold=0.75,
    model_path="/path/to/custom/model"
)

memory_store = MemoryStore(config)
```

## API Reference

### Main Classes

#### MemoryStore
Primary interface for all memory operations.

##### Methods:
- `add_session(session_id: str, messages: List[Message]) -> None`
- `optimize_context(session_id: str, user_query: Optional[str] = None) -> List[Message]`
- `search(query: str, limit: int = 10) -> List[SearchResult]`
- `generate_title(messages: List[Message]) -> str`
- `get_stats() -> dict`

#### Config
Configuration class for customizing behavior.

##### Properties:
- `max_context_length`: Maximum tokens in context (default: 4096)
- `cache_size`: Number of items to cache (default: 1000)
- `cleanup_threshold`: Memory cleanup threshold (default: 0.8)
- `model_path`: Path to AI model files

#### Message
Standard message structure.

##### Properties:
- `role`: Message sender role ("user", "assistant", "system")
- `content`: Message content text

### Exception Handling
```python
from offline_intelligence import OfflineIntelligenceError

try:
    result = await memory_store.some_operation()
except OfflineIntelligenceError as e:
    print(f"Operation failed: {e}")
```

## Performance Optimization

### Memory Management
```python
# Configure memory limits
config = Config(
    max_memory_mb=1024,  # Limit memory usage to 1GB
    cleanup_interval=300  # Cleanup every 5 minutes
)
```

### Batch Operations
```python
# Process multiple sessions efficiently
sessions = {
    "session-1": messages1,
    "session-2": messages2,
    "session-3": messages3
}

# Batch processing for better performance
for session_id, messages in sessions.items():
    await memory_store.add_session(session_id, messages)
```

## Integration Examples

### Web Framework Integration
```python
from flask import Flask, request, jsonify
from offline_intelligence import MemoryStore

app = Flask(__name__)
memory_store = MemoryStore()

@app.route('/chat', methods=['POST'])
async def chat():
    data = request.json
    messages = data.get('messages', [])
    
    response = await memory_store.process_conversation(messages)
    return jsonify({'response': response})
```

### Jupyter Notebook Usage
```python
# In Jupyter notebooks
import asyncio
from offline_intelligence import MemoryStore

# Create event loop for notebook environment
memory_store = MemoryStore()

# Interactive conversation
messages = []
while True:
    user_input = input("You: ")
    if user_input.lower() == 'quit':
        break
        
    messages.append({"role": "user", "content": user_input})
    response = await memory_store.generate_response(messages)
    print(f"AI: {response}")
    messages.append({"role": "assistant", "content": response})
```

## Troubleshooting

### Common Issues

#### Import Errors
```bash
# Ensure proper installation
pip install --force-reinstall offline-intelligence-py
```

#### Performance Issues
```python
# Monitor memory usage
stats = await memory_store.get_stats()
print(f"Memory usage: {stats['memory_usage_mb']} MB")
```

#### Model Loading Problems
```python
# Verify model path
config = Config(model_path="/correct/path/to/model")
```

## Testing

Run Python-specific tests:
```bash
cd crates/python-bindings
python -m pytest tests/
```

## License

Apache License 2.0

## Contributing

Contributions are welcome. Please see the main repository guidelines for more information.

For issues specific to Python bindings, please tag them appropriately in the issue tracker.