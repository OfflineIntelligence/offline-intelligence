# Offline Intelligence Library

High-performance LLM inference engine with memory management. Cross-platform native library with bindings for Python, Java, C++, and JavaScript.

>>>>>>> 4a97d001945d59e94c4d9baf04651020704640e7
License: https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE
Crates.io: https://crates.io/crates/offline-intelligence
PyPI: https://pypi.org/project/offline-intelligence/
npm: https://www.npmjs.com/package/offline-intelligence
JitPack: https://jitpack.io/#OfflineIntelligence/offline-intelligence
GitHub: https://github.com/OfflineIntelligence/offline-intelligence
License: https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE
=======

>>>>>>> 4a97d001945d59e94c4d9baf04651020704640e7
License: https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE

## Overview

The Offline Intelligence Library delivers enterprise-grade LLM inference capabilities with intelligent memory management across five programming languages. Built with an 80/20 open-source distribution model, 80% of core functionality is freely available under Apache 2.0 license while advanced proprietary extensions are available through commercial licensing.

The library provides optimized performance through hardware-aware resource allocation, supports multiple quantization schemes for different hardware profiles, implements robust security frameworks with compliance alignment, and offers scalable deployment patterns for various production environments. Container orchestration support enables seamless Kubernetes integration, while comprehensive monitoring capabilities ensure production reliability.

## Quick Start

Install the library for your preferred language:

```bash
# Rust (Crates.io)
cargo add offline-intelligence

# Python (PyPI)
pip install offline-intelligence

# JavaScript/Node.js (npm)
npm install offline-intelligence

# Java (JitPack - Maven/Gradle)
# See Java package documentation at JitPack link above

# C++ (Header-only)
# Download from GitHub releases or clone repository
```

Initialize and start the server with default configuration:

```rust
use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = Config::from_env()?;
    
    // Start the LLM inference server
    run_server(config).await?;
    
    Ok(())
}
```

The library supports comprehensive environment-based configuration with automatic hardware detection:

```bash
# Core LLM Settings
export LLAMA_BIN="/path/to/llama-server"        # LLM backend binary
export MODEL_PATH="/path/to/model.gguf"         # GGUF model file path
export LLAMA_HOST="127.0.0.1"                   # Backend host address
export LLAMA_PORT="8081"                        # Backend port

# API Configuration
export API_HOST="0.0.0.0"                       # API server bind address
export API_PORT="8000"                          # API server port
export REQUESTS_PER_SECOND="24"                 # Rate limiting threshold

# Performance Tuning
export CTX_SIZE="8192"                          # Context window size
export BATCH_SIZE="256"                         # Processing batch size
export THREADS="6"                              # CPU thread count
export GPU_LAYERS="20"                          # GPU acceleration layers

# Resource Management
export HEALTH_TIMEOUT_SECONDS="60"              # Health check timeout
export MAX_CONCURRENT_STREAMS="4"               # Concurrent request limit
export PROMETHEUS_PORT="9000"                   # Metrics endpoint port

# Auto-detection mode (set to "auto" for automatic configuration)
export THREADS="auto"
export GPU_LAYERS="auto"
export CTX_SIZE="auto"
export BATCH_SIZE="auto"
```

## Language Bindings

### Rust Implementation
```rust
use offline_intelligence::{Config, run_server, LLMEngine, MemoryDatabase};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize configuration with environment variables
    let config = Config::from_env()?;
    
    // Create LLM engine instance
    let llm_engine = LLMEngine::new(config.clone());
    
    // Initialize the engine asynchronously
    llm_engine.initialize().await?;
    
    // Start the inference server
    run_server(config).await?;
    
    Ok(())
}
```

### Python Integration
```python
from offline_intelligence import Config, run_server
import os

# Configure environment variables
os.environ['LLAMA_BIN'] = '/usr/local/bin/llama-server'
os.environ['MODEL_PATH'] = './models/llama-3.gguf'

# Load configuration from environment
config = Config.from_env()

# Validate configuration
if not config.validate():
    raise ValueError("Invalid configuration")

# Start server with error handling
try:
    success = run_server(config)
    print(f"Server started successfully: {success}")
except Exception as e:
    print(f"Server startup failed: {e}")
```

### Java Usage
```java
import com.offlineintelligence.Config;
import com.offlineintelligence.Server;
import com.offlineintelligence.OfflineIntelligenceException;

public class OfflineIntelligenceApp {
    public static void main(String[] args) {
        try {
            // Load configuration from environment
            Config config = Config.fromEnv();
            
            // Configure server parameters
            config.setApiHost("0.0.0.0");
            config.setApiPort(8080);
            config.setThreads(8);
            
            // Start the server
            boolean success = Server.runServer(config);
            
            if (success) {
                System.out.println("Server started on port: " + config.getApiPort());
                System.out.println("Version: " + Server.version());
            } else {
                System.err.println("Failed to start server");
            }
            
        } catch (OfflineIntelligenceException e) {
            System.err.println("Configuration error: " + e.getMessage());
        } catch (Exception e) {
            System.err.println("Unexpected error: " + e.getMessage());
        }
    }
}
```

### C++ Implementation
```cpp
#include <offline_intelligence/offline_intelligence.hpp>
#include <iostream>
#include <cstdlib>

int main() {
    try {
        // Set environment variables programmatically
        setenv("LLAMA_BIN", "/opt/llama/llama-server", 1);
        setenv("MODEL_PATH", "./models/mistral.gguf", 1);
        
        // Load configuration from environment
        auto config = offline_intelligence::Config::from_env();
        
        // Override specific settings
        config.api_host = "0.0.0.0";
        config.api_port = 9000;
        config.ctx_size = 16384;
        
        // Start server and check result
        bool success = offline_intelligence::Server::run_server(config);
        
        if (success) {
            std::cout << "Server version: " 
                      << offline_intelligence::Server::version() << std::endl;
            std::cout << "Listening on: " << config.api_host 
                      << ":" << config.api_port << std::endl;
        } else {
            std::cerr << "Failed to start server" << std::endl;
            return 1;
        }
        
    } catch (const offline_intelligence::OfflineIntelligenceException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

### JavaScript Integration
```javascript
const { Config, runServer } = require('offline-intelligence');

// Async function for server initialization
async function initializeServer() {
    try {
        // Load configuration from environment
        const config = Config.fromEnv();
        
        // Configure server settings
        config.apiHost = '0.0.0.0';
        config.apiPort = 8080;
        config.modelPath = './models/code-llama.gguf';
        
        // Validate configuration before startup
        if (!config.isValid()) {
            throw new Error('Invalid server configuration');
        }
        
        // Start server with async/await
        const success = await runServer(config);
        
        if (success) {
            console.log(`Server started successfully on ${config.apiHost}:${config.apiPort}`);
            console.log(`Model loaded: ${config.modelPath}`);
        } else {
            console.error('Server failed to start');
        }
        
    } catch (error) {
        console.error('Server initialization error:', error.message);
        process.exit(1);
    }
}

// Handle graceful shutdown
process.on('SIGINT', () => {
    console.log('Shutting down server...');
    process.exit(0);
});

// Start the server
initializeServer();
```

## API Endpoints

The library exposes standard RESTful endpoints for LLM inference:

```bash
# Streaming generation endpoint
POST /generate/stream
Content-Type: application/json

{
    "messages": [
        {"role": "user", "content": "Hello, how are you?"}
    ],
    "session_id": "session_123",
    "temperature": 0.7,
    "max_tokens": 1024
}

# Health check endpoint
GET /healthz

# Metrics endpoint (Prometheus format)
GET /metrics

# Readiness check
GET /readyz
```

## Model Support

The library supports various quantized models in GGUF format:

```python
# Model configuration examples
supported_models = {
    "llama3_8b": {
        "path": "./models/llama3-8b-q4.gguf",
        "context_size": 8192,
        "recommended_vram": "8GB"
    },
    "mistral_7b": {
        "path": "./models/mistral-7b-q4.gguf", 
        "context_size": 32768,
        "recommended_vram": "12GB"
    },
    "codellama_13b": {
        "path": "./models/codellama-13b-q4.gguf",
        "context_size": 16384,
        "recommended_vram": "16GB"
    }
}
```

## Language Bindings Overview

The Offline Intelligence Library provides native bindings for five programming languages:

**Package Links:**
- **Rust**: [Crates.io](https://crates.io/crates/offline-intelligence)
- **Python**: [PyPI](https://pypi.org/project/offline-intelligence/)
- **JavaScript**: [npm](https://www.npmjs.com/package/offline-intelligence)
- **Java**: [JitPack](https://jitpack.io/#OfflineIntelligence/offline-intelligence)
- **C++**: [GitHub](https://github.com/OfflineIntelligence/offline-intelligence)

Each binding is optimized for idiomatic usage within its respective ecosystem. All bindings share the same underlying Rust core and expose consistent APIs while maintaining language-specific conventions.

### Cross-Language Consistency

All language bindings implement the same core functionality:
- Unified configuration management through environment variables
- Consistent API endpoint interfaces
- Shared performance characteristics and resource utilization patterns
- Compatible data structures and serialization formats

### Binding-Specific Optimizations

Each language binding includes optimizations for its target platform:
- **Rust**: Zero-copy data transfer and async/await native integration
- **Python**: PyO3 integration with NumPy array compatibility
- **Java**: JNI optimizations with garbage collector awareness
- **C++**: Header-only distribution with template metaprogramming
- **JavaScript**: Node.js addon with V8 engine integration

Detailed installation and usage instructions for each binding are available in their respective package repositories and documentation.

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

### Official Resources

- **GitHub Repository**: https://github.com/OfflineIntelligence/offline-intelligence
- **Issue Tracker**: https://github.com/OfflineIntelligence/offline-intelligence/issues
- **API Documentation**: Comprehensive reference for all language bindings
- **Release Notes**: Detailed changelogs and migration guides
- **Security Advisories**: CVE notifications and vulnerability disclosures

### Community Support

- **GitHub Discussions**: Official project discussions and announcements
- **Stack Overflow**: Community-maintained Q&A for implementation questions
- **Documentation Portal**: API references and integration guides

### Enterprise Support

- **Commercial Licensing**: Priority support with SLA guarantees
- **Consulting Services**: Architecture reviews and deployment assistance
- **Training Programs**: Custom workshops and certification programs

The library follows standard open-source practices with Apache 2.0 licensing for the core 80% functionality, while advanced enterprise features are available through commercial agreements.




## License

This project is licensed under the Apache 2.0 License. The core 80% of functionality is open source, while proprietary extensions are available through separate commercial licensing agreements.

See the [LICENSE](LICENSE) file for details.
