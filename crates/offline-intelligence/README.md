# offline-intelligence

High-performance Rust library for offline AI inference with context management, three-tier memory architecture, and thread-based server.

## Features

- Thread-based async server (`run_thread_server`)
- Three-tier memory: hot Moka cache → SQLite summaries → full persistence
- KV cache management with snapshot/restore
- HNSW ANN indexing for semantic similarity
- Multi-format model support: `.gguf`, `.onnx`, `.trt`, `.safetensors`, `.ggml`, `.mlmodel`
- Platform-aware GPU detection: Apple Silicon Metal, NVIDIA (optional `nvidia` feature), CPU fallback
- OpenRouter API key support + local llama-server backend
- Prometheus metrics, JWT auth, Argon2 password hashing
- PDF/OCR extraction: lopdf (text), macOS Vision, Windows WinRT

## Quick Start

```toml
[dependencies]
offline-intelligence = "0.1.3"
```

```rust
use offline_intelligence::{config::Config, run_thread_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;
    run_thread_server(cfg, None).await
}
```

## Configuration

Set via `.env` or environment variables:

| Variable | Default | Description |
|---|---|---|
| `API_PORT` | `9999` | HTTP server port |
| `MODEL_PATH` | — | Path to model file |
| `LLAMA_BIN` | auto-detected | Path to llama-server binary |
| `GPU_LAYERS` | auto | GPU layers (auto-detects hardware) |
| `OPENROUTER_API_KEY` | — | OpenRouter API key |

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
