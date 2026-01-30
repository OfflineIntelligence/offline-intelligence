# Java Bindings for Offline Intelligence

Java bindings for the Offline Intelligence Library, providing enterprise-grade offline AI capabilities for Java applications through JNI (Java Native Interface).

## Overview

This package enables Java developers to leverage the full power of the Offline Intelligence Library in their applications. The bindings provide type-safe interfaces with automatic resource management and seamless integration with Java ecosystem tools.

## Features

### Core Capabilities
- Native JNI integration with zero-copy data transfer
- Automatic garbage collection integration
- Thread-safe operations with Java concurrency support
- Comprehensive exception handling with Java exceptions
- Maven and Gradle build system integration

### Java-Specific Features
- Strong typing with Java generics
- Builder pattern for configuration objects
- Integration with Java logging frameworks
- Support for Java collections and streams
- CompletableFuture integration for async operations

## Installation

### Maven
```xml
<dependency>
    <groupId>com.offlineintelligence</groupId>
    <artifactId>offline-intelligence-java</artifactId>
    <version>0.1.2</version>
</dependency>
```

### Gradle
```gradle
implementation 'com.offlineintelligence:offline-intelligence-java:0.1.2'
```

### Manual Installation
```bash
# Build from source
cd crates/java-bindings
./gradlew build
```

## System Requirements

- Java 11 or higher
- Maven 3.6+ or Gradle 6.0+
- Rust toolchain (for building from source)
- Compatible C++ compiler for native libraries

## Basic Usage

```java
import com.offlineintelligence.*;

public class BasicExample {
    public static void main(String[] args) throws Exception {
        // Initialize with default configuration
        Config config = Config.builder().build();
        OfflineIntelligence ai = new OfflineIntelligence(config);
        
        // Create conversation messages
        Message[] messages = {
            new Message("user", "Hello, how are you?"),
            new Message("assistant", "I'm doing well, thank you for asking!")
        };
        
        // Store conversation session
        ai.addSession("session-123", messages);
        
        // Optimize context
        Message[] optimized = ai.optimizeContext("session-123", "What did we discuss earlier?");
        
        // Search memory
        SearchResult[] results = ai.search("hello", 10);
        
        // Clean up resources
        ai.close();
    }
}
```

## Advanced Usage

### Try-With-Resources Pattern
```java
public class ResourceManagementExample {
    public static void main(String[] args) {
        // Automatic resource cleanup
        try (OfflineIntelligence ai = new OfflineIntelligence()) {
            // Perform operations
            Message[] response = ai.processConversation(messages);
            // Resources automatically released
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```

### Async Operations with CompletableFuture
```java
import java.util.concurrent.CompletableFuture;

public class AsyncExample {
    public static void main(String[] args) {
        try (OfflineIntelligence ai = new OfflineIntelligence()) {
            // Async processing
            CompletableFuture<Message[]> future = ai.optimizeContextAsync(
                "session-123", 
                "Follow-up question"
            );
            
            // Handle result
            future.thenAccept(optimizedMessages -> {
                System.out.println("Optimization complete");
                // Process results
            }).exceptionally(throwable -> {
                System.err.println("Operation failed: " + throwable.getMessage());
                return null;
            });
        }
    }
}
```

### Builder Pattern for Configuration
```java
public class ConfigurationExample {
    public static void main(String[] args) {
        Config config = Config.builder()
            .maxContextLength(8192)
            .cacheSize(2000)
            .cleanupThreshold(0.75)
            .modelPath("/path/to/model")
            .build();
            
        OfflineIntelligence ai = new OfflineIntelligence(config);
    }
}
```

## API Reference

### Main Classes

#### OfflineIntelligence
Primary interface for all AI operations.

##### Constructors:
- `OfflineIntelligence()` - Default constructor
- `OfflineIntelligence(Config config)` - Constructor with custom configuration

##### Methods:
- `void addSession(String sessionId, Message[] messages)`
- `Message[] optimizeContext(String sessionId, String userQuery)`
- `SearchResult[] search(String query, int limit)`
- `String generateTitle(Message[] messages)`
- `SessionStats getStats()`
- `void close()` - Release native resources

#### Config
Configuration builder class.

##### Builder Methods:
- `maxContextLength(int length)` - Set maximum context tokens
- `cacheSize(int size)` - Set cache capacity
- `cleanupThreshold(double threshold)` - Set memory cleanup threshold
- `modelPath(String path)` - Set custom model path
- `build()` - Create Config instance

#### Message
Immutable message class.

##### Constructor:
- `Message(String role, String content)`

##### Methods:
- `String getRole()` - Get message role
- `String getContent()` - Get message content

#### SearchResult
Search result container.

##### Methods:
- `String getId()` - Get result identifier
- `double getScore()` - Get relevance score
- `String getContent()` - Get result content

### Exception Handling
```java
try {
    OfflineIntelligence ai = new OfflineIntelligence();
    Message[] result = ai.someOperation();
} catch (OfflineIntelligenceException e) {
    System.err.println("AI operation failed: " + e.getMessage());
    // Handle specific error types
} catch (Exception e) {
    System.err.println("Unexpected error: " + e.getMessage());
}
```

## Performance Optimization

### Memory Management
```java
public class MemoryOptimization {
    public static void main(String[] args) {
        Config config = Config.builder()
            .maxMemoryMB(1024)  // Limit to 1GB
            .cleanupIntervalSeconds(300)  // Cleanup every 5 minutes
            .build();
            
        try (OfflineIntelligence ai = new OfflineIntelligence(config)) {
            // Memory-optimized operations
        }
    }
}
```

### Batch Processing
```java
public class BatchProcessing {
    public static void main(String[] args) {
        try (OfflineIntelligence ai = new OfflineIntelligence()) {
            // Process multiple sessions efficiently
            String[] sessionIds = {"session-1", "session-2", "session-3"};
            Message[][] messageBatches = {batch1, batch2, batch3};
            
            for (int i = 0; i < sessionIds.length; i++) {
                ai.addSession(sessionIds[i], messageBatches[i]);
            }
        }
    }
}
```

## Integration Examples

### Spring Boot Integration
```java
@Component
public class AIService {
    private final OfflineIntelligence ai;
    
    public AIService() {
        this.ai = new OfflineIntelligence();
    }
    
    public String processConversation(List<MessageDTO> messages) {
        Message[] aiMessages = messages.stream()
            .map(dto -> new Message(dto.getRole(), dto.getContent()))
            .toArray(Message[]::new);
            
        // Process and return result
        return ai.generateResponse(aiMessages);
    }
    
    @PreDestroy
    public void cleanup() {
        ai.close();
    }
}
```

### Android Integration
```java
public class MainActivity extends AppCompatActivity {
    private OfflineIntelligence ai;
    
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        // Initialize AI on background thread
        new Thread(() -> {
            try {
                Config config = Config.builder()
                    .modelPath(getExternalFilesDir(null).getAbsolutePath())
                    .build();
                ai = new OfflineIntelligence(config);
            } catch (Exception e) {
                Log.e("AI", "Failed to initialize", e);
            }
        }).start();
    }
}
```

## Troubleshooting

### Common Issues

#### UnsatisfiedLinkError
```bash
# Ensure native library is in java.library.path
-Djava.library.path=/path/to/native/libs
```

#### Memory Issues
```java
// Monitor and configure memory usage
Config config = Config.builder()
    .maxMemoryMB(512)  // Reduce memory footprint
    .build();
```

#### Threading Problems
```java
// Use thread-safe operations
synchronized(ai) {
    Message[] result = ai.optimizeContext(sessionId, query);
}
```

## Testing

### Unit Tests
```bash
cd crates/java-bindings
./gradlew test
```

### Integration Tests
```java
@Test
public void testConversationFlow() {
    try (OfflineIntelligence ai = new OfflineIntelligence()) {
        Message[] messages = {
            new Message("user", "Test message"),
            new Message("assistant", "Test response")
        };
        
        ai.addSession("test-session", messages);
        Message[] optimized = ai.optimizeContext("test-session", "Test query");
        
        assertNotNull(optimized);
        assertTrue(optimized.length > 0);
    }
}
```

## License

Apache License 2.0

## Contributing

Contributions welcome. Please follow the main repository guidelines.

For Java-specific issues, please use the appropriate issue labels.