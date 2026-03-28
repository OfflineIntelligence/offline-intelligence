# offline-intelligence

High-performance Rust library for offline AI inference with context management, three-tier memory architecture, and thread-based server.

## Features

- Thread-based async server (`run_thread_server`)
- Three-tier memory: hot Moka cache → SQLite summaries → full persistence
- KV cache management: real llama-server `/slots` API integration, token-bucket entries, snapshot/restore
- HNSW ANN indexing with lazy dirty-flag rebuild (no per-insert rebuild cost)
- Content-aware message importance scoring (role + content bonuses, replaces hardcoded 0.5)
- Dynamic KV memory limits via `sysinfo` (25% of available RAM, 256 MB–8 GB)
- Multi-format model support: `.gguf`, `.onnx`, `.trt`, `.safetensors`, `.ggml`, `.mlmodel`
- Platform-aware GPU detection: Apple Silicon Metal, NVIDIA (optional `nvidia` feature), CPU fallback
- OpenRouter API key support + local llama-server backend
- Prometheus metrics, JWT auth, Argon2 password hashing
- PDF/OCR extraction: lopdf (text), macOS Vision, Windows WinRT
- Operational admin endpoints: session cleanup, SQLite optimize + WAL checkpoint

## Quick Start

```toml
[dependencies]
offline-intelligence = "0.1.4"
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
