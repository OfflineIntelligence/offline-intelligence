# Environment Setup Guide

## 🌍 Environment Configuration

Proper environment setup is crucial for optimal performance and functionality of the Offline Intelligence Library.

## 📁 Directory Structure

Recommended project structure:
```
your-project/
├── src/                 # Your source code
├── models/              # AI model files
│   └── llama-2-7b.gguf
├── binaries/            # LLaMA binaries
│   └── llama-bin.exe
├── config/              # Configuration files
│   └── .env
└── examples/            # Test examples
```

## ⚙️ Environment Variables

### Required Variables

```bash
# Model file path (absolute or relative)
MODEL_PATH="./models/llama-2-7b.Q4_K_M.gguf"

# LLaMA binary path
LLAMA_BIN="./binaries/llama-bin"

# Context size (tokens)
CTX_SIZE="8192"
```

### Optional Variables

```bash
# Performance tuning
BATCH_SIZE="256"        # Processing batch size
THREADS="6"             # CPU threads to use
GPU_LAYERS="20"         # GPU acceleration layers

# Memory management
MAIN_GPU="0"            # Which GPU to use (0-indexed)
TENSOR_SPLIT="1"        # Tensor splitting for multi-GPU

# Advanced options
FREQ_PENALTY="0.5"      # Frequency penalty
PRESENCE_PENALTY="0.5"  # Presence penalty
TEMPERATURE="0.8"       # Sampling temperature
TOP_P="0.9"             # Top-p sampling
TOP_K="40"              # Top-k sampling
```

## 🖥 Platform-Specific Setup

### Windows

**Using Command Prompt:**
```cmd
set MODEL_PATH=C:\path\to\model.gguf
set LLAMA_BIN=C:\path\to\llama-bin.exe
```

**Using PowerShell:**
```powershell
$env:MODEL_PATH = "C:\path\to\model.gguf"
$env:LLAMA_BIN = "C:\path\to\llama-bin.exe"
```

**Using .env file:**
Create a `.env` file in your project root:
```env
MODEL_PATH=./models/model.gguf
LLAMA_BIN=./binaries/llama-bin.exe
CTX_SIZE=8192
```

### macOS/Linux

**Using bash/zsh:**
```bash
export MODEL_PATH="/path/to/model.gguf"
export LLAMA_BIN="/path/to/llama-bin"
```

**Using .env file:**
```bash
# Add to ~/.bashrc or ~/.zshrc
export MODEL_PATH="/path/to/model.gguf"
export LLAMA_BIN="/path/to/llama-bin"
```

## 🎯 Configuration Profiles

### Development Profile
```env
# Optimized for development speed
MODEL_PATH=./models/small-model.gguf
CTX_SIZE=2048
BATCH_SIZE=128
THREADS=4
GPU_LAYERS=0  # CPU only for consistency
```

### Production Profile  
```env
# Optimized for performance
MODEL_PATH=./models/large-model.gguf
CTX_SIZE=8192
BATCH_SIZE=512
THREADS=12
GPU_LAYERS=40  # Full GPU acceleration
```

### Memory-Constrained Profile
```env
# For systems with limited RAM
MODEL_PATH=./models/quantized-model.gguf
CTX_SIZE=1024
BATCH_SIZE=64
THREADS=2
GPU_LAYERS=10
```

## 🔍 Validation Script

Create a validation script to check your environment:

**validate_env.py:**
```python
import os
from pathlib import Path

required_vars = ['MODEL_PATH', 'LLAMA_BIN']
optional_vars = ['CTX_SIZE', 'BATCH_SIZE', 'THREADS', 'GPU_LAYERS']

print("🔍 Validating environment setup...")
print("=" * 40)

# Check required variables
for var in required_vars:
    value = os.getenv(var)
    if value:
        path = Path(value)
        if path.exists():
            print(f"✅ {var}: {value} (found)")
        else:
            print(f"❌ {var}: {value} (file not found)")
    else:
        print(f"❌ {var}: Not set")

# Check optional variables
print("\nOptional settings:")
for var in optional_vars:
    value = os.getenv(var, 'Not set')
    print(f"   {var}: {value}")

print("\n💡 Tip: Set these variables in your .env file or system environment")
```

## 🚨 Common Issues

### File Not Found Errors
```bash
# Wrong path
❌ MODEL_PATH=./wrong/path/model.gguf

# Correct path
✅ MODEL_PATH=./models/llama-2-7b.gguf
```

### Permission Denied
```bash
# Make binary executable (Linux/macOS)
chmod +x ./binaries/llama-bin

# Run as administrator (Windows)
# Right-click Command Prompt → "Run as administrator"
```

### Path Separators
```bash
# Windows: Use forward slashes or escaped backslashes
✅ MODEL_PATH=C:/models/model.gguf
✅ MODEL_PATH=C:\\models\\model.gguf

# macOS/Linux: Use forward slashes
✅ MODEL_PATH=/home/user/models/model.gguf
```

## 🧪 Testing Your Setup

Run this test to verify everything works:

```python
import os
from offline_intelligence_py import OfflineIntelligence

# Load configuration from environment
model_path = os.getenv('MODEL_PATH')
llama_bin = os.getenv('LLAMA_BIN')

if not model_path or not llama_bin:
    raise ValueError("Please set MODEL_PATH and LLAMA_BIN environment variables")

print(f"Testing with model: {model_path}")
print(f"Using binary: {llama_bin}")

# Initialize library
oi = OfflineIntelligence()

# Simple test
result = oi.generate_title([{"role": "user", "content": "Hello"}])
print(f"✅ Library working! Generated title: {result}")
```

## 📊 Performance Tuning

### CPU Optimization
```env
THREADS=8           # Match your CPU cores
BATCH_SIZE=256      # Balance between speed and memory
CTX_SIZE=4096       # Adjust based on available RAM
```

### GPU Optimization  
```env
GPU_LAYERS=40       # Max layers your GPU can handle
MAIN_GPU=0          # Primary GPU index
TENSOR_SPLIT=1,0    # Distribution across multiple GPUs
```

### Memory Management
```env
# For 16GB RAM system
CTX_SIZE=4096
BATCH_SIZE=128
GPU_LAYERS=20

# For 32GB+ RAM system  
CTX_SIZE=8192
BATCH_SIZE=512
GPU_LAYERS=40
```

---

**Next**: Configure your specific use case in [CONFIGURATION_GUIDE.md](CONFIGURATION_GUIDE.md)