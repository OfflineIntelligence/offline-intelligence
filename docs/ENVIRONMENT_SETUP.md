# Environment Setup Guide

Comprehensive guide for setting up your development environment to work with the Offline Intelligence Library.

## System Requirements

### Operating Systems
- **Windows**: Windows 10/11 (64-bit)
- **macOS**: macOS 10.15 Catalina or later
- **Linux**: Ubuntu 18.04+, CentOS 7+, or equivalent

### Core Dependencies

#### Rust Toolchain
```bash
# Install rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Or install specific version
rustup install 1.70
rustup default 1.70
```

#### Build Tools
**Windows:**
```bash
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/
# Select "C++ build tools" workload
```

**macOS:**
```bash
# Install Xcode command line tools
xcode-select --install

# Or install full Xcode from App Store
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential cmake pkg-config

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install cmake
```

## Language-Specific Setup

### Python Environment
```bash
# Install Python 3.8+
python --version  # Should be 3.8 or higher

# Install pip (if not present)
python -m ensurepip --upgrade

# Virtual environment (recommended)
python -m venv offline-ai-env
source offline-ai-env/bin/activate  # Linux/macOS
# offline-ai-env\Scripts\activate  # Windows
```

### Java Environment
```bash
# Install JDK 11 or higher
java -version  # Should show OpenJDK 11+

# Set JAVA_HOME environment variable
export JAVA_HOME=/path/to/jdk  # Linux/macOS
# set JAVA_HOME=C:\Program Files\Java\jdk-11  # Windows
```

### Node.js Environment
```bash
# Install Node.js 16+ and npm 7+
node --version  # Should be 16.0.0 or higher
npm --version   # Should be 7.0.0 or higher

# Optional: Use nvm for version management
# Linux/macOS:
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Windows:
# Download nvm-windows from https://github.com/coreybutler/nvm-windows
```

### C++ Environment
```bash
# GCC (Linux)
sudo apt install gcc g++  # Ubuntu/Debian
sudo yum install gcc gcc-c++  # CentOS/RHEL

# Clang (alternative)
sudo apt install clang  # Ubuntu/Debian

# Windows: Visual Studio with C++ support
# Download Visual Studio Community with C++ workload
```

## Library Installation

### Core Library
```bash
# Clone the repository
git clone https://github.com/offline-intelligence/library.git
cd library

# Build core library
cargo build --release

# Run tests to verify installation
cargo test
```

### Language Bindings

#### Python Bindings
```bash
cd crates/python-bindings
pip install -e .
# Or for production:
pip install offline-intelligence-py
```

#### Java Bindings
```bash
cd crates/java-bindings
./gradlew build
# Or add Maven dependency to your project
```

#### JavaScript Bindings
```bash
cd crates/js-bindings
npm install
npm run build
# Or:
npm install offline-intelligence-js
```

#### C++ Bindings
```bash
cd crates/cpp-bindings
mkdir build && cd build
cmake ..
make
sudo make install  # Optional: system-wide installation
```

## Model Setup

### Download Models
```bash
# Create models directory
mkdir -p data/models

# Download example models (sizes vary)
# GGUF format models
wget -O data/models/llama-2-7b.Q4_K_M.gguf \
  "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf"

# ONNX format models
wget -O data/models/gpt2.onnx \
  "https://github.com/onnx/models/raw/main/text/machine_comprehension/gpt-2/model/gpt2-lm-head-10.tar.gz"
```

### Model Configuration
```bash
# Set model path in environment
export OFFLINE_AI_MODEL_PATH=./data/models/llama-2-7b.Q4_K_M.gguf

# Or configure in code
Config config = Config.builder()
    .modelPath("./data/models/custom-model.gguf")
    .build();
```

## Verification Steps

### Test Core Functionality
```bash
# Run core library tests
cd crates/offline-intelligence
cargo test --release

# Expected output: All tests should pass
```

### Test Language Bindings

#### Python
```python
import offline_intelligence
print("Python bindings working correctly")
```

#### Java
```java
import com.offlineintelligence.*;
public class Test { 
    public static void main(String[] args) {
        System.out.println("Java bindings working correctly");
    }
}
```

#### JavaScript
```javascript
const { MemoryStore } = require('offline-intelligence-js');
console.log('JavaScript bindings working correctly');
```

#### C++
```cpp
#include "offline_intelligence.h"
#include <iostream>
int main() {
    std::cout << "C++ bindings working correctly" << std::endl;
    return 0;
}
```

## Troubleshooting Common Issues

### Compilation Errors
```bash
# Clear build cache
cargo clean
# Rebuild
cargo build --release
```

### Missing Dependencies
```bash
# Update package managers
sudo apt update && sudo apt upgrade  # Linux
brew update && brew upgrade  # macOS

# Install missing packages
sudo apt install libssl-dev pkg-config  # Linux
brew install openssl  # macOS
```

### Permission Issues
```bash
# Fix cargo permissions
sudo chown -R $(whoami) ~/.cargo

# Fix npm permissions
sudo chown -R $(whoami) ~/.npm
```

### Path Issues
```bash
# Add cargo to PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Add Python scripts to PATH
export PATH="$PATH:$(python -m site --user-base)/bin"
```

## Performance Tuning

### System Configuration
```bash
# Increase file descriptor limits (Linux)
ulimit -n 65536

# Optimize CPU governor (Linux)
sudo cpupower frequency-set -g performance

# Enable huge pages (Linux, for better performance)
echo madvise | sudo tee /sys/kernel/mm/transparent_hugepage/enabled
```

### Library Configuration
```python
# Python example
config = Config(
    max_memory_mb=2048,  # Adjust based on available RAM
    num_threads=8,       # Match CPU cores
    cache_size=5000      # Increase for better performance
)
```

## Development Tools

### Recommended IDE Extensions
- **VS Code**: Rust Analyzer, Python Extension, Java Extension Pack
- **IntelliJ IDEA**: Rust plugin, Python plugin
- **Vim/Neovim**: rust.vim, vim-python, vim-javascript

### Debugging Tools
```bash
# Install debugging tools
cargo install cargo-valgrind  # Memory debugging
pip install pytest  # Python testing
npm install --save-dev jest  # JavaScript testing
```

## Next Steps

After successful setup:
1. Review the [Getting Started Guide](./GETTING_STARTED.md)
2. Explore language-specific documentation
3. Try the example applications
4. Join the community for support and updates

## Support

If you encounter issues during setup:
1. Check the troubleshooting section above
2. Review system requirements match your environment
3. Ensure all dependencies are properly installed
4. Open an issue on GitHub with detailed error information