# C++ Bindings for Offline Intelligence

C++ bindings for the Offline Intelligence Library, providing high-performance offline AI capabilities for C++ applications through C-compatible interfaces.

## Overview

This package enables C++ developers to integrate the Offline Intelligence Library into their applications using standard C calling conventions. The bindings provide direct access to core functionality with minimal overhead and maximum performance.

## Features

### Core Capabilities
- Pure C API with C++ wrappers
- Zero-copy data transfer where possible
- Manual memory management for maximum control
- Thread-safe operations
- Cross-platform compatibility
- Static and dynamic linking support

### C++ Specific Features
- RAII-style resource management
- C++ exception handling integration
- STL container compatibility
- Smart pointer support
- Modern C++17 features

## Installation

### CMake Integration
```cmake
# In your CMakeLists.txt
find_package(offline_intelligence_cpp REQUIRED)
target_link_libraries(your_target PRIVATE offline_intelligence_cpp::offline_intelligence_cpp)
```

### Manual Build
```bash
# Clone repository
git clone https://github.com/offline-intelligence/library.git
cd library

# Build C++ bindings
cd crates/cpp-bindings
mkdir build && cd build
cmake ..
make
```

### Package Managers

#### Conan
```bash
conan install offline-intelligence-cpp/0.1.2@
```

#### vcpkg
```bash
vcpkg install offline-intelligence-cpp
```

## System Requirements

- C++17 compatible compiler (GCC 7+, Clang 5+, MSVC 2017+)
- CMake 3.12 or higher
- Rust toolchain (for building from source)
- Compatible C++ standard library

## Basic Usage

### C Interface
```c
#include "offline_intelligence.h"

int main() {
    // Create AI instance
    OfflineIntelligenceHandle* ai = offline_intelligence_new();
    
    if (!ai) {
        fprintf(stderr, "Failed to initialize AI\n");
        return 1;
    }
    
    // Create messages
    Message messages[2] = {
        {"user", "Hello, how are you?"},
        {"assistant", "I'm doing well, thank you for asking!"}
    };
    
    // Optimize context
    OptimizationResult result = offline_intelligence_optimize_context(
        ai, 
        "session-123", 
        messages, 
        2, 
        "What did we discuss earlier?"
    );
    
    // Process results
    printf("Original: %d, Optimized: %d\n", 
           result.original_count, 
           result.optimized_count);
    
    // Cleanup
    offline_intelligence_free(ai);
    return 0;
}
```

### C++ Interface
```cpp
#include "offline_intelligence_cpp.h"
#include <iostream>
#include <vector>
#include <memory>

class AISession {
private:
    std::unique_ptr<OfflineIntelligenceHandle, decltype(&offline_intelligence_free)> ai_;
    
public:
    AISession() : ai_(offline_intelligence_new(), &offline_intelligence_free) {
        if (!ai_) {
            throw std::runtime_error("Failed to initialize AI");
        }
    }
    
    std::vector<Message> optimizeContext(const std::string& sessionId,
                                       const std::vector<Message>& messages,
                                       const std::string& query = "") {
        OptimizationResult result = offline_intelligence_optimize_context(
            ai_.get(),
            sessionId.c_str(),
            messages.data(),
            static_cast<int>(messages.size()),
            query.empty() ? nullptr : query.c_str()
        );
        
        // Convert result back to vector
        std::vector<Message> optimized;
        // ... conversion logic
        return optimized;
    }
};

int main() {
    try {
        AISession session;
        
        std::vector<Message> messages = {
            {"user", "Hello, how are you?"},
            {"assistant", "I'm doing well, thank you for asking!"}
        };
        
        auto optimized = session.optimizeContext("session-123", messages, 
                                               "What did we discuss earlier?");
        
        std::cout << "Optimization complete\n";
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

## Advanced Usage

### RAII Wrapper Class
```cpp
#include <memory>
#include <string>
#include <vector>

class OfflineIntelligence {
private:
    struct Deleter {
        void operator()(OfflineIntelligenceHandle* handle) {
            if (handle) {
                offline_intelligence_free(handle);
            }
        }
    };
    
    std::unique_ptr<OfflineIntelligenceHandle, Deleter> handle_;
    
public:
    OfflineIntelligence() : handle_(offline_intelligence_new(), Deleter{}) {
        if (!handle_) {
            throw std::runtime_error("Failed to create AI instance");
        }
    }
    
    // Move semantics
    OfflineIntelligence(OfflineIntelligence&&) = default;
    OfflineIntelligence& operator=(OfflineIntelligence&&) = default;
    
    // Disable copy
    OfflineIntelligence(const OfflineIntelligence&) = delete;
    OfflineIntelligence& operator=(const OfflineIntelligence&) = delete;
    
    OptimizationResult optimizeContext(const std::string& sessionId,
                                     const std::vector<Message>& messages,
                                     const std::string& userQuery = "") {
        return offline_intelligence_optimize_context(
            handle_.get(),
            sessionId.c_str(),
            messages.data(),
            static_cast<int>(messages.size()),
            userQuery.empty() ? nullptr : userQuery.c_str()
        );
    }
    
    SearchResult search(const std::string& query, int limit = 10) {
        return offline_intelligence_search(
            handle_.get(),
            query.c_str(),
            nullptr,  // session_id
            limit
        );
    }
    
    std::string generateTitle(const std::vector<Message>& messages) {
        char* title = offline_intelligence_generate_title(
            handle_.get(),
            messages.data(),
            static_cast<int>(messages.size())
        );
        
        if (!title) {
            throw std::runtime_error("Failed to generate title");
        }
        
        std::string result(title);
        offline_intelligence_free_string(title);
        return result;
    }
};
```

### Exception-Safe Usage
```cpp
#include <stdexcept>

class AIException : public std::runtime_error {
public:
    explicit AIException(const std::string& message) 
        : std::runtime_error(message) {}
};

void safeAIOperation() {
    OfflineIntelligence ai;
    
    try {
        std::vector<Message> messages = {
            {"user", "Test message"},
            {"assistant", "Test response"}
        };
        
        auto result = ai.optimizeContext("test-session", messages);
        
        if (result.optimized_count == 0) {
            throw AIException("Context optimization failed");
        }
        
        std::cout << "Success: " << result.optimized_count << " messages optimized\n";
        
    } catch (const AIException& e) {
        std::cerr << "AI Error: " << e.what() << std::endl;
        throw;
    }
}
```

## API Reference

### Core Structures

#### OfflineIntelligenceHandle
Opaque handle representing an AI instance.

#### Message
```c
typedef struct {
    const char* role;      // "user", "assistant", "system"
    const char* content;   // Message content
} Message;
```

#### OptimizationResult
```c
typedef struct {
    const Message* optimized_messages;
    int original_count;
    int optimized_count;
    float compression_ratio;
} OptimizationResult;
```

#### SearchResult
```c
typedef struct {
    int total;
    const char* search_type;
} SearchResult;
```

### Core Functions

#### Instance Management
- `OfflineIntelligenceHandle* offline_intelligence_new()`
- `void offline_intelligence_free(OfflineIntelligenceHandle* handle)`

#### Core Operations
- `OptimizationResult offline_intelligence_optimize_context(OfflineIntelligenceHandle* handle, const char* session_id, const Message* messages, int message_count, const char* user_query)`
- `SearchResult offline_intelligence_search(OfflineIntelligenceHandle* handle, const char* query, const char* session_id, int limit)`
- `char* offline_intelligence_generate_title(OfflineIntelligenceHandle* handle, const Message* messages, int message_count)`

#### Utility Functions
- `void offline_intelligence_free_string(char* string)`

## Performance Optimization

### Memory Management
```cpp
class OptimizedAI {
private:
    OfflineIntelligenceHandle* handle_;
    std::vector<char*> allocated_strings_;
    
public:
    ~OptimizedAI() {
        // Clean up allocated strings
        for (char* str : allocated_strings_) {
            offline_intelligence_free_string(str);
        }
        offline_intelligence_free(handle_);
    }
    
    std::string getTitle(const std::vector<Message>& messages) {
        char* title = offline_intelligence_generate_title(handle_, 
                                                        messages.data(), 
                                                        messages.size());
        if (title) {
            allocated_strings_.push_back(title);
            return std::string(title);
        }
        return "";
    }
};
```

### Batch Processing
```cpp
class BatchProcessor {
private:
    OfflineIntelligence ai_;
    
public:
    std::vector<OptimizationResult> processBatch(
        const std::vector<std::pair<std::string, std::vector<Message>>>& sessions) {
        
        std::vector<OptimizationResult> results;
        results.reserve(sessions.size());
        
        for (const auto& [sessionId, messages] : sessions) {
            auto result = ai_.optimizeContext(sessionId, messages);
            results.push_back(result);
        }
        
        return results;
    }
};
```

## Integration Examples

### Qt Application Integration
```cpp
#include <QObject>
#include <QThread>
#include <QString>
#include "offline_intelligence_cpp.h"

class AIWorker : public QObject {
    Q_OBJECT
    
private:
    OfflineIntelligence ai_;
    
public slots:
    void processConversation(const QString& sessionId, 
                           const QStringList& messages) {
        std::vector<Message> aiMessages;
        // Convert QStringList to Message vector
        
        try {
            auto result = ai_.optimizeContext(sessionId.toStdString(), 
                                            aiMessages);
            emit processingComplete(QString::number(result.optimized_count));
        } catch (const std::exception& e) {
            emit errorOccurred(QString::fromStdString(e.what()));
        }
    }
    
signals:
    void processingComplete(const QString& result);
    void errorOccurred(const QString& error);
};
```

### Game Engine Integration
```cpp
class GameAI {
private:
    OfflineIntelligence ai_;
    
public:
    std::string generateNPCResponse(const std::string& context) {
        std::vector<Message> messages = {
            {"system", "You are an NPC in a fantasy game"},
            {"user", context}
        };
        
        return ai_.generateTitle(messages);
    }
    
    std::vector<std::string> searchGameKnowledge(const std::string& query) {
        auto result = ai_.search(query, 5);
        // Convert SearchResult to string vector
        return {};
    }
};
```

## Troubleshooting

### Common Issues

#### Linking Errors
```cmake
# Ensure proper linking
target_link_libraries(your_target 
    PRIVATE 
    offline_intelligence_cpp::offline_intelligence_cpp
    ${CMAKE_DL_LIBS}  # Required on some platforms
)
```

#### Runtime Errors
```cpp
// Enable debug output
#ifdef DEBUG
    setenv("RUST_LOG", "debug", 1);
#endif
```

#### Memory Management
```cpp
// Always free allocated strings
char* title = offline_intelligence_generate_title(handle, messages, count);
if (title) {
    std::string result(title);
    offline_intelligence_free_string(title);  // Don't forget this!
    return result;
}
```

## Testing

### CMake Tests
```bash
cd crates/cpp-bindings/build
ctest
```

### Manual Testing
```cpp
#include "offline_intelligence_cpp.h"
#include <cassert>

void testBasicFunctionality() {
    OfflineIntelligenceHandle* ai = offline_intelligence_new();
    assert(ai != nullptr);
    
    Message messages[1] = {{"user", "test"}};
    OptimizationResult result = offline_intelligence_optimize_context(
        ai, "test", messages, 1, nullptr
    );
    
    assert(result.original_count == 1);
    offline_intelligence_free(ai);
}
```

## License

Apache License 2.0

## Contributing

Contributions welcome. Please follow main repository guidelines.

For C++ specific issues, please use appropriate issue labels.