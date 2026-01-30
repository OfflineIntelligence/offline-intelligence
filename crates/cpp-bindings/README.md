# Offline Intelligence C++ Bindings

C++ bindings for the Offline Intelligence Library using C FFI, providing offline AI inference capabilities with context management and memory optimization.

## Building

### Using Cargo (Rust)

```bash
# Build the native library
cargo build --release
```

### Using CMake

```bash
mkdir build
cd build
cmake ..
make
```

## Usage

### C++ API

```cpp
#include "offline_intelligence_cpp.h"
#include <iostream>
#include <vector>

using namespace offline_intelligence;

int main() {
    try {
        // Initialize the library
        OfflineIntelligence oi;
        
        // Create messages
        std::vector<Message> messages = {
            Message("user", "Hello, how are you?"),
            Message("assistant", "I'm doing well, thank you for asking!"),
            Message("user", "What can you help me with?")
        };
        
        // Optimize context
        auto result = oi.optimize_context("session123", messages, "What can you help me with?");
        std::cout << "Optimized from " << result.original_count 
                  << " to " << result.optimized_count << " messages\n";
        
        // Search memory
        auto search_result = oi.search("help me", "session123", 10);
        std::cout << "Found " << search_result.total << " results\n";
        
        // Generate title
        std::string title = oi.generate_title(messages);
        std::cout << "Generated title: " << title << "\n";
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

### C API

```c
#include "offline_intelligence.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    // Initialize the library
    OfflineIntelligenceHandle* handle = offline_intelligence_new();
    if (!handle) {
        fprintf(stderr, "Failed to create OfflineIntelligence instance\n");
        return 1;
    }
    
    // Create messages
    Message messages[3] = {
        {"user", "Hello, how are you?"},
        {"assistant", "I'm doing well, thank you for asking!"},
        {"user", "What can you help me with?"}
    };
    
    // Optimize context
    OptimizationResult result = offline_intelligence_optimize_context(
        handle, "session123", messages, 3, "What can you help me with?"
    );
    
    printf("Optimized from %d to %d messages\n", 
           result.original_count, result.optimized_count);
    
    // Search memory
    SearchResult search_result = offline_intelligence_search(
        handle, "help me", "session123", 10
    );
    
    printf("Found %d results\n", search_result.total);
    
    // Generate title
    char* title = offline_intelligence_generate_title(handle, messages, 3);
    if (title) {
        printf("Generated title: %s\n", title);
        offline_intelligence_free_string(title);
    }
    
    // Clean up
    offline_intelligence_free(handle);
    
    return 0;
}
```

## Features

- **Offline AI Inference**: Run LLMs locally without internet connection
- **Context Management**: Intelligent conversation context optimization
- **Memory Search**: Hybrid semantic and keyword search across conversations
- **Multi-format Support**: Support for GGUF, GGML, ONNX, TensorRT, and Safetensors models
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Dual API**: Both C and C++ interfaces available
- **RAII Support**: Automatic resource management in C++

## Requirements

- C++17 or higher compiler
- CMake 3.12 or higher (for CMake builds)
- Rust toolchain for building from source
- Compatible LLM model file

## Configuration

Set environment variables before running:

```bash
export LLAMA_BIN="/path/to/llama-binary"
export MODEL_PATH="/path/to/model.gguf"
export CTX_SIZE="8192"
export BATCH_SIZE="256"
export THREADS="6"
export GPU_LAYERS="20"
```

## Installation

### Using CMake

```bash
mkdir build && cd build
cmake .. -DCMAKE_INSTALL_PREFIX=/usr/local
make install
```

Then in your CMakeLists.txt:

```cmake
find_package(offline_intelligence_cpp REQUIRED)
target_link_libraries(your_target offline_intelligence_cpp)
```

### Manual Installation

Copy the header files and compiled library to your project:

```bash
cp include/*.h /usr/local/include/
cp target/release/liboffline_intelligence_cpp.* /usr/local/lib/
```

## License

Apache 2.0