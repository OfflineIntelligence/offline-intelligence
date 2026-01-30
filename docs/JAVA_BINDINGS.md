# Offline Intelligence Java Bindings

Java bindings for the Offline Intelligence Library using JNI, providing offline AI inference capabilities with context management and memory optimization.

## Building

```bash
# Build the native library
cargo build --release

# Build Java classes and create JAR
./build.sh
```

## Usage

```java
import com.offlineintelligence.*;

public class Example {
    public static void main(String[] args) {
        // Initialize the library
        OfflineIntelligence oi = new OfflineIntelligence();
        
        // Create messages
        Message[] messages = {
            new Message("user", "Hello, how are you?"),
            new Message("assistant", "I'm doing well, thank you for asking!"),
            new Message("user", "What can you help me with?")
        };
        
        // Optimize context
        OptimizationResult result = oi.optimizeContext("session123", messages, "What can you help me with?");
        System.out.println("Optimized from " + result.getOriginalCount() + " to " + result.getOptimizedCount() + " messages");
        
        // Search memory
        SearchResult searchResult = oi.search("help me", "session123", 10);
        System.out.println("Found " + searchResult.getTotal() + " results");
        
        // Generate title
        String title = oi.generateTitle(messages);
        System.out.println("Generated title: " + title);
        
        // Clean up
        oi.dispose();
    }
}
```

## Features

- **Offline AI Inference**: Run LLMs locally without internet connection
- **Context Management**: Intelligent conversation context optimization
- **Memory Search**: Hybrid semantic and keyword search across conversations
- **Multi-format Support**: Support for GGUF, GGML, ONNX, TensorRT, and Safetensors models
- **Cross-platform**: Works on Windows, macOS, and Linux

## Requirements

- Java 8 or higher
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

## License

Apache 2.0