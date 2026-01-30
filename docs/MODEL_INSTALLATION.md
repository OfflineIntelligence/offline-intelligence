# Model Installation and Management

Guide for downloading, installing, and managing AI models for the Offline Intelligence Library.

## Supported Model Formats

The library supports multiple model formats for flexibility and performance:

### Primary Formats
- **GGUF** (GPT-Generated Unified Format) - Recommended for best performance
- **ONNX** (Open Neural Network Exchange) - Cross-platform standard
- **Safetensors** - Safe tensor serialization format
- **GGML** - Legacy format with wide model availability

### Platform-Specific Formats
- **CoreML** - Apple platforms (iOS, macOS)
- **TensorRT** - NVIDIA GPUs (Linux)
- **OpenVINO** - Intel hardware acceleration

## Finding Models

### Recommended Sources

#### Hugging Face Hub
```bash
# Browse models at: https://huggingface.co/models
# Filter by: "gguf", "quantized", "offline"
```

Popular model families:
- LLaMA series (Meta)
- Mistral models
- Phi series (Microsoft)
- Gemma models (Google)

#### Direct Downloads
```bash
# Example: Download quantized LLaMA 2 model
wget https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf

# Example: Download Mistral model
wget https://huggingface.co/TheBloke/Mistral-7B-v0.1-GGUF/resolve/main/mistral-7b.Q4_K_M.gguf
```

### Model Selection Criteria

#### Performance vs Size Trade-offs
| Quantization | Size | Performance | VRAM | Quality |
|--------------|------|-------------|------|---------|
| Q2_K | ~3GB | Fastest | 4GB | Lower |
| Q4_K_M | ~4.5GB | Balanced | 6GB | Good |
| Q5_K_M | ~5.5GB | Good | 8GB | Better |
| Q8_0 | ~7GB | Slower | 10GB | Best |

#### Recommendations by Use Case
- **General purpose**: Q4_K_M models
- **High quality required**: Q5_K_M or Q8_0
- **Resource constrained**: Q2_K or Q3_K
- **Mobile/embedded**: Q2_K or GGML small models

## Downloading Models

### Automated Download Script
```python
import requests
import os
from pathlib import Path

def download_model(model_url, destination_path):
    """Download model with progress indication"""
    destination = Path(destination_path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    
    print(f"Downloading {model_url} to {destination}")
    
    response = requests.get(model_url, stream=True)
    response.raise_for_status()
    
    total_size = int(response.headers.get('content-length', 0))
    downloaded = 0
    
    with open(destination, 'wb') as f:
        for chunk in response.iter_content(chunk_size=8192):
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total_size > 0:
                    percent = (downloaded / total_size) * 100
                    print(f"\rProgress: {percent:.1f}%", end='')
    
    print(f"\nDownload complete: {destination}")

# Example usage
MODEL_URL = "https://huggingface.co/TheBloke/Mistral-7B-v0.1-GGUF/resolve/main/mistral-7b.Q4_K_M.gguf"
DEST_PATH = "./models/mistral-7b.Q4_K_M.gguf"

download_model(MODEL_URL, DEST_PATH)
```

### Batch Download for Multiple Models
```bash
#!/bin/bash
# download_models.sh

MODELS_DIR="./models"
mkdir -p "$MODELS_DIR"

declare -A MODELS=(
    ["mistral-7b"]="https://huggingface.co/TheBloke/Mistral-7B-v0.1-GGUF/resolve/main/mistral-7b.Q4_K_M.gguf"
    ["llama-2-7b"]="https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf"
    ["phi-2"]="https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf"
)

for model_name in "${!MODELS[@]}"; do
    url="${MODELS[$model_name]}"
    filename="$MODELS_DIR/${model_name}.gguf"
    
    if [[ ! -f "$filename" ]]; then
        echo "Downloading $model_name..."
        wget -O "$filename" "$url"
    else
        echo "$model_name already exists, skipping..."
    fi
done

echo "All models downloaded to $MODELS_DIR"
```

## Model Organization

### Recommended Directory Structure
```
models/
├── gguf/
│   ├── mistral-7b.Q4_K_M.gguf
│   ├── llama-2-7b.Q4_K_M.gguf
│   └── phi-2.Q4_K_M.gguf
├── onnx/
│   ├── gpt2.onnx
│   └── bert-base.onnx
├── cache/
│   └── [temporary files]
└── configs/
    ├── default.yaml
    └── high-performance.yaml
```

### Model Metadata Management
```python
import yaml
from pathlib import Path

class ModelManager:
    def __init__(self, models_dir="./models"):
        self.models_dir = Path(models_dir)
        self.metadata_file = self.models_dir / "models.yaml"
        self.load_metadata()
    
    def load_metadata(self):
        if self.metadata_file.exists():
            with open(self.metadata_file, 'r') as f:
                self.models = yaml.safe_load(f) or {}
        else:
            self.models = {}
    
    def add_model(self, name, path, format, quantization, description=""):
        model_info = {
            'path': str(path),
            'format': format,
            'quantization': quantization,
            'size_bytes': Path(path).stat().st_size,
            'description': description,
            'added_date': datetime.now().isoformat()
        }
        self.models[name] = model_info
        self.save_metadata()
    
    def save_metadata(self):
        with open(self.metadata_file, 'w') as f:
            yaml.dump(self.models, f)
    
    def get_model(self, name):
        return self.models.get(name)

# Usage
manager = ModelManager()
manager.add_model(
    name="mistral-7b-q4",
    path="./models/gguf/mistral-7b.Q4_K_M.gguf",
    format="gguf",
    quantization="Q4_K_M",
    description="Balanced performance model for general use"
)
```

## Configuration and Usage

### Setting Model Paths
```python
from offline_intelligence import Config, MemoryStore

# Method 1: Environment variable
import os
os.environ['OFFLINE_AI_MODEL_PATH'] = './models/mistral-7b.Q4_K_M.gguf'

# Method 2: Direct configuration
config = Config(
    model_path='./models/mistral-7b.Q4_K_M.gguf',
    model_format='gguf'
)
memory_store = MemoryStore(config)

# Method 3: Model selection by name
config = Config(model_name='mistral-7b-q4')  # Uses model manager
```

### Multiple Model Support
```python
class MultiModelAI:
    def __init__(self):
        self.models = {
            'fast': MemoryStore(Config(model_name='phi-2-q4')),
            'balanced': MemoryStore(Config(model_name='mistral-7b-q4')),
            'high_quality': MemoryStore(Config(model_name='llama-2-7b-q5'))
        }
    
    async def process_request(self, query, quality='balanced'):
        model = self.models[quality]
        return await model.generate_response([
            Message(role='user', content=query)
        ])

# Usage
ai = MultiModelAI()
response = await ai.process_request("Complex question", quality='high_quality')
```

## Model Conversion and Optimization

### Converting ONNX to GGUF
```python
# Using llama.cpp tools
import subprocess

def convert_onnx_to_gguf(onnx_path, output_path, quantization='Q4_K_M'):
    cmd = [
        'python', 'convert-hf-to-gguf.py',
        '--outfile', output_path,
        '--outtype', quantization,
        onnx_path
    ]
    subprocess.run(cmd, check=True)

# Example usage
convert_onnx_to_gguf(
    'models/onnx/gpt2.onnx',
    'models/gguf/gpt2-Q4_K_M.gguf',
    'Q4_K_M'
)
```

### Model Quantization
```python
def quantize_model(input_path, output_path, quantization_level='Q4_K_M'):
    """Quantize model to reduce size and improve inference speed"""
    cmd = [
        'llama-quantize',
        input_path,
        output_path,
        quantization_level
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        raise RuntimeError(f"Quantization failed: {result.stderr}")
    
    return output_path

# Quantize large model for deployment
quantize_model(
    'models/gguf/llama-2-13b-f16.gguf',
    'models/gguf/llama-2-13b-Q4_K_M.gguf',
    'Q4_K_M'
)
```

## Performance Benchmarking

### Model Comparison Script
```python
import time
import asyncio
from offline_intelligence import MemoryStore, Config

async def benchmark_model(model_path, iterations=10):
    config = Config(model_path=model_path)
    memory_store = MemoryStore(config)
    
    test_prompt = "Explain quantum computing in simple terms."
    times = []
    
    for i in range(iterations):
        start_time = time.time()
        response = await memory_store.generate_response([
            Message(role='user', content=test_prompt)
        ])
        end_time = time.time()
        
        times.append(end_time - start_time)
        print(f"Iteration {i+1}: {times[-1]:.2f}s")
    
    avg_time = sum(times) / len(times)
    max_time = max(times)
    min_time = min(times)
    
    return {
        'average': avg_time,
        'min': min_time,
        'max': max_time,
        'times': times
    }

# Compare multiple models
models_to_test = [
    './models/phi-2-Q4_K_M.gguf',
    './models/mistral-7b-Q4_K_M.gguf',
    './models/llama-2-7b-Q4_K_M.gguf'
]

for model_path in models_to_test:
    print(f"\nTesting {model_path}")
    results = await benchmark_model(model_path)
    print(f"Average time: {results['average']:.2f}s")
```

## Model Updates and Maintenance

### Automatic Model Updates
```python
import hashlib
import requests

class ModelUpdater:
    def __init__(self, models_dir="./models"):
        self.models_dir = Path(models_dir)
        self.checksums_file = self.models_dir / "checksums.json"
        self.load_checksums()
    
    def load_checksums(self):
        if self.checksums_file.exists():
            with open(self.checksums_file, 'r') as f:
                self.checksums = json.load(f)
        else:
            self.checksums = {}
    
    def calculate_checksum(self, file_path):
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()
    
    async def check_for_updates(self, model_name, remote_url):
        local_path = self.models_dir / f"{model_name}.gguf"
        
        if not local_path.exists():
            return True  # Model doesn't exist locally
        
        local_checksum = self.calculate_checksum(local_path)
        remote_checksum = await self.get_remote_checksum(remote_url)
        
        return local_checksum != remote_checksum
    
    async def update_model(self, model_name, url):
        local_path = self.models_dir / f"{model_name}.gguf"
        
        print(f"Updating {model_name}...")
        await download_model(url, local_path)
        
        checksum = self.calculate_checksum(local_path)
        self.checksums[model_name] = checksum
        self.save_checksums()
        
        print(f"{model_name} updated successfully")

# Usage
updater = ModelUpdater()
needs_update = await updater.check_for_updates(
    "mistral-7b", 
    "https://huggingface.co/model/mistral-7b/latest.gguf"
)
```

## Troubleshooting

### Common Issues

#### Model Loading Failures
```python
# Verify model integrity
def verify_model(model_path):
    try:
        # Attempt to load model metadata
        config = Config(model_path=model_path, verify_only=True)
        memory_store = MemoryStore(config)
        print("Model loaded successfully")
        return True
    except Exception as e:
        print(f"Model verification failed: {e}")
        return False

# Check available system memory
import psutil
def check_system_resources(model_size_gb):
    available_memory = psutil.virtual_memory().available / (1024**3)
    if available_memory < model_size_gb * 1.5:
        print(f"Warning: Insufficient RAM. Available: {available_memory:.1f}GB, Required: {model_size_gb * 1.5:.1f}GB")
        return False
    return True
```

#### Performance Issues
```python
# Monitor model performance
class PerformanceMonitor:
    def __init__(self):
        self.metrics = []
    
    def record_inference(self, model_name, duration, tokens_generated):
        self.metrics.append({
            'timestamp': time.time(),
            'model': model_name,
            'duration': duration,
            'tokens': tokens_generated,
            'tokens_per_second': tokens_generated / duration if duration > 0 else 0
        })
    
    def get_performance_report(self):
        if not self.metrics:
            return "No metrics recorded"
        
        avg_tps = sum(m['tokens_per_second'] for m in self.metrics) / len(self.metrics)
        avg_duration = sum(m['duration'] for m in self.metrics) / len(self.metrics)
        
        return {
            'average_tokens_per_second': avg_tps,
            'average_duration': avg_duration,
            'total_inferences': len(self.metrics)
        }
```

## Security Considerations

### Model Verification
```python
def verify_model_signature(model_path, expected_signature):
    """Verify model hasn't been tampered with"""
    actual_signature = calculate_checksum(model_path)
    return actual_signature == expected_signature

# Usage
EXPECTED_SIGNATURES = {
    'mistral-7b-Q4_K_M.gguf': 'expected_hash_value_here'
}

for model_file, expected_hash in EXPECTED_SIGNATURES.items():
    model_path = Path('./models') / model_file
    if not verify_model_signature(model_path, expected_hash):
        raise SecurityError(f"Model {model_file} signature mismatch")
```

This guide provides a comprehensive foundation for working with AI models in the Offline Intelligence Library. Always ensure you have appropriate rights and licenses for any models you download and use.