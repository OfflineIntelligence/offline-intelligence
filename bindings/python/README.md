# Offline Intelligence Python Bindings

Python bindings for the Offline Intelligence Library - High-performance LLM inference engine with memory management capabilities.

## Installation

```bash
pip install offline-intelligence
```

## Quick Start

```python
from offline_intelligence import Config, run_server

# Configure the engine
config = Config.from_env()

# Start the server
run_server(config)
```

## Features

- **Core LLM Integration**: Direct access to LLM engine functionality
- **Memory Management**: Base memory operations and database access
- **Configuration**: Flexible configuration system
- **Metrics**: Performance monitoring and telemetry
- **Proxy Interface**: Stream generation and API proxy functionality

## Architecture

This package provides bindings to the core open-source components (80%) of the Offline Intelligence system. Proprietary extensions are available separately.

## Platform Support

- Windows (x64)
- macOS (Intel/Apple Silicon)
- Linux (x64, ARM64)

## License

Apache 2.0