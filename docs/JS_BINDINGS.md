# JavaScript Bindings for Offline Intelligence

JavaScript/Node.js bindings for the Offline Intelligence Library, bringing high-performance offline AI capabilities to JavaScript applications through N-API.

## Overview

This package provides JavaScript developers with native access to the Offline Intelligence Library through Node.js N-API bindings. The implementation offers excellent performance with familiar JavaScript patterns and seamless integration with the Node.js ecosystem.

## Features

### Core Capabilities
- Native Node.js integration with N-API
- Promise-based asynchronous operations
- Automatic memory management through V8 garbage collector
- TypeScript type definitions included
- Cross-platform compatibility (Windows, macOS, Linux)

### JavaScript-Specific Features
- Promise/async-await support
- EventEmitter integration for streaming responses
- TypeScript declarations for type safety
- CommonJS and ES Module support
- Integration with Node.js streams

## Installation

### NPM (Recommended)
```bash
npm install offline-intelligence-js
```

### Yarn
```bash
yarn add offline-intelligence-js
```

### From Source
```bash
# Clone repository
git clone https://github.com/offline-intelligence/library.git
cd library

# Build JavaScript bindings
cd crates/js-bindings
npm install
npm run build
```

## System Requirements

- Node.js 16.0 or higher
- npm 7.0 or higher
- Rust toolchain (for building from source)
- Compatible C++ compiler for native modules

## Basic Usage

```javascript
const { MemoryStore, Config, Message } = require('offline-intelligence-js');

async function basicExample() {
    // Initialize with default configuration
    const config = new Config();
    const memoryStore = new MemoryStore(config);
    
    // Create conversation messages
    const messages = [
        new Message('user', 'Hello, how are you?'),
        new Message('assistant', 'I\'m doing well, thank you for asking!')
    ];
    
    // Store conversation session
    await memoryStore.addSession('session-123', messages);
    
    // Optimize context
    const optimized = await memoryStore.optimizeContext(
        'session-123',
        'What did we discuss earlier?'
    );
    
    // Search memory
    const results = await memoryStore.search('hello', 10);
    
    console.log('Results:', results);
}

// Run the example
basicExample().catch(console.error);
```

## Advanced Usage

### Async/Await with Error Handling
```javascript
async function robustExample() {
    let memoryStore;
    
    try {
        const config = new Config({
            maxContextLength: 8192,
            cacheSize: 2000,
            cleanupThreshold: 0.75
        });
        
        memoryStore = new MemoryStore(config);
        
        const messages = [
            new Message('user', 'Tell me about machine learning'),
            new Message('assistant', 'Machine learning is a subset of AI...')
        ];
        
        await memoryStore.addSession('ml-discussion', messages);
        
        const response = await memoryStore.generateResponse(messages);
        console.log('AI Response:', response);
        
    } catch (error) {
        console.error('AI operation failed:', error.message);
    } finally {
        if (memoryStore) {
            await memoryStore.cleanup();
        }
    }
}
```

### Streaming Responses
```javascript
const { EventEmitter } = require('events');

class AIStream extends EventEmitter {
    constructor(memoryStore) {
        super();
        this.memoryStore = memoryStore;
    }
    
    async processStream(messages) {
        try {
            // Emit chunks as they arrive
            for await (const chunk of this.memoryStore.generateStream(messages)) {
                this.emit('data', chunk);
            }
            this.emit('end');
        } catch (error) {
            this.emit('error', error);
        }
    }
}

// Usage
async function streamingExample() {
    const memoryStore = new MemoryStore();
    const stream = new AIStream(memoryStore);
    
    stream.on('data', (chunk) => {
        process.stdout.write(chunk);
    });
    
    stream.on('end', () => {
        console.log('\nResponse complete');
    });
    
    stream.on('error', (error) => {
        console.error('Stream error:', error);
    });
    
    const messages = [new Message('user', 'Explain quantum computing')];
    await stream.processStream(messages);
}
```

### TypeScript Support
```typescript
import { MemoryStore, Config, Message, SearchResult } from 'offline-intelligence-js';

interface ConversationContext {
    sessionId: string;
    messages: Message[];
    timestamp: Date;
}

class AIManager {
    private memoryStore: MemoryStore;
    
    constructor() {
        const config = new Config({
            maxContextLength: 4096,
            modelPath: './models/custom-model.gguf'
        });
        this.memoryStore = new MemoryStore(config);
    }
    
    async processConversation(context: ConversationContext): Promise<string> {
        await this.memoryStore.addSession(context.sessionId, context.messages);
        const response = await this.memoryStore.generateResponse(context.messages);
        return response;
    }
    
    async searchKnowledge(query: string, limit: number = 10): Promise<SearchResult[]> {
        return await this.memoryStore.search(query, limit);
    }
}
```

## API Reference

### Main Classes

#### MemoryStore
Primary interface for AI operations.

##### Constructor:
- `new MemoryStore(config?: Config)`

##### Methods:
- `addSession(sessionId: string, messages: Message[]): Promise<void>`
- `optimizeContext(sessionId: string, userQuery?: string): Promise<Message[]>`
- `search(query: string, limit?: number): Promise<SearchResult[]>`
- `generateResponse(messages: Message[]): Promise<string>`
- `generateTitle(messages: Message[]): Promise<string>`
- `getStats(): Promise<object>`
- `cleanup(): Promise<void>`

#### Config
Configuration class.

##### Constructor:
- `new Config(options?: ConfigOptions)`

##### Options:
- `maxContextLength: number` (default: 4096)
- `cacheSize: number` (default: 1000)
- `cleanupThreshold: number` (default: 0.8)
- `modelPath: string` (optional)
- `maxMemoryMB: number` (optional)

#### Message
Message class.

##### Constructor:
- `new Message(role: string, content: string)`

##### Properties:
- `role: string` ('user', 'assistant', 'system')
- `content: string`

#### SearchResult
Search result interface.

##### Properties:
- `id: string`
- `score: number`
- `content: string`
- `metadata: object`

### Events

#### MemoryStore Events
```javascript
const memoryStore = new MemoryStore();

memoryStore.on('processing:start', () => {
    console.log('AI processing started');
});

memoryStore.on('processing:end', (result) => {
    console.log('Processing completed:', result);
});

memoryStore.on('error', (error) => {
    console.error('Processing error:', error);
});
```

## Performance Optimization

### Memory Management
```javascript
const config = new Config({
    maxMemoryMB: 1024,  // Limit memory usage
    cleanupInterval: 300000  // Cleanup every 5 minutes
});

const memoryStore = new MemoryStore(config);

// Monitor memory usage
setInterval(async () => {
    const stats = await memoryStore.getStats();
    console.log(`Memory usage: ${stats.memoryUsageMB} MB`);
}, 60000);
```

### Batch Operations
```javascript
async function batchProcessSessions(sessions) {
    const memoryStore = new MemoryStore();
    
    // Process multiple sessions concurrently
    const promises = sessions.map(async (session) => {
        await memoryStore.addSession(session.id, session.messages);
        return memoryStore.optimizeContext(session.id);
    });
    
    const results = await Promise.all(promises);
    return results;
}
```

## Integration Examples

### Express.js Integration
```javascript
const express = require('express');
const { MemoryStore, Message } = require('offline-intelligence-js');

const app = express();
const memoryStore = new MemoryStore();

app.use(express.json());

app.post('/chat', async (req, res) => {
    try {
        const { messages } = req.body;
        
        const aiMessages = messages.map(msg => 
            new Message(msg.role, msg.content)
        );
        
        const response = await memoryStore.generateResponse(aiMessages);
        
        res.json({ response });
    } catch (error) {
        res.status(500).json({ error: error.message });
    }
});

app.listen(3000, () => {
    console.log('AI server running on port 3000');
});
```

### Real-time Chat Application
```javascript
const WebSocket = require('ws');
const { MemoryStore, Message } = require('offline-intelligence-js');

const wss = new WebSocket.Server({ port: 8080 });
const memoryStore = new MemoryStore();

wss.on('connection', (ws) => {
    console.log('New client connected');
    
    ws.on('message', async (data) => {
        try {
            const { type, payload } = JSON.parse(data);
            
            if (type === 'message') {
                const message = new Message('user', payload.text);
                
                // Stream response back to client
                for await (const chunk of memoryStore.generateStream([message])) {
                    ws.send(JSON.stringify({
                        type: 'response_chunk',
                        content: chunk
                    }));
                }
                
                ws.send(JSON.stringify({
                    type: 'response_end'
                }));
            }
        } catch (error) {
            ws.send(JSON.stringify({
                type: 'error',
                message: error.message
            }));
        }
    });
});
```

## Troubleshooting

### Common Issues

#### Module Loading Errors
```bash
# Rebuild native modules
npm rebuild offline-intelligence-js
```

#### Performance Issues
```javascript
// Enable debug logging
process.env.DEBUG = 'offline-intelligence:*';
```

#### Memory Leaks
```javascript
// Proper cleanup
process.on('SIGINT', async () => {
    await memoryStore.cleanup();
    process.exit(0);
});
```

## Testing

### Unit Tests
```bash
cd crates/js-bindings
npm test
```

### TypeScript Tests
```bash
npm run test:typescript
```

## License

Apache License 2.0

## Contributing

Contributions welcome. Please follow main repository guidelines.

For JavaScript-specific issues, please use appropriate issue labels.