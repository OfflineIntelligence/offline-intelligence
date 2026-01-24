# Offline Intelligence Java Bindings

Java bindings for the Offline Intelligence Library - High-performance LLM inference engine with memory management capabilities.

## Usage with JitPack

### Maven
```xml
<repositories>
    <repository>
        <id>jitpack.io</id>
        <url>https://jitpack.io</url>
    </repository>
</repositories>

<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offline-intelligence</artifactId>
    <version>0.1.1</version>
</dependency>
```

### Gradle
```gradle
repositories {
    maven { url 'https://jitpack.io' }
}

dependencies {
    implementation 'com.github.OfflineIntelligence:offline-intelligence:0.1.1'
}
```

## Quick Start

```java
import com.offlineintelligence.Config;
import com.offlineintelligence.Server;

public class Example {
    public static void main(String[] args) {
        try {
            Config config = Config.fromEnv();
            boolean success = Server.runServer(config);
            System.out.println("Server started: " + success);
        } catch (Exception e) {
            System.err.println("Failed to start server: " + e.getMessage());
        }
    }
}
```

## Features

- **Core LLM Integration**: Direct access to LLM engine functionality
- **Memory Management**: Base memory operations and database access
- **Configuration**: Flexible configuration system
- **Metrics**: Performance monitoring and telemetry
- **Proxy Interface**: Stream generation and API proxy functionality

## Architecture

This package provides bindings to the core open-source components (80%) of the Offline Intelligence system. Proprietary extensions are available separately.

## Platform Support

- Windows (x64)
- macOS (Intel/Apple Silicon)
- Linux (x64/ARM64)

## Requirements

- Java 11 or higher
- JNA 5.13.0 (included as dependency)