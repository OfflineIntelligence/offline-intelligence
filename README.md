<div align="center">

<h1>Offline Intelligence Library</h1>

Run AI models entirely on your own machine. No internet, no cloud, no data leaves your device.
Cross-platform server with bindings for Python, JavaScript, Rust, C++, and Java.

<br>

[![Crates.io](https://img.shields.io/crates/v/offline-intelligence.svg)](https://crates.io/crates/offline-intelligence)
[![PyPI](https://img.shields.io/pypi/v/offline-intelligence.svg)](https://pypi.org/project/offline-intelligence/)
[![npm](https://img.shields.io/npm/v/offline-intelligence.svg)](https://www.npmjs.com/package/offline-intelligence)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/OfflineIntelligence/offline-intelligence/blob/main/LICENSE)

<br>

**Current Version:** v0.1.5 (March 30, 2026) | **License:** Apache 2.0

</div>

---

## What Is This?

The Offline Intelligence Library is a server that runs AI language models (LLMs) on your own computer. You download a model file once, and from that point on all AI inference happens locally. No API calls to OpenAI, no subscription fees, no data sent to anyone.

The server is written in Rust for speed and stability. Once it is running, you talk to it over HTTP from any language: Python, JavaScript, Java, C++, or Rust. The server handles everything: loading the model, managing conversation memory, streaming responses token by token, and optionally fetching live data (weather, currency, crypto prices) to answer questions the model alone could not.

**If you just want to try it:** jump to [Quick Start](#quick-start).

---

## Table of Contents

- [What Is This?](#what-is-this)
- [Features](#features)
- [What's New in v0.1.5](#whats-new-in-v015)
- [Supported Platforms](#supported-platforms)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Language Usage Guide](#language-usage-guide)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [Benchmarks](#benchmarks)
- [Architecture](#architecture)
- [Security](#security)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)
- [Support](#support)
- [Changelog](#changelog)
- [Citation](#citation)

---

## Features

| Feature | Description |
|---------|-------------|
| **5 Language Bindings** | Rust, Python, JavaScript/Node.js, Java, C++. All talk to the same server over HTTP |
| **Fully Offline** | Runs entirely on your machine. No internet required after model download |
| **Privacy First** | All data stays local. No telemetry, no cloud calls |
| **Streaming Responses** | Tokens stream back in real time, just like ChatGPT |
| **Conversation Memory** | SQLite-backed persistent memory with semantic search (HNSW index) |
| **Live Web Tools** | Automatically fetches weather, currency rates, and crypto prices to answer live questions |
| **User Authentication** | Built-in registration, login, JWT sessions, and Google OAuth 2.0 |
| **API Key Management** | Stores your HuggingFace and OpenRouter keys encrypted on-device |
| **Online / Offline Toggle** | Switch between local llama.cpp and OpenRouter cloud at runtime without restarting the server |
| **File Attachments** | Upload and attach files to conversations |
| **Auto Hardware Detection** | Automatically picks the right GPU layers, thread count, and memory limits for your machine |
| **Prometheus Metrics** | `/metrics` endpoint compatible with Grafana and any Prometheus-based monitoring stack |
| **Multi-Format Models** | Supports GGUF, GGML, ONNX, SafeTensors, CoreML, TensorRT model formats |

---

## What's New in v0.1.5

### Live Web Tools

The server now detects certain questions in real time and fetches live data before sending the conversation to the AI model. This means the model can answer questions it otherwise couldn't (current temperature, today's exchange rates, live crypto prices).

**How it works:** Every incoming user message is scanned for intent. If a relevant intent is detected, the data is fetched in parallel (max 8 seconds per source, 10-second hard deadline), formatted with numbered `[1]`, `[2]` citation markers, and injected as a system context block. If the fetch times out or fails, the model answers from its training data silently, with no error shown to the user.

| Intent | Trigger example | Data source |
|--------|----------------|-------------|
| Weather | "What's the weather in Tokyo?" | Open-Meteo + Nominatim (keyless) |
| Currency | "Convert 200 USD to EUR" | ExchangeRate-API, 160+ currencies (keyless) |
| Crypto price | "What is Bitcoin worth right now?" | CoinGecko free API (keyless) |

Manage tools via API:
```bash
GET  http://127.0.0.1:9999/tools/settings
POST http://127.0.0.1:9999/tools/settings   {"enabled": true, "brave_key": "optional"}
```

### User Authentication

Full auth stack built into the server. No third-party service needed:

```bash
POST /auth/register   {"username": "alice", "email": "alice@example.com", "password": "secret"}
POST /auth/login      {"email": "alice@example.com", "password": "secret"}
GET  /auth/google?redirect_uri=http://localhost:3000/callback
GET  /auth/verify?token=<email-verification-token>
```

Passwords are hashed with Argon2. Login returns a JWT token. Pass it as `Authorization: Bearer <token>` on protected endpoints.

### Encrypted API Key Storage

Store your HuggingFace and OpenRouter keys on-device. They are encrypted using a machine-specific key before being written to SQLite. They never exist in plaintext outside the process:

```bash
POST   /api-keys   {"key_type": "huggingface", "value": "hf_..."}
POST   /api-keys   {"key_type": "openrouter",  "value": "sk-or-..."}
GET    /api-keys?key_type=huggingface
DELETE /api-keys?key_type=openrouter
```

### Runtime Mode Switching

Switch between local (llama.cpp) and cloud (OpenRouter) inference without restarting:

```bash
POST /mode   {"mode": "offline"}
POST /mode   {"mode": "online"}
```

### User Feedback

```bash
POST /feedback   {"message": "Really helpful!", "email": "optional@email.com"}
```

---

## Supported Platforms

| OS | Architectures | Minimum Version |
|----|--------------|-----------------|
| Windows | x86_64, ARM64 | Windows 10 |
| Linux | x86_64, ARM64 | Ubuntu 20.04 / CentOS 8 |
| macOS | x86_64, Apple Silicon | macOS 11.0 |

---

## Quick Start

This gets you from zero to a running AI server in 5 steps.

### Step 1: Download llama-server

llama-server is the engine that runs the AI model. Download a prebuilt binary from:
**https://github.com/ggerganov/llama.cpp/releases**

Look for the most recent release and download the zip matching your OS:

| OS | File to look for |
|----|-----------------|
| Windows | `llama-b*-bin-win-*-x64.zip` → extract `llama-server.exe` |
| macOS Apple Silicon | `llama-b*-bin-macos-arm64.zip` → extract `llama-server` |
| macOS Intel | `llama-b*-bin-macos-x64.zip` → extract `llama-server` |
| Linux x86_64 | `llama-b*-bin-ubuntu-x64.zip` → extract `llama-server` |

Place the binary somewhere on your system, for example:
- Windows: `C:\llama\llama-server.exe`
- macOS/Linux: `/usr/local/bin/llama-server`

### Step 2: Download a Model

The library uses GGUF format model files. Pick one based on your available RAM:

| Model | File size | RAM needed | Download |
|-------|-----------|------------|----------|
| Llama 3.2 3B Q4 | ~2 GB | 4 GB | https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF |
| Mistral 7B Q4 | ~4 GB | 8 GB | https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF |
| Llama 3 8B Q4 | ~5 GB | 10 GB | https://huggingface.co/TheBloke/Llama-3-8B-Instruct-GGUF |
| Llama 3 70B Q4 | ~40 GB | 48 GB | https://huggingface.co/TheBloke/Llama-3-70B-Instruct-GGUF |

> Not sure which to pick? Start with **Llama 3.2 3B Q4**: it runs on almost any machine and is a good baseline.

Browse all GGUF models: https://huggingface.co/models?library=gguf

### Step 3: Create a .env File

Create a file called `.env` in the folder where you will run the server. This tells the server where your files are.

**macOS / Linux:**
```env
LLAMA_BIN=/usr/local/bin/llama-server
MODEL_PATH=/home/yourname/.offline-intelligence/models/llama-3.2-3b-instruct-q4_k_m.gguf
API_HOST=127.0.0.1
API_PORT=9999
```

**Windows:**
```env
LLAMA_BIN=C:\llama\llama-server.exe
MODEL_PATH=C:\models\llama-3.2-3b-instruct-q4_k_m.gguf
API_HOST=127.0.0.1
API_PORT=9999
```

Everything else (GPU layers, thread count, memory limits) is detected automatically.

### Step 4: Start the Server

```bash
cargo install offline-intelligence
offline-intelligence
```

You should see:
```
Starting with thread-based architecture
Memory database initialized
Model manager initialized successfully
Starting server on 127.0.0.1:9999
```

Verify it is running:
```bash
curl http://127.0.0.1:9999/healthz
```
Expected response: `{"status":"ok"}`

> **Note:** The server must be running before you use any of the language clients below.

### Step 5: Use Any Language Client

With the server running on port 9999, pick the language you want:

```python
pip install offline-intelligence==0.1.5
```
```javascript
npm install offline-intelligence@0.1.5
```
```bash
cargo add offline-intelligence@0.1.5
```

See the [Language Usage Guide](#language-usage-guide) for full examples in each language.

---

## Installation

### All Package Managers

**Rust (Cargo):**
```bash
cargo add offline-intelligence@0.1.5
```

**Python (PyPI):**
```bash
pip install offline-intelligence==0.1.5
```

**JavaScript / Node.js (npm):**
```bash
npm install offline-intelligence@0.1.5
```

**Java (JitPack):**
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
    <version>v0.1.5</version>
</dependency>
```

Gradle:
```gradle
repositories { maven { url 'https://jitpack.io' } }
dependencies { implementation 'com.github.OfflineIntelligence:offline-intelligence:v0.1.5' }
```

**C++ (CMake FetchContent, recommended):**
```cmake
include(FetchContent)
FetchContent_Declare(
    offline_intelligence
    GIT_REPOSITORY https://github.com/OfflineIntelligence/offline-intelligence.git
    GIT_TAG        v0.1.5
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(offline_intelligence)
target_link_libraries(your_target PRIVATE offline_intelligence)
```

**C++ (Conan):**
```bash
conan install --requires="offline-intelligence/0.1.5" --build=missing
```

**C++ (Manual):** Copy `bindings/cpp/include/offline_intelligence/offline_intelligence.hpp` into your project. Requires `cpp-httplib` and `nlohmann/json` headers.

---

## Language Usage Guide

> **Important:** The Rust crate **is** the server. Every other language binding (Python, JavaScript, Java, C++) is an HTTP client that talks to the Rust server over port 9999. You must start the server first before using any non-Rust client.

### Rust

In Rust, you embed the server directly in your application.

```rust
use offline_intelligence::{config::Config, run_thread_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;
    run_thread_server(cfg, None).await
}
```

Custom configuration:
```rust
use offline_intelligence::{config::Config, run_thread_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cfg = Config::from_env()?;
    cfg.api_host  = "0.0.0.0".to_string();
    cfg.api_port  = 9999;
    cfg.model_path = "/path/to/model.gguf".to_string();
    cfg.gpu_layers = 35;
    run_thread_server(cfg, None).await
}
```

### Python

```bash
pip install offline-intelligence==0.1.5
```

```python
from offline_intelligence import OfflineIntelligence, Config
import requests

cfg = Config.from_env()
ai  = OfflineIntelligence(cfg)

print(ai.health_check())

response = ai.generate("Explain quantum computing in simple terms")
print(response)

for chunk in ai.generate_stream("Write a short poem about the ocean"):
    print(chunk, end="", flush=True)

convs = ai.get_conversations()
title = ai.generate_title(session_id="abc123", first_message="Tell me about space")

stats = ai.get_memory_stats("abc123")
ai.optimize_memory()

settings = requests.get("http://127.0.0.1:9999/tools/settings").json()
requests.post("http://127.0.0.1:9999/mode", json={"mode": "online"})
requests.post("http://127.0.0.1:9999/api-keys", json={"key_type": "openrouter", "value": "sk-or-..."})
requests.post("http://127.0.0.1:9999/feedback", json={"message": "Great!"})
```

Custom configuration:
```python
from offline_intelligence import Config, OfflineIntelligence

cfg = Config()
cfg.api_host        = "127.0.0.1"
cfg.api_port        = 9999
cfg.backend_url     = "http://127.0.0.1:8081"
cfg.openrouter_api_key = "sk-or-..."

ai = OfflineIntelligence(cfg)
```

### JavaScript / Node.js

```bash
npm install offline-intelligence@0.1.5
```

```javascript
const { OfflineIntelligence, Config } = require('offline-intelligence');

const cfg = Config.fromEnv();
const ai  = new OfflineIntelligence(cfg);

async function main() {
    const health = await ai.healthCheck();
    console.log(health);

    const response = await ai.generate('What is machine learning?');
    console.log(response);

    await ai.generateStream('Tell me a story', chunk => process.stdout.write(chunk));

    const convs = await ai.getConversations();
    const title = await ai.generateTitle('abc123', 'Tell me about black holes');

    const stats = await ai.getMemoryStats('abc123');
    await ai.optimizeMemory();
    await ai.cleanupMemory();

    await ai.loadModel('/path/to/model.gguf');
    await ai.stopModel();
}

main().catch(console.error);
```

Custom configuration:
```javascript
const { Config, OfflineIntelligence } = require('offline-intelligence');

const cfg = new Config();
cfg.apiHost           = '127.0.0.1';
cfg.apiPort           = 9999;
cfg.backendUrl        = 'http://127.0.0.1:8081';
cfg.openrouterApiKey  = 'sk-or-...';

const ai = new OfflineIntelligence(cfg);
```

### Java

```xml
<dependency>
    <groupId>com.github.OfflineIntelligence</groupId>
    <artifactId>offline-intelligence</artifactId>
    <version>v0.1.5</version>
</dependency>
```

```java
import com.offlineintelligence.OfflineIntelligence;
import com.offlineintelligence.Config;

public class Main {
    public static void main(String[] args) throws Exception {
        Config cfg = Config.fromEnv();
        OfflineIntelligence ai = new OfflineIntelligence(cfg);

        System.out.println(ai.healthCheck());
        System.out.println(ai.generate("Summarize the theory of relativity"));
        ai.generateStream("Write a haiku", chunk -> System.out.print(chunk));
        System.out.println(ai.getConversations());
        System.out.println(ai.generateTitle("abc123", "Tell me about space"));
        System.out.println(ai.getMemoryStats("abc123"));
        ai.optimizeMemory();
    }
}
```

Custom configuration:
```java
Config cfg = new Config();
cfg.setApiHost("127.0.0.1");
cfg.setApiPort(9999);
cfg.setBackendUrl("http://127.0.0.1:8081");
cfg.setOpenrouterApiKey("sk-or-...");

OfflineIntelligence ai = new OfflineIntelligence(cfg);
```

### C++

**CMake FetchContent (recommended):**
```cmake
FetchContent_Declare(
    offline_intelligence
    GIT_REPOSITORY https://github.com/OfflineIntelligence/offline-intelligence.git
    GIT_TAG        v0.1.5
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(offline_intelligence)
target_link_libraries(your_target PRIVATE offline_intelligence)
```

```cpp
#include <offline_intelligence/offline_intelligence.hpp>
#include <iostream>

int main() {
    offline_intelligence::Config cfg;
    cfg.api_host = "127.0.0.1";
    cfg.api_port = 9999;

    offline_intelligence::OfflineIntelligence ai(cfg);

    auto health = ai.health_check();
    std::cout << health.dump(2) << std::endl;

    auto response = ai.generate("What is the capital of France?");
    std::cout << response.dump(2) << std::endl;

    ai.generate_stream("Write a short story", [](const std::string& chunk) {
        std::cout << chunk << std::flush;
    });

    auto convs = ai.get_conversations();
    std::cout << convs.dump(2) << std::endl;

    return 0;
}
```

---

## Configuration

### Environment Variables

All configuration is set in your `.env` file (or as system environment variables). The server reads this file automatically on startup.

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `LLAMA_BIN` | **Yes** | (required) | Full path to the `llama-server` binary |
| `MODEL_PATH` | **Yes** | (required) | Full path to your `.gguf` model file |
| `API_HOST` | No | `127.0.0.1` | IP address the server listens on. Use `0.0.0.0` to allow connections from other devices |
| `API_PORT` | No | `9999` | Port the server listens on |
| `LLAMA_HOST` | No | `127.0.0.1` | Host for the llama-server subprocess |
| `LLAMA_PORT` | No | `8081` | Port for the llama-server subprocess |
| `BACKEND_URL` | No | auto | Full URL to llama-server (e.g. `http://127.0.0.1:8081`) |
| `OPENROUTER_API_KEY` | No | none | OpenRouter key for online/cloud mode |
| `CTX_SIZE` | No | auto | Context window size in tokens (e.g. `8192`) |
| `BATCH_SIZE` | No | auto | Batch size for prompt processing (e.g. `512`) |
| `THREADS` | No | auto | Number of CPU threads to use |
| `GPU_LAYERS` | No | auto | How many model layers to offload to GPU |
| `MAX_CONCURRENT_STREAMS` | No | `4` | Max simultaneous streaming requests |
| `REQUESTS_PER_SECOND` | No | `24` | Rate limit in requests per second |
| `PROMETHEUS_PORT` | No | `9000` | Port for the Prometheus metrics endpoint |

### Auto-Detection

If you leave `THREADS`, `GPU_LAYERS`, `CTX_SIZE`, and `BATCH_SIZE` blank, the server detects your hardware and sets sensible values automatically.

**CPU threads** are set based on core count:

| CPU Cores | Threads used |
|-----------|-------------|
| 1–2 | 1 |
| 3–8 | 60% of cores |
| 9–16 | 50% of cores |
| 17–32 | 40% of cores |
| 32+ | 16 (max) |

**GPU layers** are set based on VRAM (NVIDIA) or unified memory (Apple Silicon):

| VRAM | GPU layers |
|------|-----------|
| 0–4 GB | 12 |
| 5–8 GB | 20 |
| 9–12 GB | 32 |
| 13–16 GB | 40 |
| 16 GB+ | 50 |
| Apple Silicon (Metal) | 24–56 based on unified memory |
| Intel Mac | 0 (CPU only) |

### Platform-Specific Examples

**Windows:**
```env
LLAMA_BIN=C:\llama\llama-server.exe
MODEL_PATH=C:\models\your-model.gguf
API_HOST=127.0.0.1
API_PORT=9999
MAX_CONCURRENT_STREAMS=2
```

**macOS (Apple Silicon):**
```env
LLAMA_BIN=/usr/local/bin/llama-server
MODEL_PATH=/Users/yourname/models/your-model.gguf
API_HOST=127.0.0.1
API_PORT=9999
```

**Linux server (allow external connections):**
```env
LLAMA_BIN=/usr/local/bin/llama-server
MODEL_PATH=/home/user/models/your-model.gguf
API_HOST=0.0.0.0
API_PORT=9999
PROMETHEUS_PORT=9000
MAX_CONCURRENT_STREAMS=8
REQUESTS_PER_SECOND=48
```

### Hardware-Specific Tuning

**CPU-only machine:**
```env
GPU_LAYERS=0
THREADS=8
CTX_SIZE=4096
BATCH_SIZE=128
```

**4 GB VRAM GPU:**
```env
GPU_LAYERS=12
CTX_SIZE=2048
BATCH_SIZE=64
```

**8 GB VRAM GPU:**
```env
GPU_LAYERS=25
CTX_SIZE=4096
BATCH_SIZE=256
```

**12 GB+ VRAM GPU:**
```env
GPU_LAYERS=40
CTX_SIZE=8192
BATCH_SIZE=512
```

---

## API Reference

The server exposes a REST API on `http://127.0.0.1:9999` (or wherever you configured it). All request and response bodies are JSON.

### Core

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/generate/stream` | Stream an AI response token by token (SSE) |
| `GET` | `/healthz` | Health check. Returns `{"status":"ok"}` |
| `GET` | `/readyz` | Readiness check. Returns backend and model status |
| `GET` | `/metrics` | Prometheus metrics |

**POST /generate/stream** request body:
```json
{
  "messages":   [{"role": "user", "content": "Hello"}],
  "session_id": "abc123",
  "temperature": 0.7,
  "max_tokens": 512
}
```

### Admin

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/admin/status` | Server status, uptime, request counts |
| `POST` | `/admin/load` | Load a model: `{"model_path": "...", "ctx_size": 8192}` |
| `POST` | `/admin/stop` | Stop the llama-server backend |
| `POST` | `/admin/cleanup` | Clean up expired sessions |
| `POST` | `/admin/optimize` | Run SQLite WAL checkpoint and PRAGMA optimize |

### Memory

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/memory/stats/{session_id}` | Message count, token count, storage size for a session |
| `POST` | `/memory/optimize` | Optimize memory usage across all sessions |
| `POST` | `/memory/cleanup` | Remove stale entries |

### Conversations

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/conversations` | List all conversations |
| `GET` | `/conversations/{id}` | Get a single conversation with all messages |
| `DELETE` | `/conversations/{id}` | Delete a conversation |
| `PATCH` | `/conversations/{id}` | Update title or pinned status |
| `POST` | `/generate/title` | Generate a title: `{"session_id": "...", "first_message": "..."}` |

### Models

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/models` | List available models |
| `GET` | `/models/active` | Currently loaded model |
| `POST` | `/models/install` | Download a model from HuggingFace |
| `DELETE` | `/models/{id}` | Remove a local model |
| `GET` | `/hardware` | Detected hardware info (CPU, GPU, RAM) |

### Authentication (v0.1.5)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/auth/register` | Register: `{"username": "...", "email": "...", "password": "..."}` |
| `POST` | `/auth/login` | Login. Returns a JWT token in the response body |
| `GET` | `/auth/verify?token=...` | Verify email address |
| `GET` | `/auth/google?redirect_uri=...` | Start Google OAuth 2.0 flow |

### API Key Management (v0.1.5)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api-keys` | Save key: `{"key_type": "huggingface"/"openrouter", "value": "..."}` |
| `GET` | `/api-keys?key_type=...` | Retrieve a stored key (returned decrypted) |
| `DELETE` | `/api-keys?key_type=...` | Delete a stored key |

### Web Tools (v0.1.5)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/tools/settings` | `{"enabled": true, "has_brave_key": false}` |
| `POST` | `/tools/settings` | `{"enabled": true, "brave_key": "optional"}` |

### Mode Switching (v0.1.5)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/mode` | `{"mode": "offline"}` or `{"mode": "online"}` |

### Feedback (v0.1.5)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/feedback` | `{"message": "...", "email": "optional"}` |

### Search

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/search` | Semantic search over conversation history |

---

## Benchmarks

> Measured directly on real hardware. No estimated or extrapolated numbers.

### Test Environment

| Component | Detail |
|-----------|--------|
| **GPU** | NVIDIA GeForce RTX 3050 Ti Laptop, 4 GB GDDR6 |
| **CPU** | Intel Core i7-11800H, 6 physical / 12 logical cores |
| **RAM** | 15.7 GB |
| **OS** | Windows 11 |
| **CUDA** | 12.4 / Driver 572.60 |
| **Engine** | llama.cpp build b8037 |
| **Model** | Qwen2.5-Coder-3B-Instruct Q4_K_M (1.924 GB) |
| **Date** | 2026-03-30 |

### What the Numbers Mean

Two things are measured:

- **Token generation (tg):** how fast the model produces output tokens. This is the response speed you feel as a user. Measured in tokens per second (T/s). Higher is better.
- **Prompt processing (pp):** how fast the model reads and processes your input (the system prompt, conversation history, attached files). This determines your Time to First Token (TTFT). Measured in tokens per second (T/s). Higher is better.

### OI Optimized vs Bare Baseline

Two configurations were tested on identical hardware and model.

**Config A: OI Optimized** (flags: `--flash-attn --n-gpu-layers 28 --cache-type-k q8_0 --cache-type-v q8_0 --ubatch-size 1024 --threads 6`)

These are the exact flags the OI server passes to llama-server in production.

| Test | Input size | Speed |
|------|-----------|-------|
| Token generation | 128 tokens | 45.22 T/s |
| Token generation | 256 tokens | 44.72 T/s |
| Token generation | 512 tokens | 43.34 T/s |
| **tg average** | (all sizes) | **44.43 T/s** |
| Prompt processing | 128 token prompt | 1,397 T/s |
| Prompt processing | 512 token prompt | 2,252 T/s |
| Prompt processing | 1,024 token prompt | 2,402 T/s |
| **pp average** | (all sizes) | **2,017 T/s** |

**Config B: Bare GPU Only** (flags: `--n-gpu-layers 28` only)

This is what Ollama, LM Studio, and Jan.ai ship by default. Same engine, minimal configuration.

| Test | Input size | Speed |
|------|-----------|-------|
| Token generation | 128 tokens | 43.01 T/s |
| Token generation | 256 tokens | 42.69 T/s |
| Token generation | 512 tokens | 41.66 T/s |
| **tg average** | (all sizes) | **42.45 T/s** |
| Prompt processing | 128 token prompt | 1,275 T/s |
| Prompt processing | 512 token prompt | 1,726 T/s |
| Prompt processing | 1,024 token prompt | 1,626 T/s |
| **pp average** | (all sizes) | **1,542 T/s** |

**OI's gain over the bare baseline:**
- Token generation: +1.98 T/s (+4.7%)
- Prompt processing: +475 T/s (**+30.8%**)

The prompt processing gain is where users actually feel the difference. Every system prompt, conversation history, and attached file goes through pp before the first response token appears.

### Comparison with Other Tools

Hardware baseline: **RTX 3050 / RTX 3060 class GPU, 3B–7B Q4 model, single user, Windows PC.**

OI numbers are directly measured. All other numbers are from published documentation, GitHub benchmarks, and community reports (sources below).

| Tool | tg T/s | pp T/s | Notes |
|------|:------:|:------:|-------|
| **OI SDK (Optimized)** | **44.4** | **2,017** | Measured 2026-03-30, Flash Attention + KV quant + cont-batching |
| **OI SDK (Bare baseline)** | **42.5** | **1,542** | Measured 2026-03-30, GPU only, no extras |
| llama.cpp direct (tuned) | ~43–46 | ~1,800–2,400 | User-configured, can match or exceed OI |
| ExLlamaV2 | 35–55 | N/A | Custom CUDA kernels, EXL2 format only (not GGUF) |
| Ollama | 30–42 | ~900–1,300 | Conservative defaults, no flash-attn |
| LM Studio | 28–40 | ~800–1,200 | Electron wrapper adds ~2–5ms/token overhead |
| Jan.ai | 26–38 | ~800–1,100 | Electron wrapper, conservative flags |
| Text Gen WebUI | 25–40 | ~800–1,500 | Python/Gradio overhead, backend-dependent |
| llama-cpp-python | 22–38 | ~700–1,200 | `logits_all=True` default hurts performance |
| GPT4All | 4–10 | ~200–400 | GPU acceleration not well-optimized, CPU-primary |
| AirLLM (3B model) | ~1–5 | N/A | Layer-by-layer VRAM management, designed for running 70B+ models on 4GB GPUs |

### Why OI's Prompt Processing Advantage Matters

Flash Attention reduces the memory traffic for processing long inputs from O(n²) to O(n). In practical terms:

- A 512-token system prompt: **0.23s** through OI vs **~0.47s** through Ollama
- A 1,024-token conversation history: **0.43s** through OI vs **~0.93s** through Ollama

This difference accumulates on every single request. If you have any non-trivial system prompt or conversation history, TTFT is dominated by prompt processing, not token generation.

### Multi-User Throughput (Continuous Batching)

OI enables `--parallel 8` continuous batching. Without it, requests are queued and each user waits for the previous one to finish. With it, all 8 users share a single GPU pass every decode step.

| Setup | 8-user aggregate throughput |
|-------|----------------------------|
| OI (continuous batching on) | ~74 T/s measured |
| Ollama / LM Studio / Jan.ai | ~42 T/s (single-user speed, others wait in queue) |
| OI advantage | **~1.76× more total output** for the same GPU |

### Understanding the Other Tools

**Ollama, LM Studio, Jan.ai, Text Gen WebUI, and OI all use the same llama.cpp engine.** The math is identical, the CUDA kernels are identical, the GGUF files are identical. Performance differences come entirely from which flags are passed and how much overhead the wrapper adds.

**ExLlamaV2** is the exception. It has hand-written CUDA kernels and EXL2 quantization, which is why it can exceed llama.cpp at single-user throughput. It requires models in EXL2 format (not GGUF; a separate conversion step is needed). OI's speculative decoding using a 0.5B draft model narrows this gap significantly.

**AirLLM** solves a different problem entirely: running a 70B model on a 4 GB GPU by loading one transformer layer at a time. Speed is 1–5 T/s because each token requires many disk reads. It is not useful for a 3B model that fits entirely in VRAM.

**GPT4All** uses llama.cpp internally but GPU acceleration is not well-optimized in its default builds. It is primarily a CPU inference tool with a polished desktop UI.

### Sources

| Tool | Source |
|------|--------|
| Ollama (RTX 3060/3060 Ti) | [DatabaseMart RTX 3060 Ti benchmark](https://www.databasemart.com/blog/ollama-gpu-benchmark-rtx3060ti) · [LinkedIn M4 Pro vs RTX 3060](https://www.linkedin.com/pulse/benchmarking-local-ollama-llms-apple-m4-pro-vs-rtx-3060-dmitry-markov-6vlce) |
| LM Studio | [NVIDIA RTX AI Garage × LM Studio](https://blogs.nvidia.com/blog/rtx-ai-garage-lmstudio-llamacpp-blackwell/) · [InsiderLLM speed gap analysis](https://insiderllm.com/guides/lm-studio-vs-llamacpp-speed-gap/) |
| Jan.ai | [Jan.ai benchmarking methodology](https://www.jan.ai/post/how-we-benchmark-kernels) · [Jan.ai TensorRT-LLM results](https://www.jan.ai/post/benchmarking-nvidia-tensorrt-llm) |
| ExLlamaV2 | [ExLlamaV2 GitHub](https://github.com/turboderp-org/exllamav2) · [Towards Data Science review](https://towardsdatascience.com/exllamav2-the-fastest-library-to-run-llms-32aeda294d26/) |
| llama-cpp-python overhead | [GitHub issue #398](https://github.com/abetlen/llama-cpp-python/issues/398) |
| Text Gen WebUI | [oobabooga GitHub](https://github.com/oobabooga/text-generation-webui) community benchmarks |
| GPT4All | Community reports |
| AirLLM | [AirLLM GitHub](https://github.com/lyogavin/airllm) · [Towards AI writeup](https://pub.towardsai.net/run-70b-llms-on-4gb-gpu-with-airllm-795185975f3b) |
| OI SDK results | **Directly measured**, see `benchmarks/results/llama_bench_20260330_210218.json` |

To run the benchmarks yourself:
```bash
python benchmarks/llama_bench.py

python benchmarks/llama_bench.py \
    --llama-bench /path/to/llama-bench \
    --model /path/to/model.gguf \
    --reps 5
```

Results are saved as timestamped JSON files in `benchmarks/results/`.

---

## Architecture

The library is a Rust workspace. The core crate lives in `crates/offline-intelligence/src/`.

### Request Flow

```
Client (Python / JS / Java / C++)
        ↓  HTTP POST /generate/stream
API Gateway (Axum, port 9999)
        ↓  Auth check, rate limit, queue
Web Tools detector  →  [optional] fetch weather / currency / crypto
        ↓  inject live data as system context
LLM Worker thread
        ↓  forwards to llama-server (port 8081)
llama-server (llama.cpp)
        ↓  SSE stream of tokens
Response streamed back to client
        ↓
Database Worker  →  store messages to SQLite
Cache Worker     →  update KV cache index
```

### Module Map

```
src/
├── api/                    HTTP endpoint handlers
│   ├── stream_api.rs       POST /generate/stream  (SSE)
│   ├── conversation_api.rs GET/DELETE /conversations
│   ├── title_api.rs        POST /generate/title
│   ├── memory_api.rs       GET /memory/stats, optimize, cleanup
│   ├── model_api.rs        GET /models, install, remove
│   ├── admin_api.rs        GET /admin/status, load, stop
│   ├── auth_api.rs         POST /auth/register, login, Google OAuth  (v0.1.5)
│   ├── api_keys_api.rs     POST/GET/DELETE /api-keys  (v0.1.5)
│   ├── tools_api.rs        GET/POST /tools/settings  (v0.1.5)
│   ├── mode_api.rs         POST /mode  (offline / online)
│   ├── feedback_api.rs     POST /feedback  (v0.1.5)
│   ├── files_api.rs        File upload and retrieval
│   ├── attachment_api.rs   Attachment handling  (v0.1.5)
│   ├── all_files_api.rs    All-files management  (v0.1.5)
│   ├── search_api.rs       POST /search
│   └── online_api.rs       OpenRouter passthrough
│
├── tools/                  Live data injection  (v0.1.5)
│   ├── detector.rs         Intent detection (weather / currency / crypto)
│   ├── weather.rs          Open-Meteo + Nominatim geocoding
│   └── currency.rs         ExchangeRate-API fiat + CoinGecko crypto
│
├── memory_db/              SQLite database layer
│   ├── conversation_store.rs
│   ├── embedding_store.rs  HNSW ANN index (lazy dirty-flag rebuild)
│   ├── users_store.rs      User accounts, Argon2 hashes  (v0.1.5)
│   ├── api_keys_store.rs   Encrypted key storage  (v0.1.5)
│   ├── all_files_store.rs  (v0.1.5)
│   ├── local_files_store.rs  (v0.1.5)
│   ├── session_file_contexts_store.rs  (v0.1.5)
│   ├── session_summaries_store.rs  (v0.1.5)
│   └── schema.rs
│
├── cache_management/       KV cache layer
│   ├── cache_manager.rs    Lifecycle + sysinfo-based memory limits
│   ├── cache_scorer.rs     Content-aware importance scoring
│   ├── cache_bridge.rs     Cache → database bridge
│   └── llama_cache_interface.rs   GET/POST /slots HTTP API
│
├── context_engine/         Context assembly for each request
│   ├── context_builder.rs
│   ├── orchestrator.rs
│   └── retrieval_planner.rs
│
├── worker_threads/         Background worker threads
│   ├── llm_worker.rs       LLM inference
│   ├── context_worker.rs   Context processing
│   ├── cache_worker.rs     Cache management
│   └── database_worker.rs  Database I/O
│
├── model_management/       Model download and registry
│   ├── downloader.rs       HuggingFace download
│   ├── registry.rs         Local + remote model catalog
│   ├── recommendation.rs   Hardware-aware model suggestions
│   └── storage.rs
│
├── model_runtime/          Multi-format runtime support
│   ├── format_detector.rs  Auto-detect GGUF / ONNX / TRT / etc.
│   └── platform_detector.rs
│
├── engine_management/      llama-server binary management
│   ├── downloader.rs       Auto-download llama-server
│   └── registry.rs
│
├── shared_state.rs         Arc-based unified application state
├── backend_target.rs       Lock-free backend URL switching (arc-swap)
├── thread_server.rs        Server entry point
├── config.rs               Configuration + auto-detection
└── lib.rs                  Public exports
```

---

## Security

### What Is Secure by Default

- **Local only by default.** The server binds to `127.0.0.1` and is not reachable from other machines unless you set `API_HOST=0.0.0.0`.
- **Memory-safe.** The server is written in Rust, which prevents buffer overflows and use-after-free vulnerabilities.
- **No external calls by default.** Web tools are the only source of outbound HTTP calls, and they are off by default unless a user message triggers a recognized intent.
- **Passwords never stored in plaintext.** User passwords are hashed with Argon2 before storage.
- **API keys never stored in plaintext.** HuggingFace and OpenRouter keys are encrypted with a machine-specific key before being written to SQLite.

### Recommendations for Production Use

- Do not bind to `0.0.0.0` without a firewall or reverse proxy in front
- Enable rate limiting (`REQUESTS_PER_SECOND`) to protect against abuse
- Use HTTPS via a reverse proxy (nginx, Caddy) if exposing the server over a network
- Set `MAX_CONCURRENT_STREAMS` based on expected load

---

## Troubleshooting

### The server will not start

**`LLAMA_BIN` is wrong or missing:**
```
Error: No such file or directory (os error 2)
```
Open your `.env` file and check that `LLAMA_BIN` points to the exact binary. On Windows the file is `llama-server.exe`, not `llama-server`.

**`MODEL_PATH` is wrong:**
```
Error: failed to load model from ...
```
Double-check that the path in `MODEL_PATH` leads directly to a `.gguf` file. Spaces in the path are fine as long as you do not add extra quotes inside the `.env` file.

**Port already in use:**
```
Error: Address already in use (os error 98)
```
Something else is on port 9999. Either stop the other process or add `API_PORT=9998` (or any free port) to your `.env`.

---

### The model is loading but responses are very slow

The model is probably running on CPU only. Check whether GPU layers are being used:
```bash
curl http://127.0.0.1:9999/hardware
```
If `gpu_layers` is 0 and you have a CUDA-capable GPU, make sure:
1. You downloaded the CUDA build of llama-server (the zip name includes `cuda` or `cublas`)
2. Your NVIDIA driver is up to date (CUDA 12.x requires driver 525+)
3. You have not explicitly set `GPU_LAYERS=0` in your `.env`

---

### CUDA errors at startup

```
CUDA error: no kernel image is available for execution on the device
```
You downloaded a llama-server binary compiled for a different CUDA compute capability. Download the matching build from the llama.cpp releases page (e.g. `cu121` for CUDA 12.1).

```
CUDA error: out of memory
```
The model does not fit in your GPU VRAM at the current `GPU_LAYERS` setting. Lower it by adding `GPU_LAYERS=8` (or any value smaller than what auto-detection chose) to your `.env`.

---

### Out of system RAM

```
ggml_backend_alloc_ctx_tensors: not enough memory
```
Your model is too large for the RAM available. Options:
- Use a smaller model (e.g. Q3_K_S instead of Q4_K_M)
- Lower `CTX_SIZE` to `2048` or `1024`
- Set `BATCH_SIZE=64`

---

### Responses cut off or incomplete

The model is hitting the `max_tokens` limit. In your request body, increase `max_tokens`:
```json
{
  "messages": [{"role": "user", "content": "..."}],
  "max_tokens": 2048
}
```

---

### `cargo install offline-intelligence` fails to compile

On Windows, the linker sometimes fails on first install due to PDB locking. Try:
```bash
cargo install offline-intelligence --locked
```
If you are on Linux and see missing library errors (`libssl`, `libsqlite3`), install them:
```bash
sudo apt install libssl-dev libsqlite3-dev pkg-config   # Debian / Ubuntu
sudo dnf install openssl-devel sqlite-devel              # Fedora / RHEL
```

---

### `401 Unauthorized` on API calls

Protected endpoints require a JWT token in the `Authorization` header. Log in first:
```bash
curl -X POST http://127.0.0.1:9999/auth/login \
     -H "Content-Type: application/json" \
     -d '{"email": "you@example.com", "password": "yourpassword"}'
```
Copy the returned token and pass it on subsequent requests:
```bash
curl http://127.0.0.1:9999/conversations \
     -H "Authorization: Bearer <your-token>"
```

---

### Live web tools are not triggering

Web tools are enabled by default but require the user message to match a recognized intent. The trigger is keyword-based:
- Weather: include a city name and "weather", "temperature", "forecast", or similar
- Currency: include an amount, a currency code (USD, EUR, BTC), and "convert" or "to"
- Crypto: include a coin name and "price", "worth", "value", or similar

Check that tools are enabled:
```bash
curl http://127.0.0.1:9999/tools/settings
```
If `"enabled": false`, enable them:
```bash
curl -X POST http://127.0.0.1:9999/tools/settings \
     -H "Content-Type: application/json" \
     -d '{"enabled": true}'
```

---

## FAQ

**Does it work without a GPU?**
Yes. Set `GPU_LAYERS=0` in your `.env` and the model runs entirely on CPU. Expect roughly 3–8 T/s on a modern 8-core CPU with a 3B model, compared to 40+ T/s with a GPU. For everyday use on CPU, stick to models at or below 3B parameters.

---

**What model formats are supported?**
The primary format is GGUF (used by llama.cpp). The library also accepts GGML, ONNX, SafeTensors, CoreML (macOS), and TensorRT (`.engine`) files, though llama.cpp itself only runs GGUF natively. Other formats go through the relevant runtime.

---

**Can I connect my own frontend or UI to this?**
Yes. The server is a plain HTTP API on port 9999. Any tool that can make HTTP requests works: a web app, a mobile app, Postman, curl, or anything else. The streaming endpoint (`POST /generate/stream`) uses Server-Sent Events (SSE), which is supported natively in all modern browsers.

---

**Is my data private?**
All inference runs locally on your machine. No conversation data, prompts, or responses are sent anywhere. The only outbound network calls are:
- Web tools (weather, currency, crypto) — only when triggered by a matching user message, and all use keyless public APIs
- OpenRouter (only if you explicitly switch to online mode via `POST /mode`)

---

**What is the difference between offline mode and online mode?**
In offline mode (the default), every request is processed by the local llama-server subprocess using your local GGUF model. In online mode, requests are forwarded to OpenRouter, giving you access to hosted models like GPT-4o, Claude, Gemini, and others. You need an OpenRouter API key for online mode. Switch between them at runtime without restarting the server:
```bash
POST /mode   {"mode": "offline"}
POST /mode   {"mode": "online"}
```

---

**Can multiple users use the server at the same time?**
Yes. The server enables `--parallel 8` continuous batching by default, so up to 8 concurrent streaming requests share a single GPU pass per decode step. This gives roughly 1.76× more total output throughput compared to a queued single-user setup. Raise `MAX_CONCURRENT_STREAMS` if you need more.

---

**How much disk space do I need?**
The server binary itself is small (a few MB). The space requirement is dominated by the model:

| Model size | Disk space |
|-----------|-----------|
| 3B Q4 | ~2 GB |
| 7B Q4 | ~4 GB |
| 13B Q4 | ~8 GB |
| 34B Q4 | ~20 GB |
| 70B Q4 | ~40 GB |

SQLite databases for conversations and memory grow slowly. Expect a few MB per month for typical usage.

---

**Can I run multiple models and switch between them?**
Yes. Use the model API to install and switch models at runtime:
```bash
GET  /models              # list available models
GET  /models/active       # see which model is loaded
POST /admin/load          # load a different model: {"model_path": "...", "ctx_size": 8192}
POST /admin/stop          # stop the current model
```

---

**What happens if the web tool fetch fails or times out?**
The model answers from its training data as normal. No error is shown to the user. Each tool has an 8-second individual timeout, and there is a 10-second hard deadline across all tools combined. Failure is silent and graceful.

---

**How do I expose the server to other devices on my network?**
Change `API_HOST` in your `.env` to `0.0.0.0`:
```env
API_HOST=0.0.0.0
API_PORT=9999
```
The server will then accept connections from any device on your local network using your machine's IP address (e.g. `http://192.168.1.10:9999`). Do not expose port 9999 to the public internet without a reverse proxy and TLS.

---

## Contributing

```bash
git clone https://github.com/OfflineIntelligence/offline-intelligence.git
cd offline-intelligence
cargo build
cargo test
cargo test --test integration
cargo bench
```

Please follow standard Rust style guidelines (`cargo clippy`, `cargo fmt`). All public-facing API changes should maintain backward compatibility within a minor version.

---

## License

The core library (80% of functionality) is released under the **Apache 2.0 License**.

Advanced context management and enterprise features are available under a commercial license.

Third-party components: llama.cpp (MIT), Axum (MIT), Tokio (MIT), Serde (MIT/Apache 2.0), SQLite (Public Domain).

---

## Support

- **Bug reports:** [GitHub Issues](https://github.com/OfflineIntelligence/offline-intelligence/issues)
- **Questions:** [GitHub Discussions](https://github.com/OfflineIntelligence/offline-intelligence/discussions)
- **Enterprise support:** Contact us for priority support, custom development, and training

---

## Changelog

### v0.1.5 (2026-03-30)
- Web tools module: intent-driven live data injection covering weather (Open-Meteo + Nominatim), fiat currency (ExchangeRate-API, 160+ currencies), and crypto prices (CoinGecko). All keyless. Parallel execution with 8s per-tool and 10s hard deadline. Results injected as system context with `[N]` citation sources.
- Authentication: user registration and login, Argon2 password hashing, JWT session tokens, email verification, Google OAuth 2.0
- Encrypted API key management: HuggingFace and OpenRouter keys stored with machine-specific encryption in SQLite
- Tools settings API: `GET/POST /tools/settings` for enabling or disabling web tools and setting the Brave Search key
- Mode management: `POST /mode` for runtime offline/online switching without server restart
- Feedback endpoint: `POST /feedback` with optional admin email notification
- Login notification tracking
- File attachment API and all-files management API
- New database stores: users, API keys, all files, local files, session file contexts, session summaries

### v0.1.4 (2026-03-27)
- Lazy HNSW index rebuild: dirty-flag deferred rebuild eliminates per-insert O(n²) cost
- Content-aware message importance scoring: replaced hardcoded `0.5` with `score_message_importance()`. Role weights: system=0.9, assistant=0.6, user=0.4. Bonuses added for code blocks, key concepts, and message length
- Real llama-server KV cache integration: `GET /slots` for live token counts, `POST /slots/0` for erase/restore
- Token-bucket KV entries: slot sequences divided into 64-token buckets; earlier position = higher priority
- `sysinfo`-based memory limits: 25% of available RAM allocated to KV cache, clamped 256 MB–8 GB
- Database and cache workers fully wired
- Admin maintenance: `cleanup_expired_sessions`, `optimize_database` (PRAGMA optimize + WAL checkpoint)

### v0.1.3 (2026-03-22)
- Thread-based server architecture replacing single-threaded server
- All 4 language bindings rewritten as pure HTTP clients
- Multi-format model support: .gguf, .onnx, .trt, .engine, .safetensors, .ggml, .mlmodel
- New `backend_url` and `openrouter_api_key` config fields
- API port changed from 8000 to 9999
- New modules: model_management, model_runtime, engine_management, worker_threads
- New APIs: conversations CRUD, title generation, memory optimize/cleanup, mode switching
- Lock-free backend URL switching via `arc-swap`
- Platform-specific GPU detection: Apple Silicon Metal, NVIDIA NVML, CPU fallback

### v0.1.2 (2026-02-07)
- Automatic hardware detection
- Improved memory management
- Enhanced error handling
- Fixed critical security vulnerabilities

### v0.1.1 (2025-12-15)
- Initial public release with multi-language bindings, core LLM integration, and memory management system

---

## Citation

```
Offline Intelligence Library v0.1.5
Author: Offline Intelligence Team
URL: https://github.com/OfflineIntelligence/offline-intelligence
```
