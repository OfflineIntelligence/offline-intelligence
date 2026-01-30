# Rust Getting Started Guide

## 🦀 Rust Development Setup

Complete guide for using the Offline Intelligence Library in Rust projects.

## 📦 Installation

### Add to Existing Project
```bash
cargo add offline-intelligence
```

### Create New Project
```bash
cargo new my-ai-project
cd my-ai-project
cargo add offline-intelligence
```

## 🛠 Project Setup

### Cargo.toml Configuration
```toml
[dependencies]
offline-intelligence = "0.1.2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Basic Project Structure
```
my-ai-project/
├── src/
│   ├── main.rs          # Main application
│   ├── config.rs        # Configuration management
│   ├── ai_handler.rs    # AI interaction logic
│   └── models.rs        # Data structures
├── Cargo.toml
├── .env                 # Environment variables
└── README.md
```

## ⚙️ Environment Configuration

### .env File Setup
```env
# Model configuration
MODEL_PATH=./models/llama-2-7b.Q4_K_M.gguf
LLAMA_BIN=./binaries/llama-bin
CTX_SIZE=8192
BATCH_SIZE=256
THREADS=6
GPU_LAYERS=20

# Application settings
LOG_LEVEL=info
MAX_CONCURRENT_REQUESTS=10
```

### Loading Environment Variables
```rust
// src/main.rs
use dotenvy::dotenv;
use std::env;

fn load_config() {
    dotenv().ok(); // Load .env file
    
    // Validate required variables
    let required_vars = ["MODEL_PATH", "LLAMA_BIN"];
    for var in &required_vars {
        env::var(var).expect(&format!("{} must be set", var));
    }
    
    println!("Configuration loaded successfully!");
}
```

## 🚀 Basic Usage Examples

### Simple Text Generation
```rust
// src/main.rs
use offline_intelligence::{OfflineIntelligence, Message};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the library
    let oi = OfflineIntelligence::new()?;
    
    // Create conversation messages
    let messages = vec![
        Message::new("user", "Explain Rust ownership in simple terms"),
    ];
    
    // Generate response
    let response = oi.generate_chat_completion(messages, None).await?;
    
    println!("AI Response: {}", response);
    Ok(())
}
```

### Context Management
```rust
// src/ai_handler.rs
use offline_intelligence::{OfflineIntelligence, Message, OptimizationResult};
use std::collections::HashMap;

pub struct AIHandler {
    oi: OfflineIntelligence,
    sessions: HashMap<String, Vec<Message>>,
}

impl AIHandler {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            oi: OfflineIntelligence::new()?,
            sessions: HashMap::new(),
        })
    }
    
    pub async fn chat(
        &mut self, 
        session_id: &str, 
        user_message: &str
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Add user message to session
        let messages = self.sessions.entry(session_id.to_string())
            .or_insert_with(Vec::new);
        messages.push(Message::new("user", user_message));
        
        // Optimize context if needed
        if messages.len() > 10 {
            let result = self.oi.optimize_context(
                session_id, 
                messages.clone(), 
                Some(user_message)
            ).await?;
            
            *messages = result.optimized_messages;
        }
        
        // Generate response
        let response = self.oi.generate_chat_completion(
            messages.clone(), 
            None
        ).await?;
        
        // Add AI response to session
        messages.push(Message::new("assistant", &response));
        
        Ok(response)
    }
}
```

## 🏗 Advanced Patterns

### Async Handler with Connection Pooling
```rust
// src/async_handler.rs
use offline_intelligence::OfflineIntelligence;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct AsyncAIHandler {
    oi: Arc<Mutex<OfflineIntelligence>>,
}

impl AsyncAIHandler {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            oi: Arc::new(Mutex::new(OfflineIntelligence::new()?)),
        })
    }
    
    pub async fn generate_response(
        &self, 
        prompt: &str
    ) -> Result<String, Box<dyn std::error::Error>> {
        let oi = self.oi.lock().await;
        oi.generate_completion(prompt, Some(200)).await
    }
}
```

### Configuration Management
```rust
// src/config.rs
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub model_path: String,
    pub llama_bin: String,
    pub ctx_size: u32,
    pub batch_size: u32,
    pub threads: u32,
    pub gpu_layers: u32,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            model_path: env::var("MODEL_PATH").expect("MODEL_PATH not set"),
            llama_bin: env::var("LLAMA_BIN").expect("LLAMA_BIN not set"),
            ctx_size: env::var("CTX_SIZE")
                .unwrap_or_else(|_| "8192".to_string())
                .parse()
                .expect("Invalid CTX_SIZE"),
            batch_size: env::var("BATCH_SIZE")
                .unwrap_or_else(|_| "256".to_string())
                .parse()
                .expect("Invalid BATCH_SIZE"),
            threads: env::var("THREADS")
                .unwrap_or_else(|_| "6".to_string())
                .parse()
                .expect("Invalid THREADS"),
            gpu_layers: env::var("GPU_LAYERS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .expect("Invalid GPU_LAYERS"),
        }
    }
}
```

## 🧪 Testing Your Setup

### Unit Tests
```rust
// tests/integration_tests.rs
use offline_intelligence::{OfflineIntelligence, Message};

#[tokio::test]
async fn test_basic_generation() {
    let oi = OfflineIntelligence::new().unwrap();
    
    let response = oi.generate_completion(
        "Say hello in French", 
        Some(20)
    ).await.unwrap();
    
    assert!(!response.is_empty());
    println!("Response: {}", response);
}

#[tokio::test]
async fn test_chat_completion() {
    let oi = OfflineIntelligence::new().unwrap();
    
    let messages = vec![
        Message::new("user", "What is 2+2?"),
    ];
    
    let response = oi.generate_chat_completion(messages, None).await.unwrap();
    assert!(!response.is_empty());
    
    println!("Chat response: {}", response);
}
```

### Example Application
```rust
// examples/chat_app.rs
use offline_intelligence::{OfflineIntelligence, Message};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Offline Intelligence Chat");
    println!("Type 'quit' to exit\n");
    
    let oi = OfflineIntelligence::new()?;
    let mut messages = Vec::new();
    
    loop {
        print!("You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.eq_ignore_ascii_case("quit") {
            break;
        }
        
        messages.push(Message::new("user", input));
        
        let response = oi.generate_chat_completion(messages.clone(), None).await?;
        println!("AI: {}\n", response);
        
        messages.push(Message::new("assistant", &response));
    }
    
    Ok(())
}
```

Run the example:
```bash
cargo run --example chat_app
```

## 🚀 Performance Optimization

### Thread Pool Configuration
```rust
// src/performance.rs
use tokio::runtime::Builder;

fn create_optimized_runtime() -> tokio::runtime::Runtime {
    Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .thread_name("ai-worker")
        .enable_all()
        .build()
        .unwrap()
}
```

### Batch Processing
```rust
// Process multiple requests efficiently
async fn batch_process_prompts(
    oi: &OfflineIntelligence,
    prompts: Vec<String>
) -> Vec<Result<String, Box<dyn std::error::Error>>> {
    let mut handles = Vec::new();
    
    for prompt in prompts {
        let oi_clone = oi.clone();
        let handle = tokio::spawn(async move {
            oi_clone.generate_completion(&prompt, Some(100)).await
        });
        handles.push(handle);
    }
    
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }
    
    results
}
```

## 🛠 Development Workflow

### Development Setup Script
```bash
#!/bin/bash
# setup_dev.sh

echo "🚀 Setting up Offline Intelligence development environment"

# Install dependencies
cargo check

# Download test model (if not exists)
if [ ! -f "./models/test-model.gguf" ]; then
    echo "📥 Downloading test model..."
    mkdir -p models
    # Add model download command here
fi

# Run tests
echo "🧪 Running tests..."
cargo test

echo "✅ Development environment ready!"
```

### Continuous Integration
```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Run tests
      run: cargo test
    - name: Check formatting
      run: cargo fmt -- --check
    - name: Run clippy
      run: cargo clippy -- -D warnings
```

## 🐛 Troubleshooting

### Common Compilation Issues

**Missing Dependencies:**
```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install build-essential

# Install system dependencies (macOS)
brew install llvm
```

**Linking Errors:**
```toml
# Cargo.toml - Add if needed
[build-dependencies]
cc = "1.0"
```

### Runtime Issues

**Model Loading Failures:**
```rust
// Add detailed error handling
match OfflineIntelligence::new() {
    Ok(oi) => println!("✅ Library initialized"),
    Err(e) => {
        eprintln!("❌ Failed to initialize: {}", e);
        std::process::exit(1);
    }
}
```

**Performance Problems:**
```rust
// Monitor performance
use std::time::Instant;

let start = Instant::now();
let result = oi.generate_completion(prompt, None).await?;
let duration = start.elapsed();

println!("Generation took: {:?}", duration);
```

## 📚 Next Steps

1. **Explore Examples**: Check the `examples/` directory in the repository
2. **Advanced Configuration**: Read [CONFIGURATION_GUIDE.md](CONFIGURATION_GUIDE.md)
3. **API Reference**: Visit the [documentation](https://docs.rs/offline-intelligence)
4. **Community**: Join discussions on [GitHub Issues](https://github.com/OfflineIntelligence/offline-intelligence/issues)

---

**Ready to build?** Start with the example application and customize it for your needs!