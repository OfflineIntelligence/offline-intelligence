# Model Installation Guide

## 🤖 AI Model Setup

This guide covers how to download, prepare, and configure AI models for use with the Offline Intelligence Library.

## 📚 Understanding Model Types

### Model Formats Supported
 **GGUF** (Recommended)  Modern, efficient format
 **GGML**  Legacy format, still widely used
 **ONNX**  Crossplatform neural network format
 **TensorRT**  NVIDIA GPU optimized
 **Safetensors**  Safe tensor format

### Quantization Levels
Different quantization levels offer tradeoffs between:
 **Size**: Storage requirements
 **Speed**: Inference performance  
 **Quality**: Output quality

| Quantization | Size | Speed | Quality | Use Case |
||||||
| Q2_K | Smallest | Fastest | Lowest | Testing only |
| Q4_0 | Small | Fast | Good | Mobile/constrained |
| Q4_K_M | Balanced | Balanced | Very Good | Recommended |
| Q5_K_M | Large | Slower | Excellent | High quality |
| Q6_K | Largest | Slowest | Best | Maximum quality |

## Downloading Models

### Method 1: Hugging Face (Recommended Examples)

**Install huggingfacecli:**
```bash
pip install huggingface_hub
```

**Download models:**
```bash
# Llama 2 7B (recommended for beginners)
huggingfacecli download TheBloke/Llama27BGGUF llama27b.Q4_K_M.gguf

# Llama 2 13B (better quality)
huggingfacecli download TheBloke/Llama213BGGUF llama213b.Q4_K_M.gguf

# Mistral 7B (alternative)
huggingfacecli download TheBloke/Mistral7Bv0.1GGUF mistral7bv0.1.Q4_K_M.gguf
```

### Method 2: Direct Download

Visit model repositories:
 **Llama 2**: https://huggingface.co/TheBloke/Llama27BGGUF
 **Mistral**: https://huggingface.co/TheBloke/Mistral7Bv0.1GGUF
 **WizardLM**: https://huggingface.co/TheBloke/WizardLM7BV1.0UncensoredGGUF

Download the `.gguf` file directly.

### Method 3: Model Conversion

Convert existing models to GGUF format:

```bash
# Install llama.cpp
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
make

# Convert model
python convert.py yourmodel.bin \
  outfile models/convertedmodel.gguf \
  outtype q4_K_M
```

## Organizing Your Models

### Directory Structure Examples
```
project/
├── models/
│   ├── llama27b/
│   │   ├── Q4_K_M.gguf     # Main model
│   │   ├── Q5_K_M.gguf     # Higher quality version
│   │   └── small.gguf      # Quantized for testing
│   ├── mistral7b/
│   │   └── Q4_K_M.gguf
│   └── config/
│       └── modelconfig.json
```

### Model Selection Guide

**For Development/Testing:**
```bash
# Small, fast model for quick iteration
models/llama27b/Q4_K_M.gguf  # ~4GB
CTX_SIZE=2048
```

**For Production:**
```bash
# Larger, higher quality model
models/llama213b/Q5_K_M.gguf  # ~8GB  
CTX_SIZE=8192
```

**For Resource-Constrained Environments:**
```bash
# Smallest viable model
models/llama27b/small.gguf    # ~2GB
CTX_SIZE=1024
```

## Model Configuration

### Configuration File Examples

Create `modelconfig.json`:
```json
{
  "default_model": "llama27b/Q4_K_M.gguf",
  "models": {
    "development": {
      "path": "llama27b/small.gguf",
      "ctx_size": 2048,
      "batch_size": 128
    },
    "production": {
      "path": "llama213b/Q5_K_M.gguf", 
      "ctx_size": 8192,
      "batch_size": 512
    }
  }
}
```

### Loading Different Models

```python
import json
import os

# Load configuration
with open('models/config/modelconfig.json') as f:
    config = json.load(f)

# Set environment based on mode
mode = os.getenv('APP_MODE', 'development')
model_config = config['models'][mode]

os.environ['MODEL_PATH'] = f"./models/{model_config['path']}"
os.environ['CTX_SIZE'] = str(model_config['ctx_size'])
os.environ['BATCH_SIZE'] = str(model_config['batch_size'])
```

## Model Validation

### Validation Script

Create `test_models.py`:
```python
import os
from pathlib import Path
from offline_intelligence_py import OfflineIntelligence

def test_model(model_path):
    """Test if a model file works correctly"""
    print(f"Testing model: {model_path}")
    
    # Set environment
    os.environ['MODEL_PATH'] = model_path
    
    try:
        # Initialize library
        oi = OfflineIntelligence()
        
        # Simple generation test
        prompt = "The capital of France is"
        result = oi.generate_completion(prompt, max_tokens=10)
        
        print(f"✅ Success! Generated: {result}")
        return True
        
    except Exception as e:
        print(f"❌ Failed: {e}")
        return False

# Test all models
models_dir = Path("./models")
for model_file in models_dir.rglob("*.gguf"):
    test_model(str(model_file))
    print("" * 50)
```

## Performance Optimization

### Loading Strategies

**Lazy Loading:**
```python
class ModelManager:
    def __init__(self):
        self.current_model = None
        self.oi = None
    
    def load_model(self, model_path):
        if self.current_model != model_path:
            os.environ['MODEL_PATH'] = model_path
            self.oi = OfflineIntelligence()
            self.current_model = model_path
            print(f"Loaded model: {model_path}")
    
    def generate(self, prompt):
        return self.oi.generate_completion(prompt)
```

**Model Pooling:**
```python
# Keep multiple models ready for different tasks
model_pool = {
    'fast': './models/small.gguf',
    'quality': './models/large.gguf',
    'chat': './models/chat.gguf'
}

# Switch based on requirements
def get_best_model(task_type):
    return model_pool.get(task_type, model_pool['fast'])
```

## 🧪 Benchmarking Models

### Performance Comparison Script

```python
import time
import os
from offline_intelligence_py import OfflineIntelligence

def benchmark_model(model_path, iterations=5):
    """Benchmark model performance"""
    
    os.environ['MODEL_PATH'] = model_path
    oi = OfflineIntelligence()
    
    prompt = "Explain quantum computing in simple terms:"
    times = []
    
    for i in range(iterations):
        start_time = time.time()
        result = oi.generate_completion(prompt, max_tokens=100)
        end_time = time.time()
        
        duration = end_time  start_time
        times.append(duration)
        print(f"Iteration {i+1}: {duration:.2f}s")
    
    avg_time = sum(times) / len(times)
    print(f"\nAverage time: {avg_time:.2f}s")
    print(f"Tokens/second: {100/avg_time:.1f}")
    
    return avg_time

# Compare models
models = [
    './models/small.gguf',
    './models/medium.gguf', 
    './models/large.gguf'
]

for model in models:
    print(f"\nBenchmarking: {model}")
    benchmark_model(model)
```

## 🛠 Troubleshooting

### Common Issues

**Model Won't Load:**
```bash
# Check file integrity
sha256sum model.gguf

# Verify format
file model.gguf  # Should show it's a data file
```

**Memory Issues:**
```bash
# Reduce context size
export CTX_SIZE=1024

# Use smaller model
# Download Q2_K or Q3_K quantization
```

**Slow Performance:**
```bash
# Enable GPU acceleration
export GPU_LAYERS=20

# Increase batch size
export BATCH_SIZE=256
```

## 🔐 Model Management Best Practices

### Version Control
```bash
# Don't commit models to git (add to .gitignore)
echo "models/" >> .gitignore
echo "*.gguf" >> .gitignore

# Track model versions separately
echo "models/versions.txt" >> .gitkeep
```

### Backup Strategy
```bash
# Create backup script
#!/bin/bash
rsync av models/ /backup/models/
```

### Model Documentation
Create `models/README.md`:
```markdown
# Model Inventory

## Current Models

### Llama 2 7B Q4_K_M
 Size: 4.1GB
 Purpose: General development
 Performance: Fast, good quality
 Last updated: 20240130

### Llama 2 13B Q5_K_M  
 Size: 8.2GB
 Purpose: Production use
 Performance: Excellent quality
 Last updated: 20240130
```



**Next**: Learn about advanced configuration in [CONFIGURATION_GUIDE.md](CONFIGURATION_GUIDE.md)
