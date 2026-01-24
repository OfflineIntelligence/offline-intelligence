# Offline Intelligence C++ Bindings

C++ bindings for the Offline Intelligence Library - High-performance LLM inference engine with memory management capabilities.

## Installation

### Header-Only Installation
Simply copy the header file to your project:

```bash
# Clone the repository
git clone https://github.com/OfflineIntelligence/offline-intelligence.git

# Copy the header to your include path
cp offline-intelligence/bindings/cpp/include/offline_intelligence/offline_intelligence.hpp /path/to/your/project/include/
```

### CMake Integration
Add to your CMakeLists.txt:

```cmake
# Add the include directory
include_directories(/path/to/offline-intelligence/bindings/cpp/include)

# Link against the library (if building from source)
target_link_libraries(your_target offline_intelligence_cpp)
```

## Usage

```cpp
#include <offline_intelligence/offline_intelligence.hpp>
#include <iostream>

int main() {
    try {
        // Create configuration
        auto config = offline_intelligence::Config::from_env();
        
        // Customize configuration if needed
        config.api_host = "0.0.0.0";
        config.api_port = 8080;
        
        // Start the server
        bool success = offline_intelligence::Server::run_server(config);
        
        if (success) {
            std::cout << "Server started successfully!" << std::endl;
            std::cout << "Version: " << offline_intelligence::Server::version() << std::endl;
        } else {
            std::cerr << "Failed to start server" << std::endl;
        }
        
    } catch (const offline_intelligence::OfflineIntelligenceException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

## Features

- **Header-Only**: Easy integration with no linking required
- **Modern C++17**: Uses modern C++ features and best practices
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Exception Safety**: Proper exception handling with custom exception types
- **Configuration Management**: Flexible configuration system

## API Reference

### Config Structure
Holds all configuration parameters for the Offline Intelligence engine.

### Server Class
Main interface for starting and managing the intelligence engine.

### OfflineIntelligenceException
Custom exception type for error handling.

## Requirements

- C++17 or later
- CMake 3.16+ (for building from source)
- Standard C++ library

## Platform Support

- **Windows**: x64
- **macOS**: Intel and Apple Silicon
- **Linux**: x64 and ARM64

## Building from Source

```bash
cd offline-intelligence/bindings/cpp
mkdir build && cd build
cmake ..
make
```

## License

Apache 2.0 License - Same as the core Offline Intelligence library.