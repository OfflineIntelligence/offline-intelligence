<div align="center">

<h1>Offline Intelligence Library</h1>

High-performance LLM inference engine with memory management. Cross-platform native library with bindings for Python, JavaScript, Rust, C++, and Java.

<br>

[![Crates.io](https://img.shields.io/crates/v/offline-intelligence.svg)](https://crates.io/crates/offline-intelligence)
[![PyPI](https://img.shields.io/pypi/v/offline-intelligence.svg)](https://pypi.org/project/offline-intelligence/)
[![npm](https://img.shields.io/npm/v/offline-intelligence.svg)](https://www.npmjs.com/package/offline-intelligence)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE)

<br>

**[Documentation]** | **[Installation]** | **[Tutorials]** | **[Resources]**

<br>

**Current Version:** v0.1.2 (February 7, 2026) |
**License:** Apache 2.0

</div>

## Table of Contents

- [Features](#features)
- [Supported Platforms](#supported-platforms)
- [Multi-Language Usage Guide](#multi-language-usage-guide)
- [Installation](#installation)
- [Model Download & Local Usage](#model-download--local-usage)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [Performance](#performance)
- [Security](#security)
- [System Design & Architecture](#system-design--architecture)
- [Technical Specifications](#technical-specifications)
- [API Documentation](#api-documentation)
- [Developer Guide](#developer-guide)
- [Use Cases and Applications](#use-cases-and-applications)
- [Contributing](#contributing)
- [License](#license)
- [Support](#support)
- [Changelog](#changelog)
- [Citation](#citation)

## Features

The Offline Intelligence Library is a high-performance, cross-platform LLM inference engine designed for enterprise-grade deployments. Built with a modular architecture, it provides optimized performance through hardware-aware resource allocation and supports multiple quantization schemes for different hardware profiles.

Key Features:
- Multi-language Support: Native bindings for Rust, Python, Java, C++, and JavaScript
- Hardware Optimization: Automatic resource detection and allocation
- Memory Management: Persistent conversation storage with SQLite backend
- Scalable Architecture: Concurrent request handling with rate limiting
- Monitoring Ready: Prometheus metrics and structured logging
- Production Ready: Kubernetes-friendly with health checks and readiness probes

Core Principles:
- Offline-First: Designed to operate without external dependencies
- Privacy-First: All data processing occurs locally
- Open Source: 80% of functionality available under Apache 2.0 license
- Enterprise Ready: Commercial extensions available for advanced features

## Supported Platforms

### Operating Systems
- Windows: x86_64, ARM64 (Windows 10+)
- Linux: x86_64, ARM64 (Ubuntu 20.04+, CentOS 8+)
- macOS: x86_64, Apple Silicon (macOS 11.0+)

### Hardware Architectures
- x86_64: Intel and AMD 64-bit processors
- ARM64: Apple Silicon, Raspberry Pi 4, and other ARM 64-bit processors

Project Links:
- Crates.io: https://crates.io/crates/offline-intelligence
- PyPI: https://pypi.org/project/offline-intelligence/
- npm: https://www.npmjs.com/package/offline-intelligence
- JitPack: https://jitpack.io/#OfflineIntelligence/offline-intelligence
- GitHub: https://github.com/OfflineIntelligence/offline-intelligence
- License: https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE

## Release Versions

Current Version: **v0.1.2** (Released February 7, 2026)

Version History:
- v0.1.2 (2026-02-07): Added automatic hardware detection, improved memory management, enhanced error handling, fixed critical security vulnerabilities
- v0.1.1 (2025-12-15): Initial public release with multi-language bindings, core LLM integration, and memory management system

## Library Details

The Offline Intelligence Library is a high-performance, cross-platform LLM inference engine designed for enterprise-grade deployments. Built with a modular architecture, it provides optimized performance through hardware-aware resource allocation and supports multiple quantization schemes for different hardware profiles.

Key Features:
- Multi-language Support: Native bindings for Rust, Python, Java, C++, and JavaScript
- Hardware Optimization: Automatic resource detection and allocation
- Memory Management: Persistent conversation storage with SQLite backend
- Scalable Architecture: Concurrent request handling with rate limiting
- Monitoring Ready: Prometheus metrics and structured logging
- Production Ready: Kubernetes-friendly with health checks and readiness probes

Core Principles:
- Offline-First: Designed to operate without external dependencies
- Privacy-First: All data processing occurs locally
- Open Source: 80% of functionality available under Apache 2.0 license
- Enterprise Ready: Commercial extensions available for advanced features

## System Design & Architecture

The Offline Intelligence Library implements a comprehensive system design focused on performance, privacy, and scalability.

Architecture Principles:
- Modularity: Clear separation of concerns with well-defined interfaces
- Performance: Hardware-aware optimization with efficient resource utilization
- Reliability: Robust error handling and recovery mechanisms
- Scalability: Support for varying load patterns and deployment scenarios
- Security: Privacy-first design with secure-by-default configurations

System Overview:
The Offline Intelligence Library consists of interconnected components that work together to provide efficient LLM inference with memory management capabilities.

Core Components:

1. LLM Integration Layer
- Backend: Direct integration with llama.cpp for optimal performance
- Streaming: Real-time response streaming with backpressure handling
- Model Management: Dynamic model loading and hot-swapping capabilities
- Health Monitoring: Continuous backend health checks with auto-recovery

2. Memory Management System
- Storage: SQLite-based persistent conversation storage
- Indexing: Fast message retrieval with session-based organization
- Migration: Automated schema evolution with backward compatibility
- Compression: Optional data compression for large conversation histories

3. API Gateway
- Endpoints: RESTful HTTP interface with standardized responses
- Rate Limiting: Configurable request throttling with burst handling
- CORS: Flexible cross-origin resource sharing policies
- Queuing: Request queue management with timeout controls

4. Resource Management
- Auto-detection: Hardware-aware configuration with optimal defaults
- Memory Management: Efficient memory allocation and garbage collection
- Concurrency Control: Thread-safe request handling with configurable limits
- GPU Acceleration: Automatic GPU layer assignment based on available VRAM

5. Monitoring & Telemetry
- Metrics: Prometheus-compatible performance metrics
- Logging: Structured logging with configurable verbosity
- Tracing: Distributed request tracing for performance analysis
- Alerting: Configurable alert thresholds for operational metrics

Deployment Architecture:
The system supports multiple deployment patterns:
- Single Process: All components run in a single application process
- Container: Deployed as a Docker container with embedded components
- Microservices: Distributed deployment with separate services for each component

Data Flow Architecture:
Requests follow a defined processing flow:
1. Client Request enters the system
2. API Gateway handles validation and routing
3. Rate Limiting and Authentication are applied
4. Request Queue manages the request if needed
5. LLM Integration Layer processes the request
6. Memory Management retrieves context
7. Backend Processing executes with llama.cpp
8. Response Streaming delivers results
9. Memory Management stores updated context
10. Response is delivered to the client

Error Handling & Recovery:
The system implements comprehensive error handling:
- System Errors: Resource exhaustion, hardware failures
- Application Errors: Invalid inputs, configuration issues
- Backend Errors: LLM service unavailability, model issues
- Network Errors: Connectivity problems, timeouts

Recovery Strategies include:
- Automatic Retry: Exponential backoff for transient failures
- Fallback Mechanisms: Graceful degradation for partial failures
- Circuit Breakers: Prevent cascading failures
- Health Monitoring: Continuous health checks with alerts

Modular Architecture:
The library follows a modular design with clear separation of concerns:
- api/: HTTP endpoints and route definitions
  - admin_api.rs: Administrative functions
  - memory_api.rs: Memory management endpoints
  - search_api.rs: Search and retrieval functions
- cache_management/: Caching layer (commercial feature)
  - cache_bridge.rs: Cache-to-database bridge
  - cache_manager.rs: Cache lifecycle management
- context_engine/: Context processing (commercial feature)
  - context_builder.rs: Context construction algorithms
  - orchestrator.rs: Context management orchestrator
- memory_db/: Database layer
  - conversation_store.rs: Conversation storage
  - embedding_store.rs: Embedding vector storage
  - schema.rs: Database schema definitions
- utils/: Utility functions
  - text_utils.rs: Text processing utilities
  - topic_extractor.rs: Topic extraction algorithms
- config.rs: Configuration management
- llm_integration.rs: LLM backend integration
- proxy.rs: API proxy and request handling
- lib.rs: Public API exports and main logic

## Cross-Platform Support

### Operating Systems
- Windows: x86_64, ARM64 (Windows 10+)
- Linux: x86_64, ARM64 (Ubuntu 20.04+, CentOS 8+)
- macOS: x86_64, Apple Silicon (macOS 11.0+)

### Hardware Architectures
- x86_64: Intel and AMD 64-bit processors
- ARM64: Apple Silicon, Raspberry Pi 4, and other ARM 64-bit processors

## Multi-Language Usage Guide

### Rust Usage

Installation:
Add offline-intelligence = "0.1.2" to your Cargo.toml dependencies

Basic Usage:
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

Custom Configuration:
```rust
use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::from_env()?;
    config.api_host = "0.0.0.0".to_string();
    config.api_port = 8080;
    config.model_path = "/path/to/model.gguf".to_string();
    
    run_server(config).await?;
    
    Ok(())
}
```

### Python Usage

Installation:
Run pip install offline-intelligence

Basic Usage:
```python
from offline_intelligence import Config, run_server

# Load configuration from environment variables
config = Config.from_env()

# Start the server with error handling
try:
    success = run_server(config)
    print(f"Server started: {success}")
except Exception as e:
    print(f"Failed to start server: {e}")
```

Custom Configuration:
```python
from offline_intelligence import Config

# Create custom configuration
config = Config()
config.api_host = "0.0.0.0"
config.api_port = 8080
config.model_path = "/path/to/model.gguf"

# Start the server
success = run_server(config)
```

### JavaScript/Node.js Usage

Installation:
Run npm install offline-intelligence

Basic Usage:
```javascript
const { Config, OfflineIntelligence } = require('offline-intelligence');

async function main() {
    try {
        // Load configuration from environment
        const config = Config.fromEnv();
        
        // Start the server with promise handling
        const ai = new OfflineIntelligence(config);
        
        // Check server health
        const health = await ai.healthCheck();
        console.log('Server health:', health);
        
        // Generate text
        const stream = await ai.generateStream('Hello, world!');
        stream.on('data', (chunk) => {
            process.stdout.write(chunk.toString());
        });
        
    } catch (error) {
        console.error('AI operation failed:', error.message);
    }
}

main();
```

Custom Configuration:
```javascript
const { Config } = require('offline-intelligence');

// Create custom configuration
const customConfig = new Config();
customConfig.apiHost = '0.0.0.0';
customConfig.apiPort = 8080;

// Use with OfflineIntelligence instance
const ai = new OfflineIntelligence(customConfig);
```

### Java Usage

Installation:
Add the JitPack repository and dependency to your pom.xml or build.gradle:
- Repository: https://jitpack.io
- GroupId: com.github.OfflineIntelligence
- ArtifactId: offline-intelligence
- Version: 0.1.1

Maven:
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
    <version>0.1.2</version>
</dependency>
```

Gradle:
```gradle
repositories {
    maven { url 'https://jitpack.io' }
}

dependencies {
    implementation 'com.github.OfflineIntelligence:offline-intelligence:0.1.2'
}
```

Basic Usage:
```java
import com.offlineintelligence.Config;
import com.offlineintelligence.Server;
import com.offlineintelligence.OfflineIntelligenceException;

public class AIServer {
    public static void main(String[] args) {
        try {
            // Load configuration from environment
            Config config = Config.fromEnv();
            
            // Start the server with exception handling
            boolean success = Server.runServer(config);
            System.out.println("Server started: " + success);
            
        } catch (OfflineIntelligenceException e) {
            System.err.println("Failed to start server: " + e.getMessage());
        }
    }
}
```

Custom Configuration:
```java
import com.offlineintelligence.Config;
import com.offlineintelligence.Server;

public class CustomAIServer {
    public static void main(String[] args) {
        Config customConfig = new Config();
        customConfig.setApiHost("0.0.0.0");
        customConfig.setApiPort(8080);
        customConfig.setModelPath("/path/to/model.gguf");
        
        try {
            Server.runServer(customConfig);
        } catch (OfflineIntelligenceException e) {
            System.err.println("Error: " + e.getMessage());
        }
    }
}
```

### C++ Usage

Installation:
Clone the repository and copy the header files to your project:
- Clone from: https://github.com/OfflineIntelligence/offline-intelligence.git
- Copy offline_intelligence headers to your project include directory

Basic Usage:
```cpp
#include <offline_intelligence/offline_intelligence.hpp>
#include <iostream>

using namespace offline_intelligence;

int main() {
    try {
        // Load configuration from environment variables
        Config config = Config::from_env();
        
        // Start the server with exception handling
        bool success = Server::run_server(config);
        std::cout << "Server started: " << success << std::endl;
        
    } catch (const OfflineIntelligenceException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

Custom Configuration:
```cpp
#include <offline_intelligence/offline_intelligence.hpp>

using namespace offline_intelligence;

int main() {
    Config custom_config;
    custom_config.api_host = "0.0.0.0";
    custom_config.api_port = 8080;
    custom_config.model_path = "/path/to/model.gguf";
    
    bool success = Server::run_server(custom_config);
    return success ? 0 : 1;
}
```

## Installation

Prerequisites:
- Rust Toolchain: rustc 1.70+ (for building from source)
- LLaMA.cpp: Pre-built binary or build from source
- System Libraries: OpenSSL, pkg-config (Linux/MacOS)

## Model Download & Local Usage

### Downloading Models

The Offline Intelligence Library works with GGUF format models. You can download pre-trained models from the following sources:

1. **Hugging Face Model Hub**: Visit https://huggingface.co/models and search for GGUF compatible models
2. **GGML Model Repository**: Check https://huggingface.co/TheBloke for popular models converted to GGUF format
3. **Official LLaMA Models**: Available through Meta's official channels after registration
4. **Popular Model Examples**:
   - Llama 3 8B Q4: https://huggingface.co/TheBloke/Llama-3-8B-Instruct-GGUF
   - Mistral 7B Q4: https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF
   - Phi-3 Mini Q4: https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf

### Setting Up Local Models

1. Create a directory for your models:
   ```bash
   mkdir -p ~/.offline-intelligence/models
   ```

2. Download a GGUF model file (e.g., `model.q4_k_m.gguf`) to your models directory

3. Set the MODEL_PATH environment variable to point to your downloaded model:
   ```bash
   export MODEL_PATH="/path/to/your/model.q4_k_m.gguf"
   ```

### Using the Library with Local Models

1. Configure the library to use your local model:
   - Set `MODEL_PATH` to the path of your downloaded GGUF file
   - Set `LLAMA_BIN` to the path of your llama.cpp server binary
   - Adjust `CTX_SIZE` based on your model's context window (4096 for most models, 8192 for newer models)

2. Example .env configuration:
   ```env
   LLAMA_BIN=/usr/local/bin/llama-server
   MODEL_PATH=/home/user/.offline-intelligence/models/llama-3-8b-instruct.q4_k_m.gguf
   CTX_SIZE=8192
   BATCH_SIZE=512
   THREADS=8
   GPU_LAYERS=20
   API_HOST=127.0.0.1
   API_PORT=8000
   ```

3. Start the server with your local model:
   ```bash
   # Using Rust
   cargo run
   
   # Or using Python after installation
   python -c "from offline_intelligence import Config, run_server; run_server(Config.from_env())"
   ```

Package Managers:

Rust (Cargo):
Add offline-intelligence = "0.1.2" to your Cargo.toml dependencies

Python (PyPI):
Run pip install offline-intelligence

JavaScript/Node.js (npm):
Run npm install offline-intelligence

Java (JitPack):
Add the JitPack repository and dependency to your pom.xml or build.gradle:
- Repository: https://jitpack.io
- GroupId: com.github.OfflineIntelligence
- ArtifactId: offline-intelligence
- Version: 0.1.2

C++ (Header-only):
Clone the repository and copy the header files to your project:
- Clone from: https://github.com/OfflineIntelligence/offline-intelligence.git
- Copy offline_intelligence headers to your project include directory

## Configuration

Environment Variables:
The library uses environment variables for configuration, with automatic hardware detection capabilities:

Variable Descriptions:
- LLAMA_BIN: Path to llama.cpp server binary (required, no auto-detect)
- MODEL_PATH: Path to GGUF model file (required, auto-detect available)
- API_HOST: API server host (default: 127.0.0.1, no auto-detect)
- API_PORT: API server port (default: 8000, no auto-detect)
- LLAMA_HOST: LLaMA backend host (default: 127.0.0.1, no auto-detect)
- LLAMA_PORT: LLaMA backend port (default: 8081, no auto-detect)
- CTX_SIZE: Context window size (default: 8192, auto-detect available)
- BATCH_SIZE: Processing batch size (default: 256, auto-detect available)
- THREADS: CPU thread count (default: 6, auto-detect available)
- GPU_LAYERS: GPU acceleration layers (default: 20, auto-detect available)
- MAX_CONCURRENT_STREAMS: Max concurrent requests (default: 4, no auto-detect)
- PROMETHEUS_PORT: Metrics endpoint port (default: 9000, no auto-detect)
- REQUESTS_PER_SECOND: Rate limiting threshold (default: 24, no auto-detect)

Auto-Detection Capabilities:
The library automatically optimizes configuration based on available hardware:

CPU Detection:
Thread Count is automatically calculated based on CPU core count:
- 1-2 cores: 1 thread
- 3-4 cores: 60% of cores
- 5-8 cores: 60% of cores
- 9-16 cores: 50% of cores
- 17-32 cores: 40% of cores
- 32+ cores: Maximum 16 threads

GPU Detection:
VRAM-Based automatically determines GPU layers based on available VRAM:
- 0-4GB: 12 GPU layers
- 5-8GB: 20 GPU layers
- 9-12GB: 32 GPU layers
- 13-16GB: 40 GPU layers
- 16GB+: 50 GPU layers

Memory Optimization:
- Context Size: Inferred from model filename and adjusted for available RAM
- Batch Size: Calculated based on context size and available memory
- Safety Limits: Prevents memory exhaustion on constrained systems

Sample .env File Configuration:
Configure LLM settings with LLAMA_BIN and MODEL_PATH
Set API configuration with API_HOST and API_PORT
Use auto-detection for performance tuning (THREADS, GPU_LAYERS, CTX_SIZE, BATCH_SIZE)
Configure resource management and monitoring settings

## Platform-Specific and Use Case Configuration Guide

The Offline Intelligence Library is designed to work across different platforms and hardware configurations. Below are optimized .env configurations for various use cases:

### Platform-Specific Configurations

#### Windows Configuration
```env
# Windows-specific paths
LLAMA_BIN=C:\llama\llama-server.exe
MODEL_PATH=C:\models\your-model.gguf
API_HOST=127.0.0.1
API_PORT=8000
# Windows may need lower concurrency
MAX_CONCURRENT_STREAMS=2
REQUESTS_PER_SECOND=12
```

#### macOS Configuration
```env
# macOS-specific paths
LLAMA_BIN=/usr/local/bin/llama-server
MODEL_PATH=/Users/$USER/.offline-intelligence/models/your-model.gguf
API_HOST=127.0.0.1
API_PORT=8000
# macOS Metal support
GPU_LAYERS=10  # Adjust based on your Mac's GPU
```

#### Linux Configuration
```env
# Linux-specific paths
LLAMA_BIN=/usr/local/bin/llama-server
MODEL_PATH=/home/$USER/.offline-intelligence/models/your-model.gguf
API_HOST=0.0.0.0  # Allow external connections if needed
API_PORT=8000
```

### Hardware-Specific Configurations

#### CPU-Only Systems
```env
# CPU-only configuration (no GPU acceleration)
GPU_LAYERS=0
THREADS=8  # Adjust based on your CPU core count
CTX_SIZE=4096  # Reduce context size for better CPU performance
BATCH_SIZE=128  # Lower batch size for CPU
# Conservative settings for memory usage
MAX_CONCURRENT_STREAMS=2
```

#### GPU-Accelerated Systems
```env
# GPU-optimized configuration
GPU_LAYERS=35  # Adjust based on your GPU VRAM (see below)
THREADS=4  # Reduce CPU threads when using GPU
CTX_SIZE=8192  # Larger context when GPU accelerated
BATCH_SIZE=512  # Higher batch size for GPU efficiency
# Higher concurrency with GPU acceleration
MAX_CONCURRENT_STREAMS=6
```

#### GPU VRAM-Specific Settings
- **4GB VRAM**: `GPU_LAYERS=12`, `CTX_SIZE=2048`, `BATCH_SIZE=64`
- **6GB VRAM**: `GPU_LAYERS=20`, `CTX_SIZE=4096`, `BATCH_SIZE=128`
- **8GB VRAM**: `GPU_LAYERS=25`, `CTX_SIZE=4096`, `BATCH_SIZE=256`
- **12GB+ VRAM**: `GPU_LAYERS=40+`, `CTX_SIZE=8192`, `BATCH_SIZE=512`

#### Cloud/Server Deployment
```env
# Cloud/server optimized settings
API_HOST=0.0.0.0
API_PORT=8000
# Allow higher concurrency for server usage
MAX_CONCURRENT_STREAMS=8
REQUESTS_PER_SECOND=48
# Enable metrics for monitoring
PROMETHEUS_PORT=9000
# Conservative resource usage
HEALTH_TIMEOUT_SECONDS=120
```

#### Compute Cluster Configuration
```env
# High-performance cluster settings
GPU_LAYERS=auto  # Max GPU utilization
THREADS=auto  # Use more CPU threads
CTX_SIZE=auto  # Very large context window
BATCH_SIZE=auto  # Large batch processing
MAX_CONCURRENT_STREAMS=auto  # High concurrency
```

#### Low-Resource/Edge Device Configuration
```env
# Minimal resource usage for edge devices
GPU_LAYERS=0
THREADS=2
CTX_SIZE=2048
BATCH_SIZE=32
MAX_CONCURRENT_STREAMS=1
REQUESTS_PER_SECOND=6
QUEUE_SIZE=20
# Reduce timeouts for quicker response to resource constraints
HEALTH_TIMEOUT_SECONDS=30
QUEUE_TIMEOUT_SECONDS=15
```

### Model-Specific Tuning

#### Small Models (<3B parameters)
```env
CTX_SIZE=2048
BATCH_SIZE=64
THREADS=2
GPU_LAYERS=5  # May not need GPU acceleration
```

#### Medium Models (3B-20B parameters)
```env
CTX_SIZE=4096
BATCH_SIZE=256
THREADS=6
GPU_LAYERS=20  # Beneficial for medium models
```

#### Large Models (>30B parameters)
```env
CTX_SIZE=8192
BATCH_SIZE=512
THREADS=8
GPU_LAYERS=35  # Highly recommended for large models
```

### Use Case-Specific Examples

#### Chatbot Application
```env
CTX_SIZE=4096  # Good for conversation history
BATCH_SIZE=128
MAX_CONCURRENT_STREAMS=4
HEALTH_TIMEOUT_SECONDS=60
QUEUE_TIMEOUT_SECONDS=45  # Reasonable timeout for chat
```

#### Content Generation
```env
CTX_SIZE=8192  # Larger context for creative tasks
BATCH_SIZE=512  # Higher throughput for generation
MAX_CONCURRENT_STREAMS=2  # Fewer but longer requests
REQUESTS_PER_SECOND=12  # Lower rate limit for longer generations
```

#### API Service
```env
API_HOST=0.0.0.0  # Listen on all interfaces
CTX_SIZE=4096
BATCH_SIZE=256
MAX_CONCURRENT_STREAMS=8  # Handle multiple API requests
REQUESTS_PER_SECOND=36  # Moderate rate limiting
PROMETHEUS_PORT=9000  # Enable metrics
```

## API Reference

Core Endpoints:

POST /generate/stream:
Stream generation endpoint for real-time responses.

Request Body includes:
- messages: Array of message objects with role and content
- session_id: Identifier for conversation continuity
- temperature: Sampling temperature parameter
- max_tokens: Maximum tokens to generate
- top_p: Nucleus sampling parameter
- frequency_penalty: Penalty for frequent tokens

Response: Server-Sent Events (SSE) stream with JSON chunks

GET /healthz:
Health check endpoint.

Response contains:
- status: Health status (OK)
- timestamp: Current timestamp

GET /readyz:
Readiness check endpoint.

Response contains:
- status: Readiness status (READY)
- backend_connected: Boolean indicating backend connection
- model_loaded: Boolean indicating if model is loaded

GET /metrics:
Prometheus metrics endpoint.

Response: Plain text metrics in Prometheus format

Admin Endpoints:

GET /admin/status:
System status information.

Response contains:
- status: Current system status
- version: Library version
- uptime: Server uptime
- active_connections: Number of active connections
- total_requests: Total requests served

POST /admin/load:
Load a specific model.

Request Body includes:
- model_path: Path to the model file
- ctx_size: Context size to use

Response contains:
- success: Boolean indicating success
- message: Status message

POST /admin/stop:
Stop the backend server.

Response contains:
- success: Boolean indicating success
- message: Status message

Memory Endpoints:

GET /memory/stats/{session_id}:
Get memory statistics for a session.

Response contains:
- session_id: The session identifier
- message_count: Number of messages in session
- total_tokens: Total tokens in session
- estimated_cost: Estimated cost of the session

## Performance

Benchmark Results:
Performance varies based on model and hardware configuration:
- Llama 3 8B Q4 on RTX 4090: 120 tokens/sec, 8GB GPU + 4GB RAM, 15ms average latency
- Mistral 7B Q4 on RTX 3080: 85 tokens/sec, 6GB GPU + 3GB RAM, 22ms average latency
- Phi-3 Mini Q4 on i9-13900K: 45 tokens/sec, 12GB RAM, 35ms average latency

Optimization Strategies:

Hardware-Aware Scheduling:
- CPU: Thread pool optimized for core count
- GPU: Layer distribution based on VRAM availability
- Memory: Adaptive batching based on available RAM

Resource Management:
- Connection Pooling: Reusable backend connections
- Request Queuing: Fair scheduling with timeout handling
- Memory Recycling: Object pooling for reduced GC pressure

Performance Tuning:

High-Throughput Configuration:
Configure THREADS to 16, GPU_LAYERS to 40, CTX_SIZE to 8192, BATCH_SIZE to 512, and MAX_CONCURRENT_STREAMS to 8

Low-Resource Configuration:
Configure THREADS to 4, GPU_LAYERS to 12, CTX_SIZE to 2048, BATCH_SIZE to 64, and MAX_CONCURRENT_STREAMS to 2

## Security

Security Model:

Isolation:
- Process Isolation: LLM backend runs in separate process
- Memory Protection: Memory-safe Rust implementation
- Network Isolation: Configurable network binding

Authentication:
- API Keys: Optional API key authentication
- Rate Limiting: Built-in request throttling
- IP Filtering: Configurable IP allow/deny lists

Data Protection:
- Encryption at Rest: Optional database encryption
- Secure Defaults: Safe-by-default configuration
- Audit Logging: Comprehensive activity logs

Compliance:

Privacy Controls:
- Local Processing: All data processed locally
- No External Dependencies: Offline-first design
- Data Retention: Configurable conversation retention

Enterprise Security:
- Role-Based Access: Fine-grained permission controls
- Audit Trails: Comprehensive event logging
- Compliance Reports: Automated compliance reporting

## Technical Specifications

System Overview:
The Offline Intelligence Library is a high-performance, cross-platform LLM inference engine that provides native bindings for Rust, Python, Java, C++, and JavaScript. The system is designed for enterprise-grade deployments with emphasis on privacy, performance, and scalability.

Core Capabilities:
- LLM Integration: Direct integration with llama.cpp backend
- Memory Management: Persistent conversation storage with SQLite
- API Gateway: RESTful HTTP interface with streaming support
- Resource Management: Hardware-aware optimization and allocation
- Monitoring: Prometheus metrics and structured logging

Target Platforms:
- Operating Systems: Windows 10+, Linux (Ubuntu 20.04+, CentOS 8+), macOS 11+
- Architectures: x86_64, ARM64
- Languages: Rust, Python, Java, JavaScript/Node.js

Component Specifications:

LLM Integration Layer:
- Backend: llama.cpp integration via FFI
- Streaming: Server-Sent Events (SSE) with JSON chunks
- Models: GGUF format support
- Concurrency: Up to 64 concurrent streams
- Timeouts: Configurable request and stream timeouts
- Health: Continuous backend health monitoring

Memory Management System:
- Storage: SQLite database with ACID transactions
- Tables: Conversations, Messages, Sessions, Embeddings
- Indexing: Optimized indexes for fast retrieval
- Migrations: Automated schema evolution
- Compression: Optional data compression for large histories
- Retention: Configurable data retention policies

API Gateway:
- Framework: Axum web framework
- Endpoints: RESTful HTTP interface
- Rate Limiting: Configurable RPS with burst handling
- CORS: Flexible cross-origin policies
- Security: Built-in authentication and authorization
- Queuing: Request queue management with timeout controls

Resource Management:
- Auto-detection: Hardware-aware configuration
- CPU: Dynamic thread pool sizing
- GPU: VRAM-based layer assignment
- Memory: Adaptive memory allocation
- Concurrency: Configurable request limits
- Safety: Resource exhaustion prevention

Monitoring & Telemetry:
- Metrics: Prometheus-compatible format
- Logging: Structured JSON logging
- Tracing: Distributed request tracing
- Alerting: Configurable threshold alerts
- Health: Liveness and readiness checks
- Dashboards: Pre-built monitoring dashboards

Performance Specifications:
- Latency: <100ms average response time (typical queries)
- Throughput: 100+ requests per second on commodity hardware
- Memory: <4GB RAM for basic operation, scalable to 32GB+
- CPU: Support for 4-64 cores with optimal utilization
- GPU: Efficient utilization of available GPU resources
- Concurrent: Support for 1-1000 concurrent connections

Configuration Specifications:
Environment Variables are categorized into:
- Core Configuration: LLAMA_BIN, MODEL_PATH, API_HOST, API_PORT, etc.
- Performance Configuration: CTX_SIZE, BATCH_SIZE, THREADS, GPU_LAYERS, etc.
- Resource Management: HEALTH_TIMEOUT_SECONDS, PROMETHEUS_PORT, etc.
- Queue Configuration: QUEUE_SIZE, QUEUE_TIMEOUT_SECONDS

Configuration Validation includes:
- Required Fields: All mandatory configuration values must be present
- Value Ranges: Configuration values must fall within acceptable ranges
- Dependency Checks: Interdependent configuration values validated
- Type Safety: Strong typing with validation

Auto-Detection Specifications:
- CPU Detection: Thread count based on core count
- GPU Detection: GPU layers based on VRAM availability
- Memory Optimization: Context and batch sizes adjusted for available RAM
- Safety Limits: Resource usage capped to prevent exhaustion

Language Binding Specifications:
Each language binding maintains consistency while leveraging platform-specific optimizations:
- Rust: Zero-copy data transfer, async/await integration
- Python: PyO3 integration, NumPy compatibility
- JavaScript: Native addon, V8 integration
- Java: JNI optimizations, GC awareness
- C++: Template metaprogramming, RAII patterns

Cross-Language Consistency ensures:
- Configuration: Same structure across all languages
- API Endpoints: Identical HTTP interface
- Data Formats: Compatible serialization formats
- Error Handling: Consistent error types and codes
- Documentation: Uniform documentation standards

## API Documentation

The Offline Intelligence Library provides a comprehensive RESTful API for LLM inference with memory management capabilities. All endpoints follow consistent patterns and return standardized responses across all language bindings.

Authentication:
Most endpoints do not require authentication by default. However, authentication can be enabled through configuration using API keys in headers or as query parameters.

Core Endpoints:

POST /generate/stream:
Stream generation endpoint for real-time responses.

Request parameters include messages array with roles and content, session_id for continuity, temperature for sampling, max_tokens for generation limit, and other model parameters.

Response is streamed in Server-Sent Events format with tokens and completion indicators.

GET /healthz:
Health check endpoint to verify service availability.

Returns status, timestamp, version, and backend connection status.

GET /readyz:
Readiness check endpoint to verify service readiness.

Returns status, timestamp, backend connection status, and model loading status.

GET /metrics:
Prometheus metrics endpoint for monitoring.

Returns metrics in Prometheus-compatible text format covering requests, duration, resources, and performance.

Admin Endpoints:

GET /admin/status:
Retrieve system status information.

Returns status, version, uptime, request counts, resource usage, and backend information.

POST /admin/load:
Load a specific model into the LLM backend.

Accepts model path, context size, GPU layers, and batch size parameters.

Returns success status, message, and model information.

POST /admin/stop:
Stop the backend LLM service.

Returns success status and message.

Memory Management Endpoints:

GET /memory/stats/{session_id}:
Get memory statistics for a specific session.

Returns session information, message counts, token counts, timestamps, and storage size.

Error Handling:
All error responses follow a standard format with error type, message, details, and timestamp.

Common error types include validation_error, authentication_error, authorization_error, not_found, rate_limit_exceeded, server_error, backend_unavailable, model_load_error, and resource_exhausted.

## Developer Guide

Getting Started:
Install the library using the package manager for your preferred language.
Set up the required environment variables for LLaMA binary and model path.
Load configuration from environment variables.
Start the server with the loaded configuration.

Architecture Deep Dive:
Understand the interconnected components: LLM Integration Layer, Memory Management System, API Gateway, and Resource Manager.
Learn about the request processing flow from client request to response delivery.
Explore the data flow architecture and how components interact.

Configuration Guide:
Use environment variables for configuration with optional auto-detection features.
Start with auto-detection values to let the system optimize for your hardware.
Monitor resource usage and fine-tune based on workload patterns.
Test configuration changes in a staging environment.

API Usage:
Use the core endpoints for streaming generation, health checks, and metrics.
Manage sessions with unique session identifiers for conversation continuity.
Handle common error responses appropriately in your client applications.

Performance Optimization:
Tune configuration based on your hardware specifications and use case requirements.
Monitor key metrics for optimization including memory usage, CPU usage, and GPU utilization.
Apply performance tips such as matching context size to use case and balancing GPU/CPU resources.

Troubleshooting:
Address common issues like model loading failures, performance problems, connection issues, and memory issues.
Use diagnostic commands to check system resources, network connectivity, and library diagnostics.
Analyze logs in the structured JSON format for issue resolution.

Best Practices:
Follow security best practices including network binding, authentication, rate limiting, and firewall configuration.
Apply performance best practices such as resource matching, model selection, and connection management.
Implement operational best practices for configuration management, backups, health checks, and monitoring.
Adhere to development best practices for testing, error handling, and version management.

## Use Cases and Applications

Multiple Programming Languages Support:
The library provides native bindings for Rust, Python, Java, C++, and JavaScript, allowing seamless integration into projects built with different technologies. Each binding maintains API consistency while leveraging language-specific optimizations and idioms.

Multiple OS Setups:
Support for Windows, Linux, and macOS across different architectures (x86_64, ARM64) makes the library versatile for deployment in diverse computing environments. The hardware-aware auto-detection adjusts configuration based on the underlying OS and hardware capabilities.

Multiple Use Cases:
The Offline Intelligence Library addresses various use cases including:
- Enterprise AI applications requiring privacy and data security
- Edge computing scenarios where internet connectivity is limited
- Cost-sensitive deployments avoiding cloud-based AI services
- High-performance applications needing optimized inference
- Multi-modal applications requiring memory management
- Scalable services requiring concurrent request handling

## Contributing

Development Setup:
Clone the repository and install prerequisites including Rust toolchain and LLaMA.cpp.
Build the library using cargo build command.
Follow Rust guidelines for code style, documentation, testing, and Clippy compliance.
Maintain API consistency with backward compatibility and uniform configuration structure.

Testing:
Run unit tests with cargo test.
Execute integration tests with cargo test --test integration.
Perform performance tests with cargo bench.

## License

Open Source License:
The core 80% of the Offline Intelligence Library is released under the Apache 2.0 License, providing permissive usage rights while maintaining attribution requirements.

Commercial Extensions:
The remaining 20% of functionality, including advanced context management and enterprise features, is available under commercial licensing terms.

Third-Party Licenses:
This software incorporates components from LLaMA.cpp (MIT), Axum (MIT), Tokio (MIT), Serde (MIT/Apache 2.0), and SQLite (Public Domain).

## Support

Documentation includes comprehensive API reference, examples for all languages, and tutorials for common use cases.

Community support is available through GitHub Issues for bug reports, Discussions for Q&A, and contribution guidelines.

Enterprise support options include priority support for commercial users, professional services for consulting and custom development, and training sessions.

## Changelog

### v0.1.2 (2026-02-07)
- Added automatic hardware detection
- Improved memory management
- Enhanced error handling
- Fixed critical security vulnerabilities

### v0.1.1 (2025-12-15)
- Initial public release with multi-language bindings, core LLM integration, and memory management system

## Citation

If you use Offline Intelligence Library in your research, please cite it as follows:

```
Offline Intelligence Library
Author: Offline Intelligence Team
URL: https://github.com/OfflineIntelligence/offline-intelligence
```
