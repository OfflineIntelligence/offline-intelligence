# Getting Started with Offline Intelligence

Quick start guide to begin using the Offline Intelligence Library in your projects.

## Quick Installation

### For Python Developers
```bash
pip install offline-intelligence-py
```

### For Node.js Developers
```bash
npm install offline-intelligence-js
```

### For Java Developers
Add to your `pom.xml`:
```xml
<dependency>
    <groupId>com.offlineintelligence</groupId>
    <artifactId>offline-intelligence-java</artifactId>
    <version>0.1.2</version>
</dependency>
```

### For C++ Developers
```bash
# Using Conan
conan install offline-intelligence-cpp/0.1.2@

# Using CMake
find_package(offline_intelligence_cpp REQUIRED)
```

## Your First AI Conversation

### Python Example
```python
import asyncio
from offline_intelligence import MemoryStore, Message

async def first_conversation():
    # Initialize the AI
    memory_store = MemoryStore()
    
    # Create a simple conversation
    messages = [
        Message(role="user", content="Hello! Can you help me learn about AI?"),
        Message(role="assistant", content="Of course! AI stands for Artificial Intelligence, which refers to computer systems designed to perform tasks that typically require human intelligence.")
    ]
    
    # Store the conversation
    await memory_store.add_session("learning-session", messages)
    
    # Ask a follow-up question
    follow_up = Message(role="user", content="That's interesting! What are some practical applications?")
    response = await memory_store.generate_response([follow_up])
    
    print(f"AI Response: {response}")

# Run the example
asyncio.run(first_conversation())
```

### JavaScript Example
```javascript
const { MemoryStore, Message } = require('offline-intelligence-js');

async function firstConversation() {
    const memoryStore = new MemoryStore();
    
    const messages = [
        new Message('user', 'Hello! Can you help me learn about AI?'),
        new Message('assistant', 'Of course! AI stands for Artificial Intelligence...')
    ];
    
    await memoryStore.addSession('learning-session', messages);
    
    const followUp = new Message('user', 'What are some practical applications?');
    const response = await memoryStore.generateResponse([followUp]);
    
    console.log(`AI Response: ${response}`);
}

firstConversation().catch(console.error);
```

### Java Example
```java
import com.offlineintelligence.*;

public class FirstConversation {
    public static void main(String[] args) throws Exception {
        try (OfflineIntelligence ai = new OfflineIntelligence()) {
            Message[] messages = {
                new Message("user", "Hello! Can you help me learn about AI?"),
                new Message("assistant", "Of course! AI stands for Artificial Intelligence...")
            };
            
            ai.addSession("learning-session", messages);
            
            Message followUp = new Message("user", "What are some practical applications?");
            String response = ai.generateResponse(new Message[]{followUp});
            
            System.out.println("AI Response: " + response);
        }
    }
}
```

### C++ Example
```cpp
#include "offline_intelligence.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    OfflineIntelligenceHandle* ai = offline_intelligence_new();
    if (!ai) {
        fprintf(stderr, "Failed to initialize AI\n");
        return 1;
    }
    
    Message messages[2] = {
        {"user", "Hello! Can you help me learn about AI?"},
        {"assistant", "Of course! AI stands for Artificial Intelligence..."}
    };
    
    // Add session and process (implementation details omitted for brevity)
    
    offline_intelligence_free(ai);
    return 0;
}
```

## Core Concepts

### Sessions
Sessions represent individual conversation threads. Each session maintains its own context and memory.

```python
# Create a new session
await memory_store.add_session("my-session-id", initial_messages)

# Work with existing session
optimized_context = await memory_store.optimize_context("my-session-id")
```

### Messages
Messages are the basic units of communication, consisting of a role and content.

```python
# Message roles:
# - "user": Human input
# - "assistant": AI responses  
# - "system": System instructions

message = Message(role="user", content="Your message here")
```

### Context Optimization
The library automatically optimizes conversation context to maintain relevance while managing memory usage.

```python
# Optimize context for better performance
optimized_messages = await memory_store.optimize_context(
    session_id="my-session",
    user_query="specific question about previous conversation"
)
```

## Common Use Cases

### 1. Chatbot Implementation
```python
class SimpleChatbot:
    def __init__(self):
        self.memory_store = MemoryStore()
        self.session_id = "chatbot-session"
    
    async def respond(self, user_message):
        # Add user message
        message = Message(role="user", content=user_message)
        
        # Get AI response
        response = await self.memory_store.generate_response([message])
        
        # Store the exchange
        await self.memory_store.add_session(self.session_id, [message])
        
        return response

# Usage
bot = SimpleChatbot()
response = await bot.respond("What can you help me with?")
```

### 2. Document Analysis
```python
async def analyze_document(content):
    memory_store = MemoryStore()
    
    # Process document in chunks
    chunks = split_into_chunks(content)
    messages = []
    
    for i, chunk in enumerate(chunks):
        message = Message(
            role="user", 
            content=f"Analyze this document section {i+1}: {chunk}"
        )
        messages.append(message)
    
    # Get comprehensive analysis
    analysis = await memory_store.generate_response(messages)
    return analysis
```

### 3. Question Answering System
```python
class QASystem:
    def __init__(self, knowledge_base):
        self.memory_store = MemoryStore()
        self.load_knowledge(knowledge_base)
    
    async def answer_question(self, question):
        # Search relevant context
        search_results = await self.memory_store.search(question, limit=5)
        
        # Generate contextual response
        context_messages = [
            Message(role="system", content="Use the following context to answer:"),
            Message(role="user", content=str(search_results)),
            Message(role="user", content=question)
        ]
        
        return await self.memory_store.generate_response(context_messages)
```

## Configuration Options

### Basic Configuration
```python
from offline_intelligence import Config

# Default configuration
config = Config()

# Custom configuration
config = Config(
    max_context_length=8192,  # Larger context window
    cache_size=2000,          # More cached items
    cleanup_threshold=0.75    # Earlier cleanup
)
```

### Performance Tuning
```python
# High-performance configuration
config = Config(
    max_memory_mb=2048,       # 2GB memory limit
    num_threads=8,            # Parallel processing
    cache_size=5000,          # Large cache
    model_path="/path/to/fast/model.gguf"
)
```

## Error Handling

### Python Error Handling
```python
from offline_intelligence import OfflineIntelligenceError

try:
    response = await memory_store.generate_response(messages)
except OfflineIntelligenceError as e:
    print(f"AI processing failed: {e}")
    # Handle specific error types
except Exception as e:
    print(f"Unexpected error: {e}")
```

### JavaScript Error Handling
```javascript
try {
    const response = await memoryStore.generateResponse(messages);
} catch (error) {
    if (error.code === 'AI_PROCESSING_ERROR') {
        console.error('AI processing failed:', error.message);
    } else {
        console.error('Unexpected error:', error);
    }
}
```

## Best Practices

### 1. Session Management
```python
# Use descriptive session IDs
session_id = f"user-{user_id}-conversation-{timestamp}"

# Clean up old sessions periodically
await memory_store.cleanup_old_sessions(days=30)
```

### 2. Memory Optimization
```python
# Monitor memory usage
stats = await memory_store.get_stats()
if stats['memory_usage_mb'] > 1000:
    await memory_store.perform_cleanup()
```

### 3. Error Recovery
```python
async def robust_ai_call(messages, max_retries=3):
    for attempt in range(max_retries):
        try:
            return await memory_store.generate_response(messages)
        except OfflineIntelligenceError:
            if attempt == max_retries - 1:
                raise
            await asyncio.sleep(2 ** attempt)  # Exponential backoff
```

## Next Steps

### Explore Documentation
- [Core Library Documentation](./OFFLINE_INTELLIGENCE_CORE.md)
- [Python Bindings Guide](./PYTHON_BINDINGS.md)
- [JavaScript Bindings Guide](./JS_BINDINGS.md)
- [Java Bindings Guide](./JAVA_BINDINGS.md)
- [C++ Bindings Guide](./CPP_BINDINGS.md)

### Advanced Topics
- Model selection and optimization
- Custom training and fine-tuning
- Integration with web frameworks
- Performance benchmarking
- Production deployment

### Community Resources
- Example projects and templates
- Tutorials and guides
- Community forums and support
- Contribution opportunities

## Need Help?

If you encounter issues:
1. Check the [Environment Setup Guide](./ENVIRONMENT_SETUP.md)
2. Review language-specific documentation
3. Search existing issues on GitHub
4. Open a new issue with detailed information

Start building amazing AI-powered applications today!