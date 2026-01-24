# Offline Intelligence Library

High-performance LLM inference engine with advanced memory management and context orchestration capabilities. Built in Rust for maximum performance across Windows, macOS, and Linux platforms.

## Architecture Overview

This project follows a dual-licensing model:
- **Open Source Core (80%)**: Publicly available under Apache 2.0 license
- **Proprietary Extensions (20%)**: Private plugins for advanced features

### Core Components (Public)
- LLM Integration Engine
- Basic Memory Management
- Configuration System
- Metrics and Telemetry
- API Proxy Layer
- Administration Interface

### Proprietary Extensions (Private/Future)
- Advanced Context Management (`context_engine`)
- Key-Value Cache System (`cache_management`)
- Enhanced Memory Components
- Advanced API Features

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|---------|
| Windows | x86_64 | ✅ Supported |
| macOS | x86_64, ARM64 | ✅ Supported |
| Linux | x86_64, ARM64 | ✅ Supported |

## Language Bindings

The library provides native bindings for multiple languages:

### Native Rust
Direct access to all core functionality:
```rust
use offline_intelligence::{Config, run_server};

let config = Config::from_env()?;
run_server(config).await?;
```

### Python
Install via pip:
```bash
pip install offline-intelligence
```

Usage:
```python
from offline_intelligence import Config, run_server

config = Config.from_env()
run_server(config)
```

### C++
CMake integration:
```cpp
#include <offline_intelligence/offline_intelligence.h>

auto config = offline_intelligence::Config::from_env();
offline_intelligence::run_server(config);
```

### JavaScript/Node.js
NPM package:
```bash
npm install offline-intelligence
```

Usage:
```javascript
const { Config, runServer } = require('offline-intelligence');

const config = Config.fromEnv();
runServer(config);
```

### Java
Maven dependency:
```xml
<dependency>
    <groupId>com.offlineintelligence</groupId>
    <artifactId>offline-intelligence-java</artifactId>
    <version>0.1.0</version>
</dependency>
```

Usage:
```java
import com.offlineintelligence.Config;
import com.offlineintelligence.Server;

Config config = Config.fromEnv();
Server.runServer(config);
```

## Building from Source

### Prerequisites
- Rust 1.70+
- CMake 3.16+ (for C++ bindings)
- Python 3.8+ (for Python bindings)
- Node.js 16+ (for JavaScript bindings)
- Java 11+ (for Java bindings)

### Build Process

#### Windows
```cmd
build.bat
```

#### Linux/macOS
```bash
chmod +x build.sh
./build.sh
```

### Build Output
The build process creates distribution packages in the `dist/` directory:
- `rust/` - Native Rust binaries
- `python/` - Python wheels
- `cpp-lib/` - C++ libraries and headers
- `javascript/` - Node.js packages
- `java/` - Java JAR files

## Configuration

The library uses environment variables for configuration:

```bash
# Core settings
LLAMA_BIN=/path/to/llama-server
MODEL_PATH=/path/to/model.gguf
API_HOST=127.0.0.1
API_PORT=8000

# Resource allocation
THREADS=auto
GPU_LAYERS=auto
CTX_SIZE=auto
BATCH_SIZE=auto
```

## API Endpoints

### Core Endpoints
- `POST /generate/stream` - Stream generation
- `GET /healthz` - Health check
- `GET /readyz` - Readiness check
- `GET /metrics` - Prometheus metrics

### Admin Endpoints
- `GET /admin/status` - System status
- `POST /admin/load` - Load model
- `POST /admin/stop` - Stop backend

### Memory Endpoints
- `GET /memory/stats/{session_id}` - Memory statistics
- `POST /memory/optimize` - Optimize memory
- `POST /memory/cleanup` - Cleanup memory

## Performance Characteristics

- **Low Latency**: Optimized for real-time inference
- **Memory Efficient**: Smart caching and garbage collection
- **Multi-threaded**: Automatic thread pool management
- **GPU Accelerated**: CUDA support for NVIDIA GPUs

## Contributing

We welcome contributions to the open-source core components. Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

- Core library: Apache 2.0 License
- Proprietary extensions: Commercial licensing available

## Support

For support, please open an issue on our GitHub repository or contact our team at support@offlineintelligence.com.

## Roadmap

### Short Term (0.2.0)
- Enhanced documentation
- Additional language bindings
- Performance optimizations

### Medium Term (0.3.0)
- Plugin architecture for proprietary extensions
- Cloud deployment support
- Enhanced monitoring capabilities

### Long Term (1.0.0)
- Full commercial plugin ecosystem
- Enterprise features
- Advanced orchestration capabilities