# Offline Intelligence JavaScript Bindings

JavaScript/Node.js bindings for the Offline Intelligence Library using N-API, providing offline AI inference capabilities with context management and memory optimization.

## Installation

```bash
npm install offline-intelligence
```

Or build from source:

```bash
npm install
npm run build
```

## Quick Start

```javascript
const { OfflineIntelligence, Message, Config } = require('offline-intelligence');

async function main() {
  // Initialize the library
  const oi = new OfflineIntelligence();
  
  // Load configuration
  const config = new Config();
  console.log(`Model path: ${config.modelPath}`);
  console.log(`Context size: ${config.ctxSize}`);
  
  // Create messages
  const messages = [
    new Message('user', 'Hello, how are you?'),
    new Message('assistant', 'I\'m doing well, thank you for asking!'),
    new Message('user', 'What can you help me with?')
  ];
  
  // Optimize context
  const result = await oi.optimizeContext('session123', messages, 'What can you help me with?');
  console.log(`Optimized from ${result.originalCount} to ${result.optimizedCount} messages`);
  
  // Search memory
  const searchResult = await oi.search('help me', 'session123', 10);
  console.log(`Found ${searchResult.total} results`);
  
  // Generate title
  const title = await oi.generateTitle(messages);
  console.log(`Generated title: ${title}`);
}

main().catch(console.error);
```

## Features

- **Offline AI Inference**: Run LLMs locally without internet connection
- **Context Management**: Intelligent conversation context optimization
- **Memory Search**: Hybrid semantic and keyword search across conversations
- **Multi-format Support**: Support for GGUF, GGML, ONNX, TensorRT, and Safetensors models
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Async/Await Support**: Full Promise-based API for modern JavaScript

## Requirements

- Node.js 14.0.0 or higher
- npm or yarn
- Rust toolchain for building from source
- Compatible LLM model file

## Configuration

The library reads configuration from environment variables:

```bash
export LLAMA_BIN="/path/to/llama-binary"
export MODEL_PATH="/path/to/model.gguf"
export CTX_SIZE="8192"
export BATCH_SIZE="256"
export THREADS="6"
export GPU_LAYERS="20"
```

## TypeScript Support

Full TypeScript definitions are included:

```typescript
import { OfflineIntelligence, Message, Config } from 'offline-intelligence';

const oi = new OfflineIntelligence();
const messages: Message[] = [
  new Message('user', 'Hello'),
  new Message('assistant', 'Hi there!')
];
```

## License

Apache 2.0