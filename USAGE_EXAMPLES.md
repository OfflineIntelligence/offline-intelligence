# Offline Intelligence - Usage Examples

## 🎯 Multi-Language Integration Examples

These examples demonstrate how to use the Offline Intelligence library in different programming languages.

## 🐍 Python Usage

```python
# Install: pip install offline-intelligence

from offline_intelligence import Config, run_server

# Basic usage
config = Config.from_env()
success = run_server(config)
print(f"Server started: {success}")

# Custom configuration
config = Config()
config.api_host = "0.0.0.0"
config.api_port = 8080
config.model_path = "/path/to/model.gguf"

# Advanced usage with error handling
try:
    result = run_server(config)
    if result:
        print("Server running successfully")
except Exception as e:
    print(f"Failed to start server: {e}")
```

## ➕ C++ Usage

```cpp
// Install via CMake or manual compilation
#include <offline_intelligence/offline_intelligence.h>
#include <iostream>

using namespace offline_intelligence;

int main() {
    try {
        // Basic usage
        Config config = Config::from_env();
        bool success = Server::run_server(config);
        std::cout << "Server started: " << success << std::endl;
        
        // Custom configuration
        Config custom_config;
        custom_config.api_host = "0.0.0.0";
        custom_config.api_port = 8080;
        custom_config.model_path = "/path/to/model.gguf";
        
        Server::run_server(custom_config);
        
    } catch (const OfflineIntelligenceException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

## 🟨 JavaScript/Node.js Usage

```javascript
// Install: npm install offline-intelligence

const { Config, OfflineIntelligence } = require('offline-intelligence');

async function main() {
    try {
        // Basic usage
        const config = Config.fromEnv();
        const ai = new OfflineIntelligence(config);
        
        // Check server health
        const health = await ai.healthCheck();
        console.log('Server health:', health);
        
        // Generate text
        const stream = await ai.generateStream('Hello, world!');
        stream.on('data', (chunk) => {
            process.stdout.write(chunk.toString());
        });
        
        // Custom configuration
        const customConfig = new Config();
        customConfig.apiHost = '0.0.0.0';
        customConfig.apiPort = 8080;
        
    } catch (error) {
        console.error('AI operation failed:', error.message);
    }
}

main();
```

## ☕ Java Usage

```java
// Add Maven dependency:
// <dependency>
//     <groupId>com.offlineintelligence</groupId>
//     <artifactId>offline-intelligence-java</artifactId>
//     <version>0.1.1</version>
// </dependency>

import com.offlineintelligence.Config;
import com.offlineintelligence.Server;
import com.offlineintelligence.OfflineIntelligenceException;

public class AIServer {
    public static void main(String[] args) {
        try {
            // Basic usage
            Config config = Config.fromEnv();
            boolean success = Server.runServer(config);
            System.out.println("Server started: " + success);
            
            // Custom configuration
            Config customConfig = new Config();
            customConfig.setApiHost("0.0.0.0");
            customConfig.setApiPort(8080);
            customConfig.setModelPath("/path/to/model.gguf");
            
            Server.runServer(customConfig);
            
        } catch (OfflineIntelligenceException e) {
            System.err.println("Failed to start server: " + e.getMessage());
        }
    }
}
```

## ➕ Rust Usage (Native)

```rust
// Add to Cargo.toml:
// [dependencies]
// offline-intelligence = "0.1.1"

use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic usage
    let config = Config::from_env()?;
    run_server(config).await?;
    
    // Custom configuration
    let mut config = Config::from_env()?;
    config.api_host = "0.0.0.0".to_string();
    config.api_port = 8080;
    
    run_server(config).await?;
    
    Ok(())
}
```

## 🔧 HTTP API Usage (Universal)

All languages can interact with the HTTP API directly:

```bash
# Health check
curl http://localhost:8000/healthz

# Generate text (streaming)
curl -X POST http://localhost:8000/generate/stream \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello, world!"}'

# Get status
curl http://localhost:8000/admin/status

# Load model
curl -X POST http://localhost:8000/admin/load \
  -H "Content-Type: application/json" \
  -d '{"model_path": "/path/to/model.gguf"}'
```

## 🚀 Advanced Integration Patterns

### Microservice Architecture
```python
# Python microservice example
from flask import Flask, request, jsonify
from offline_intelligence import Config, run_server

app = Flask(__name__)
config = Config.from_env()

@app.route('/ai/generate', methods=['POST'])
def generate():
    prompt = request.json.get('prompt')
    # Process with Offline Intelligence
    return jsonify({"result": "processed"})

if __name__ == '__main__':
    # Start AI server in background
    import threading
    threading.Thread(target=lambda: run_server(config)).start()
    app.run(host='0.0.0.0', port=5000)
```

### Batch Processing
```javascript
// JavaScript batch processing
const { OfflineIntelligence, Config } = require('offline-intelligence');

async function processBatch(prompts) {
    const config = Config.fromEnv();
    const ai = new OfflineIntelligence(config);
    
    const results = [];
    for (const prompt of prompts) {
        try {
            const result = await ai.generateStream(prompt);
            results.push(result);
        } catch (error) {
            results.push({ error: error.message });
        }
    }
    
    return results;
}
```

### Real-time Streaming
```java
// Java real-time streaming
import com.offlineintelligence.*;
import java.util.concurrent.CompletableFuture;

public class StreamingAI {
    public static void main(String[] args) {
        Config config = Config.fromEnv();
        
        CompletableFuture.supplyAsync(() -> {
            try {
                return Server.runServer(config);
            } catch (OfflineIntelligenceException e) {
                throw new RuntimeException(e);
            }
        }).thenAccept(success -> {
            if (success) {
                System.out.println("Streaming server ready");
                // Implement WebSocket or SSE streaming
            }
        });
    }
}
```

## 📊 Performance Optimization Examples

### Connection Pooling (Python)
```python
import asyncio
from offline_intelligence import Config, run_server

class AIPool:
    def __init__(self, pool_size=4):
        self.pool_size = pool_size
        self.servers = []
    
    async def initialize(self):
        config = Config.from_env()
        for i in range(self.pool_size):
            server_task = asyncio.create_task(run_server(config))
            self.servers.append(server_task)
```

### Resource Management (C++)
```cpp
#include <offline_intelligence/offline_intelligence.h>
#include <memory>

class AIManager {
private:
    std::unique_ptr<offline_intelligence::Config> config;
    
public:
    AIManager() {
        config = std::make_unique<offline_intelligence::Config>(
            offline_intelligence::Config::from_env()
        );
    }
    
    ~AIManager() {
        // Automatic cleanup
    }
};
```

## 🔒 Security Best Practices

### Environment-based Configuration
```python
# Python - Secure configuration
import os
from offline_intelligence import Config

def create_secure_config():
    config = Config()
    config.api_key = os.getenv('OFFLINE_AI_API_KEY')
    config.model_path = os.getenv('MODEL_PATH', '/default/model.gguf')
    return config
```

### Input Validation
```javascript
// JavaScript - Input sanitization
const sanitizePrompt = (prompt) => {
    // Remove potentially harmful characters
    return prompt.replace(/[<>]/g, '');
};

const safeGenerate = async (prompt) => {
    const cleanPrompt = sanitizePrompt(prompt);
    return ai.generateStream(cleanPrompt);
};
```

## 🧪 Testing Examples

### Unit Tests (Python)
```python
import unittest
from offline_intelligence import Config

class TestAIConfig(unittest.TestCase):
    def test_default_config(self):
        config = Config.from_env()
        self.assertIsNotNone(config.api_host)
        self.assertGreater(config.api_port, 0)
    
    def test_custom_config(self):
        config = Config()
        config.api_host = "127.0.0.1"
        config.api_port = 9000
        self.assertEqual(config.api_host, "127.0.0.1")
        self.assertEqual(config.api_port, 9000)
```

### Integration Tests (Java)
```java
@Test
public void testServerStartup() {
    Config config = Config.fromEnv();
    config.setApiPort(8081); // Use different port for testing
    
    boolean result = Server.runServer(config);
    assertTrue(result);
    
    // Test API endpoints
    // ... HTTP client tests
}
```

## 📈 Monitoring and Logging

### Structured Logging (Rust)
```rust
use tracing::{info, error};
use offline_intelligence::{Config, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let config = Config::from_env()?;
    info!("Starting AI server on {}:{}", config.api_host, config.api_port);
    
    match run_server(config).await {
        Ok(_) => info!("Server stopped gracefully"),
        Err(e) => error!("Server failed: {}", e),
    }
    
    Ok(())
}
```

These examples demonstrate the flexibility and power of the Offline Intelligence library across different programming ecosystems while maintaining consistent functionality and performance characteristics.