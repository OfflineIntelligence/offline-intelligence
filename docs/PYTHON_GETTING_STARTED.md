# Python Getting Started Guide

## 🐍 Python Development Setup

Complete guide for using the Offline Intelligence Library in Python projects.

## 📦 Installation

### Basic Installation
```bash
pip install offlineintelligence
```

### Development Installation
```bash
# Install with development dependencies
pip install offlineintelligence[dev]

# Install with numpy support
pip install offlineintelligence[numpy]
```

### Virtual Environment Setup
```bash
# Create virtual environment
python m venv ai_project_env
source ai_project_env/bin/activate  # Linux/macOS
# or
ai_project_env\Scripts\activate     # Windows

# Install the library
pip install offlineintelligence
```

## 🛠 Project Setup

### Requirements File
```txt
# requirements.txt
offlineintelligence==0.1.2
pythondotenv>=0.19.0
numpy>=1.21.0
```

### Basic Project Structure
```
aiproject/
├── src/
│   ├── __init__.py
│   ├── main.py              # Main application
│   ├── config.py            # Configuration management
│   ├── ai_handler.py        # AI interaction logic
│   └── utils.py             # Helper functions
├── models/
│   └── llama27b.gguf      # AI model file
├── config/
│   └── .env                 # Environment variables
├── tests/
│   └── test_ai.py           # Unit tests
├── requirements.txt
└── README.md
```

## ⚙️ Environment Configuration

### .env File Setup
```env
# Model configuration
MODEL_PATH=./models/llama27b.Q4_K_M.gguf
LLAMA_BIN=./binaries/llamabin
CTX_SIZE=8192
BATCH_SIZE=256
THREADS=6
GPU_LAYERS=20

# Application settings
LOG_LEVEL=INFO
MAX_TOKENS=200
TEMPERATURE=0.8
```

### Loading Environment Variables
```python
# src/config.py
import os
from dotenv import load_dotenv
from typing import Optional

load_dotenv()

class Config:
    # Required settings
    MODEL_PATH: str = os.getenv("MODEL_PATH", "")
    LLAMA_BIN: str = os.getenv("LLAMA_BIN", "")
    
    # Optional settings with defaults
    CTX_SIZE: int = int(os.getenv("CTX_SIZE", "8192"))
    BATCH_SIZE: int = int(os.getenv("BATCH_SIZE", "256"))
    THREADS: int = int(os.getenv("THREADS", "6"))
    GPU_LAYERS: int = int(os.getenv("GPU_LAYERS", "20"))
    
    # Application settings
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "INFO")
    MAX_TOKENS: int = int(os.getenv("MAX_TOKENS", "200"))
    TEMPERATURE: float = float(os.getenv("TEMPERATURE", "0.8"))
    
    @classmethod
    def validate(cls) > bool:
        """Validate required configuration"""
        required = [cls.MODEL_PATH, cls.LLAMA_BIN]
        if not all(required):
            raise ValueError("MODEL_PATH and LLAMA_BIN must be set")
        return True

# Validate configuration on import
Config.validate()
```

## 🚀 Basic Usage Examples

### Simple Text Generation
```python
# src/main.py
from offline_intelligence_py import OfflineIntelligence
import asyncio

async def main():
    # Initialize the library
    oi = OfflineIntelligence()
    
    # Generate simple response
    prompt = "Explain quantum computing in simple terms"
    response = await oi.generate_completion(prompt, max_tokens=150)
    
    print(f"AI Response: {response}")

if __name__ == "__main__":
    asyncio.run(main())
```

### Chat Interface
```python
# src/ai_handler.py
from offline_intelligence_py import OfflineIntelligence, Message
from typing import List, Dict
import asyncio

class ChatHandler:
    def __init__(self):
        self.oi = OfflineIntelligence()
        self.sessions: Dict[str, List[Message]] = {}
    
    async def chat(self, session_id: str, user_message: str) > str:
        # Get or create session
        if session_id not in self.sessions:
            self.sessions[session_id] = []
        
        messages = self.sessions[session_id]
        
        # Add user message
        messages.append(Message("user", user_message))
        
        # Optimize context if conversation is long
        if len(messages) > 10:
            result = await self.oi.optimize_context(
                session_id, messages, user_message
            )
            messages[:] = result["optimized_messages"]
        
        # Generate response
        response = await self.oi.generate_chat_completion(messages)
        
        # Add AI response
        messages.append(Message("assistant", response))
        
        return response

# Usage example
async def chat_example():
    handler = ChatHandler()
    
    # Simulate conversation
    session_id = "user123"
    responses = []
    
    messages = [
        "Hello!",
        "What can you help me with?",
        "Tell me about artificial intelligence"
    ]
    
    for msg in messages:
        response = await handler.chat(session_id, msg)
        responses.append(response)
        print(f"User: {msg}")
        print(f"AI: {response}\n")

if __name__ == "__main__":
    asyncio.run(chat_example())
```

## 🏗 Advanced Patterns

### Async Context Manager
```python
# src/async_handler.py
import asyncio
from offline_intelligence_py import OfflineIntelligence
from contextlib import asynccontextmanager

class AsyncAIHandler:
    def __init__(self):
        self.oi = None
    
    @asynccontextmanager
    async def get_ai_instance(self):
        """Async context manager for AI instance"""
        if self.oi is None:
            self.oi = OfflineIntelligence()
        try:
            yield self.oi
        finally:
            # Cleanup if needed
            pass
    
    async def process_batch(self, prompts: list[str]) > list[str]:
        """Process multiple prompts concurrently"""
        async with self.get_ai_instance() as ai:
            tasks = [
                ai.generate_completion(prompt, max_tokens=100)
                for prompt in prompts
            ]
            return await asyncio.gather(*tasks)

# Usage
async def batch_processing():
    handler = AsyncAIHandler()
    
    prompts = [
        "Summarize climate change",
        "Explain machine learning",
        "Describe renewable energy"
    ]
    
    results = await handler.process_batch(prompts)
    for prompt, result in zip(prompts, results):
        print(f"Prompt: {prompt}")
        print(f"Response: {result}\n")
```

### ConfigurationBased Initialization
```python
# src/factory.py
from offline_intelligence_py import OfflineIntelligence
from .config import Config
import os

class AIFactory:
    @staticmethod
    def create_ai_instance(**kwargs) > OfflineIntelligence:
        """Create AI instance with configuration"""
        # Override config with kwargs if provided
        config_dict = {
            "model_path": Config.MODEL_PATH,
            "llama_bin": Config.LLAMA_BIN,
            "ctx_size": Config.CTX_SIZE,
            "batch_size": Config.BATCH_SIZE,
            "threads": Config.THREADS,
            "gpu_layers": Config.GPU_LAYERS,
            **kwargs
        }
        
        # Set environment variables temporarily
        old_values = {}
        for key, value in config_dict.items():
            env_key = key.upper()
            old_values[env_key] = os.environ.get(env_key)
            os.environ[env_key] = str(value)
        
        try:
            return OfflineIntelligence()
        finally:
            # Restore original environment
            for key, value in old_values.items():
                if value is not None:
                    os.environ[key] = value
                else:
                    os.environ.pop(key, None)
```

## 🧪 Testing Your Setup

### Unit Tests
```python
# tests/test_ai.py
import pytest
import asyncio
from offline_intelligence_py import OfflineIntelligence, Message

@pytest.fixture
async def ai_instance():
    return OfflineIntelligence()

@pytest.mark.asyncio
async def test_basic_generation(ai_instance):
    response = await ai_instance.generate_completion(
        "Say hello in Spanish", 
        max_tokens=20
    )
    assert isinstance(response, str)
    assert len(response) > 0
    print(f"Response: {response}")

@pytest.mark.asyncio  
async def test_chat_completion(ai_instance):
    messages = [
        Message("user", "What is the capital of France?"),
    ]
    
    response = await ai_instance.generate_chat_completion(messages)
    assert isinstance(response, str)
    assert len(response) > 0
    assert "paris" in response.lower()
    print(f"Chat response: {response}")

@pytest.mark.asyncio
async def test_context_optimization(ai_instance):
    # Create long conversation
    messages = []
    for i in range(15):
        messages.append(Message("user", f"Message {i}"))
        messages.append(Message("assistant", f"Response {i}"))
    
    result = await ai_instance.optimize_context(
        "test_session", 
        messages, 
        "Latest message"
    )
    
    assert "optimized_messages" in result
    assert len(result["optimized_messages"]) <= len(messages)
    print(f"Optimized from {len(messages)} to {len(result['optimized_messages'])} messages")
```

### Integration Test Script
```python
# tests/integration_test.py
import asyncio
import sys
import os

# Add src to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

from config import Config
from ai_handler import ChatHandler

async def run_integration_test():
    """Complete integration test"""
    print("🧪 Running integration test...")
    
    try:
        # Test configuration
        Config.validate()
        print("✅ Configuration validated")
        
        # Test AI handler
        handler = ChatHandler()
        
        # Test conversation
        session_id = "integration_test"
        test_messages = [
            "Hello AI!",
            "What can you do?",
            "Tell me a fun fact"
        ]
        
        print("\n🗣️  Testing conversation:")
        for i, message in enumerate(test_messages, 1):
            print(f"\n{i}. User: {message}")
            response = await handler.chat(session_id, message)
            print(f"   AI: {response[:100]}...")  # Truncate long responses
            
        print("\n✅ Integration test completed successfully!")
        return True
        
    except Exception as e:
        print(f"\n❌ Integration test failed: {e}")
        return False

if __name__ == "__main__":
    success = asyncio.run(run_integration_test())
    sys.exit(0 if success else 1)
```

## 🚀 Performance Optimization

### Concurrency Management
```python
# src/performance.py
import asyncio
import time
from typing import List
from offline_intelligence_py import OfflineIntelligence

class PerformanceOptimizer:
    def __init__(self, max_concurrent: int = 5):
        self.semaphore = asyncio.Semaphore(max_concurrent)
        self.ai = OfflineIntelligence()
    
    async def process_with_limit(self, prompt: str) > str:
        """Process with concurrency limiting"""
        async with self.semaphore:
            return await self.ai.generate_completion(prompt, max_tokens=100)
    
    async def batch_process(self, prompts: List[str]) > List[str]:
        """Efficiently process multiple prompts"""
        start_time = time.time()
        
        tasks = [self.process_with_limit(prompt) for prompt in prompts]
        results = await asyncio.gather(*tasks)
        
        elapsed = time.time()  start_time
        print(f"Processed {len(prompts)} prompts in {elapsed:.2f}s")
        
        return results

# Usage example
async def performance_demo():
    optimizer = PerformanceOptimizer(max_concurrent=3)
    
    prompts = [f"Fact {i}" for i in range(10)]
    results = await optimizer.batch_process(prompts)
    
    for prompt, result in zip(prompts, results):
        print(f"{prompt}: {result[:50]}...")
```

### Memory Management
```python
# src/memory_manager.py
import gc
from offline_intelligence_py import OfflineIntelligence

class MemoryManagedAI:
    def __init__(self):
        self.ai = OfflineIntelligence()
        self.request_count = 0
        self.cleanup_threshold = 100
    
    async def generate_with_cleanup(self, prompt: str, **kwargs) > str:
        """Generate response with periodic cleanup"""
        self.request_count += 1
        
        if self.request_count % self.cleanup_threshold == 0:
            gc.collect()  # Force garbage collection
            print(f"🧹 Memory cleanup performed after {self.request_count} requests")
        
        return await self.ai.generate_completion(prompt, **kwargs)
```

## 🛠 Development Workflow

### Development Setup Script
```bash
#!/bin/bash
# setup_dev.sh

echo "🐍 Setting up Python development environment"

# Create virtual environment
python m venv venv
source venv/bin/activate

# Install dependencies
pip install r requirements.txt
pip install pytest pytestasyncio black flake8

# Download test model if needed
if [ ! f "./models/testmodel.gguf" ]; then
    echo "📥 Downloading test model..."
    mkdir p models
    # Add model download command
fi

# Run tests
echo "🧪 Running tests..."
python m pytest tests/ v

echo "✅ Python development environment ready!"
```

### Example Application
```python
# examples/chat_cli.py
import asyncio
import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from ai_handler import ChatHandler

async def chat_interface():
    """Interactive chat interface"""
    print("🐍 Offline Intelligence Chat Interface")
    print("Type 'quit' to exit\n")
    
    handler = ChatHandler()
    session_id = "cli_user"
    
    while True:
        try:
            user_input = input("You: ").strip()
            
            if user_input.lower() in ['quit', 'exit', 'bye']:
                print("👋 Goodbye!")
                break
                
            if not user_input:
                continue
                
            print("🤖 Thinking...")
            response = await handler.chat(session_id, user_input)
            print(f"AI: {response}\n")
            
        except KeyboardInterrupt:
            print("\n👋 Goodbye!")
            break
        except Exception as e:
            print(f"❌ Error: {e}")

if __name__ == "__main__":
    asyncio.run(chat_interface())
```

Run the example:
```bash
python examples/chat_cli.py
```

## 🐛 Troubleshooting

### Common Import Issues

**Module Not Found:**
```bash
# Reinstall with proper dependencies
pip uninstall offlineintelligence
pip install offlineintelligence nocachedir
```

**Import Errors on Windows:**
```python
# Add to top of your script if needed
import os
os.add_dll_directory(r"C:\path\to\rust\libs")
```

### Runtime Issues

**Model Loading Failures:**
```python
# Add detailed error handling
try:
    oi = OfflineIntelligence()
    print("✅ AI library loaded successfully")
except Exception as e:
    print(f"❌ Failed to load AI library: {e}")
    print("Check that MODEL_PATH and LLAMA_BIN are correctly set")
    sys.exit(1)
```

**Performance Problems:**
```python
import time
import psutil

def monitor_performance():
    """Monitor system resources during AI operations"""
    process = psutil.Process()
    
    def wrapper(func):
        async def wrapped(*args, **kwargs):
            start_time = time.time()
            start_memory = process.memory_info().rss / 1024 / 1024  # MB
            
            result = await func(*args, **kwargs)
            
            end_time = time.time()
            end_memory = process.memory_info().rss / 1024 / 1024  # MB
            
            print(f"⏱️  Time: {end_time  start_time:.2f}s")
            print(f"💾 Memory: {end_memory  start_memory:.1f}MB")
            
            return result
        return wrapped
    return wrapper
```

## 📚 Next Steps

1. **Explore Examples**: Check the `examples/` directory
2. **Advanced Configuration**: Read [CONFIGURATION_GUIDE.md](CONFIGURATION_GUIDE.md)  
3. **API Reference**: Visit the [documentation](https://pypi.org/project/offlineintelligence)
4. **Community**: Join discussions on [GitHub Issues](https://github.com/OfflineIntelligence/offlineintelligence/issues)



**Ready to build?** Start with the example application and customize it for your needs!
