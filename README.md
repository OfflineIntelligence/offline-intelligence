# Offline Intelligence Library

High-performance LLM inference engine with memory management. Cross-platform native library with bindings for Python, Java, C++, and JavaScript.

Crates.io: https://crates.io/crates/offline-intelligence
PyPI: https://pypi.org/project/offline-intelligence/
npm: https://www.npmjs.com/package/offline-intelligence
License: https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE

## Overview

The Offline Intelligence Library provides enterprise-grade LLM inference capabilities with intelligent memory management across 5 programming languages. This comprehensive guide covers installation, configuration, usage, and API reference for all supported platforms.

Built with an 80/20 open-source model - 80% of core functionality is freely available, with advanced proprietary extensions available separately for enhanced performance and features.

## Quick Start

### Installation

Choose your preferred language binding:

```bash
# Rust (Crates.io)
cargo add offline-intelligence

# Python (PyPI)
pip install offline-intelligence

# JavaScript/Node.js (npm)
npm install offline-intelligence

# Java (JitPack - Maven/Gradle)
# See Java section below

# C++ (Header-only)
# Download from GitHub releases
```

### Basic Usage

```rust
use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    run_server(config).await?;
    Ok(())
}
```

## Language Bindings

### Rust Usage

**Installation**
```toml
[dependencies]
offline-intelligence = "0.1.1"
```

**Location**: crates.io/crates/offline-intelligence

**Basic Usage**
```rust
use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    run_server(config).await?;
    Ok(())
}
```

**Available Imports**
```rust
use offline_intelligence::{
    Config,
    run_server,
    LLMEngine,
    MemoryDatabase,
};
```

### Python Usage

**Installation**
```bash
pip install offline-intelligence
```

**Location**: pypi.org/project/offline-intelligence

**Basic Usage**
```python
from offline_intelligence import Config, run_server

config = Config.from_env()
success = run_server(config)
print(f"Server started: {success}")
```

**Available Imports**
```python
from offline_intelligence import (
    Config,
    run_server,
)
```

### C++ Usage

**Installation (Header-Only)**
```bash
git clone https://github.com/OfflineIntelligence/offline-intelligence.git
cp offline-intelligence/bindings/cpp/include/offline_intelligence/offline_intelligence.hpp /path/to/your/include/
```

**Location**: GitHub - bindings/cpp/include/offline_intelligence/offline_intelligence.hpp

**Basic Usage**
```cpp
#include <offline_intelligence/offline_intelligence.hpp>
#include <iostream>

int main() {
    try {
        auto config = offline_intelligence::Config::from_env();
        bool success = offline_intelligence::Server::run_server(config);
        
        if (success) {
            std::cout << "Server version: " 
                      << offline_intelligence::Server::version() << std::endl;
        }
    } catch (const offline_intelligence::OfflineIntelligenceException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
    }
    
    return 0;
}
```

**Available Classes/Functions**
```cpp
namespace offline_intelligence {
    struct Config {
        static Config from_env();
    };
    
    class Server {
    public:
        static bool run_server(const Config& config);
        static std::string version();
    };
    
    class OfflineIntelligenceException;
}
```

### Java Usage

**Installation (Maven)**
```xml
<repositories>
    <repository>
        <id>jitpack.io</id>
        <url>https://jitpack.io</url>
    </repository>
</repositories>

<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offline-intelligence</artifactId>
    <version>v0.1.1</version>
</dependency>
```

**Installation (Gradle)**
```gradle
repositories {
    maven { url 'https://jitpack.io' }
}

dependencies {
    implementation 'com.github.OfflineIntelligence:offline-intelligence:v0.1.1'
}
```

**Location**: Service: JitPack - OfflineIntelligence/offline-intelligence

**Basic Usage**
```java
import com.offlineintelligence.Config;
import com.offlineintelligence.Server;
import com.offlineintelligence.OfflineIntelligenceException;

public class Example {
    public static void main(String[] args) {
        try {
            Config config = Config.fromEnv();
            boolean success = Server.runServer(config);
            
            if (success) {
                System.out.println("Server version: " + Server.version());
            }
        } catch (OfflineIntelligenceException e) {
            System.err.println("Error: " + e.getMessage());
        }
    }
}
```

**Available Classes**
```java
package com.offlineintelligence;

public class Config {
    public static Config fromEnv();
}

public class Server {
    public static boolean runServer(Config config);
    public static String version();
}

public class OfflineIntelligenceException extends Exception {
}
```

### JavaScript/Node.js Usage

**Installation**
```bash
npm install offline-intelligence
```

**Location**: npmjs.com/package/offline-intelligence

**Basic Usage**
```javascript
const { Config, runServer } = require('offline-intelligence');

const config = Config.fromEnv();
const success = runServer(config);
console.log(`Server started: ${success}`);
```

**Available Exports**
```javascript
const {
    Config,
    runServer,
} = require('offline-intelligence');
```

## Configuration Reference

All language bindings use the same underlying configuration structure.

### Core Configuration Fields

```yaml
# LLM Settings
model_path: "default.gguf"
llama_bin: "llama-server"
llama_host: "127.0.0.1"
llama_port: 8081

# API Settings
api_host: "127.0.0.1"
api_port: 8000
requests_per_second: 24

# Performance Settings
ctx_size: 8192
batch_size: 256
threads: 6
gpu_layers: 20

# Resource Management
health_timeout_seconds: 60
hot_swap_grace_seconds: 25
max_concurrent_streams: 4

# Monitoring
prometheus_port: 9000

# Queue Settings
queue_size: 100
queue_timeout_seconds: 30
```

### Environment Variables

```bash
# Core settings
export LLAMA_BIN="/path/to/llama-server"
export MODEL_PATH="/path/to/model.gguf"
export API_HOST="0.0.0.0"
export API_PORT="8000"

# Performance tuning
export THREADS="8"
export GPU_LAYERS="30"
export CTX_SIZE="16384"
export BATCH_SIZE="512"

# Auto-detection
export THREADS="auto"
export GPU_LAYERS="auto"
export CTX_SIZE="auto"
export BATCH_SIZE="auto"
```

### Auto-Detection Features

The library automatically detects optimal settings based on your hardware:

- CPU Threads: Automatically calculated based on core count
- GPU Layers: Detected from available VRAM (requires NVIDIA GPU)
- Context Size: Inferred from model filename and adjusted for available RAM
- Batch Size: Calculated based on context size and available memory

## API Endpoints

### Core Endpoints (Available in all bindings)

```
POST /generate/stream     # Stream generation
GET  /healthz            # Health check
GET  /readyz             # Readiness check
GET  /metrics            # Prometheus metrics
```

### Admin Endpoints

```
GET  /admin/status       # System status
POST /admin/load         # Load model
POST /admin/stop         # Stop backend
```

### Memory Endpoints

```
GET  /memory/stats/{session_id}   # Memory statistics
POST /memory/optimize             # Optimize memory (requires proprietary extension)
POST /memory/cleanup              # Cleanup memory (requires proprietary extension)
```

Note: Memory optimization and cleanup endpoints require the proprietary context engine extension for full functionality.

## Technical Architecture

### Core Components (80% Open Source)

1. LLM Integration Layer
   - Direct integration with llama.cpp backend
   - Streaming response handling
   - Automatic model loading and management
   - Health monitoring and recovery

2. Memory Management System
   - SQLite-based conversation storage
   - Message indexing and retrieval
   - Session management
   - Data persistence and migration

3. API Gateway
   - RESTful HTTP interface
   - Rate limiting and concurrency control
   - CORS support
   - Request queuing and timeout handling

4. Monitoring & Telemetry
   - Prometheus metrics collection
   - Structured logging with tracing
   - Performance observability
   - Resource utilization tracking

### Proprietary Extensions (20% - Available Separately)

Advanced features available through separate licensing:
- Context-aware conversation optimization
- KV cache management system
- Advanced memory compression algorithms
- Enterprise security features
- Priority support and consulting

## Performance Characteristics

### Resource Requirements

Minimum System Requirements:
- RAM: 8GB (16GB recommended)
- CPU: 4 cores (8+ cores recommended)
- Storage: 10GB free space for models
- GPU: Optional (CUDA-compatible NVIDIA GPU recommended)

Recommended for Production:
- RAM: 32GB+
- CPU: 16+ cores
- GPU: RTX 3080+ or equivalent (8GB+ VRAM)
- Storage: NVMe SSD

### Performance Tuning

```bash
# High-throughput configuration
export THREADS="16"
export GPU_LAYERS="40"
export CTX_SIZE="8192"
export BATCH_SIZE="512"
export MAX_CONCURRENT_STREAMS="8"

# Low-resource configuration
export THREADS="4"
export GPU_LAYERS="12"
export CTX_SIZE="2048"
export BATCH_SIZE="64"
export MAX_CONCURRENT_STREAMS="2"
```

## Model Selection Guide

### Recommended Models

For General Purpose Use:
- Llama 3.1 8B (4-bit quantized)
- Mistral 7B (4-bit quantized)
- Phi-3 Mini (4-bit quantized)

For Code Generation:
- CodeLlama 7B/13B
- DeepSeek-Coder 6.7B
- StarCoder2 3B/7B/15B

For Reasoning Tasks:
- Llama 3.1 70B (4-bit quantized)
- Mixtral 8x7B (4-bit quantized)
- Gemma 2 9B/27B

### Model Placement

Place your GGUF model files in one of these locations:
```
./resources/models/model.gguf
./models/model.gguf
./model.gguf
```

Or specify the path using the MODEL_PATH environment variable.

## Quick Start Examples

### Rust Example

```rust
use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        api_host: "0.0.0.0".to_string(),
        api_port: 8080,
        model_path: "./models/llama.gguf".to_string(),
        ..Config::from_env()?
    };
    
    run_server(config).await?;
    Ok(())
}
```

### Python Example

```python
from offline_intelligence import Config, run_server

config = Config()
config.api_host = "0.0.0.0"
config.api_port = 8080
config.model_path = "./models/llama.gguf"

success = run_server(config)
```

### C++ Example

```cpp
#include <offline_intelligence/offline_intelligence.hpp>

int main() {
    auto config = offline_intelligence::Config::from_env();
    config.api_host = "0.0.0.0";
    config.api_port = 8080;
    
    bool success = offline_intelligence::Server::run_server(config);
    return success ? 0 : 1;
}
```

### Java Example

```java
import com.offlineintelligence.*;

public class Main {
    public static void main(String[] args) {
        try {
            Config config = Config.fromEnv();
            config.setApiHost("0.0.0.0");
            config.setApiPort(8080);
            
            boolean success = Server.runServer(config);
            System.out.println("Started: " + success);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```

### JavaScript Example

```javascript
const { Config, runServer } = require('offline-intelligence');

const config = Config.fromEnv();
config.apiHost = "0.0.0.0";
config.apiPort = 8080;

const success = runServer(config);
console.log(`Started: ${success}`);
```

## Support and Documentation

- GitHub Repository: https://github.com/OfflineIntelligence/offline-intelligence
- Issue Tracker: https://github.com/OfflineIntelligence/offline-intelligence/issues
- Documentation: Each language binding includes README files with detailed API references
- Community: Join our Discord server for community support
- Enterprise Support: Contact sales@offlineintelligence.com for commercial licensing

## License

This project is licensed under the Apache 2.0 License - see the LICENSE file for details.

The core 80% of functionality is open source. Proprietary extensions are available through separate commercial licensing agreements.
