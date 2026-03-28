//! Interface to llama.cpp's attention KV cache for infinite context management
//!
//! Connects to llama-server's `/slots` and `/health` HTTP endpoints to query and
//! control the running model's KV cache state. Slot save/restore (file-based) is
//! used for inject/extract because the HTTP API does not expose raw tensor data.

use crate::cache_management::cache_extractor::KVEntry;
use crate::cache_management::cache_scorer::{CacheEntryScorer, CacheEntryParams, CacheScoringConfig};
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

// --- llama-server response shapes ---

#[derive(Debug, Deserialize)]
struct SlotInfo {
    id: i32,
    /// 0 = idle, 1 = processing
    #[serde(default)]
    state: i32,
    /// Maximum context tokens for this slot
    #[serde(default)]
    n_ctx: usize,
    /// Tokens currently loaded in the KV cache for this slot
    #[serde(rename = "n_past", default)]
    n_past: usize,
}

#[derive(Debug, Serialize)]
struct SlotActionRequest {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

// --- Public types ---

/// Snapshot of llama-server's KV cache state for a slot
#[derive(Debug, Clone)]
pub struct LlamaKVCacheState {
    pub layer_count: usize,
    pub head_count: usize,
    pub kv_dim: usize,
    pub context_size: usize,
    pub current_tokens: usize,
    pub used_memory_bytes: usize,
    pub capacity_percentage: f32,
}

/// Interface for interacting with llama.cpp's KV cache via the llama-server HTTP API.
///
/// Construct with `LlamaKVCacheInterface::with_backend(url)` when the server URL is known.
/// The default constructor creates an instance with no URL, which falls back to safe defaults
/// for all read operations and no-ops for mutating operations.
pub struct LlamaKVCacheInterface {
    backend_url: Option<String>,
    http_client: reqwest::Client,
}

impl LlamaKVCacheInterface {
    pub fn new() -> Self {
        Self {
            backend_url: None,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Create an interface pre-wired to a running llama-server.
    pub fn with_backend(backend_url: String) -> Self {
        info!("LlamaKVCacheInterface wired to backend: {}", backend_url);
        Self {
            backend_url: Some(backend_url),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Return `Some(url)` for a given path if a backend URL is configured.
    fn url(&self, path: &str) -> Option<String> {
        self.backend_url.as_ref().map(|base| format!("{}{}", base.trim_end_matches('/'), path))
    }

    /// Fetch slot list from llama-server. Returns an empty list when the server
    /// is unavailable rather than propagating a connection error — callers should
    /// treat an empty list as "no active slots" and fall back gracefully.
    async fn fetch_slots(&self) -> Vec<SlotInfo> {
        let Some(url) = self.url("/slots") else { return Vec::new() };
        match self.http_client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<Vec<SlotInfo>>().await.unwrap_or_default()
            }
            Ok(resp) => {
                debug!("GET /slots returned {}: non-fatal", resp.status());
                Vec::new()
            }
            Err(e) => {
                debug!("GET /slots unreachable: {} — using defaults", e);
                Vec::new()
            }
        }
    }

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /// Get current KV cache state from llama-server.
    ///
    /// Queries `GET /slots` and returns data for slot 0 (the default interactive
    /// slot). Falls back to a zero-filled state when the server is unreachable.
    pub async fn get_current_cache_state(&self) -> anyhow::Result<LlamaKVCacheState> {
        let slots = self.fetch_slots().await;
        let slot = slots.into_iter().find(|s| s.id == 0);

        let (n_ctx, n_past) = slot
            .map(|s| (s.n_ctx, s.n_past))
            .unwrap_or((0, 0));

        let capacity_percentage = if n_ctx > 0 {
            n_past as f32 / n_ctx as f32
        } else {
            0.0
        };

        // Memory estimate: each token requires 2 (K+V) × layer_count × head_dim × 4 bytes.
        // Without model metadata in the HTTP response we use a conservative heuristic:
        // 32 layers × 128 head_dim × 2 × 4 = 32 768 bytes per token.
        let bytes_per_token = 32 * 128 * 2 * 4;
        let used_memory_bytes = n_past * bytes_per_token;

        debug!(
            "KV cache state: {}/{} tokens ({:.1}%)",
            n_past, n_ctx,
            capacity_percentage * 100.0
        );

        Ok(LlamaKVCacheState {
            layer_count: 32,
            head_count: 32,
            kv_dim: 128,
            context_size: n_ctx,
            current_tokens: n_past,
            used_memory_bytes,
            capacity_percentage,
        })
    }

    /// Extract current KV cache metadata from llama-server.
    ///
    /// The llama-server HTTP API does not expose raw tensor data; this method
    /// constructs representative `KVEntry` descriptors from slot state data so
    /// the rest of the cache management pipeline has something to work with.
    /// Importance scores are derived from token position — early tokens (system
    /// prompt, opening context) score higher than later ones.
    pub async fn extract_current_kv_entries(&self) -> anyhow::Result<Vec<KVEntry>> {
        let slots = self.fetch_slots().await;
        let slot = match slots.into_iter().find(|s| s.id == 0) {
            Some(s) if s.n_past > 0 => s,
            _ => return Ok(Vec::new()),
        };

        let scorer = CacheEntryScorer::new(CacheScoringConfig::default());
        let mut entries = Vec::new();

        // Divide the token sequence into buckets of 64 tokens each and represent
        // each bucket as a single KVEntry. Earlier buckets get higher importance.
        let bucket_size = 64usize;
        let n_buckets = (slot.n_past + bucket_size - 1) / bucket_size;

        for bucket_idx in 0..n_buckets {
            let token_start = bucket_idx * bucket_size;
            let token_end = (token_start + bucket_size).min(slot.n_past);

            // Earlier layers capture more structure — use layer 0 for high-importance anchors
            let layer_index = (bucket_idx % 32) as i32;
            let head_index = (bucket_idx % 8) as i32;

            // How far through the context is this bucket (0.0 = start, 1.0 = end)
            let position_fraction = token_start as f32 / slot.n_past as f32;
            // seconds_ago proxy: tokens at the start were "processed long ago"
            let last_accessed_seconds_ago = position_fraction * 3600.0;

            let key_hash = format!("slot0_bucket{}_tokens{}-{}", bucket_idx, token_start, token_end);

            let importance = scorer.score_entry(CacheEntryParams {
                key_hash: &key_hash,
                key_data: None,
                key_type: "attention_key",
                layer_index,
                head_index: Some(head_index),
                access_count: 1,
                last_accessed_seconds_ago,
                value_size_bytes: (token_end - token_start) * 128,
            });

            entries.push(KVEntry {
                key_hash,
                key_data: None,
                value_data: Vec::new(),
                key_type: "attention_key".to_string(),
                layer_index,
                head_index: Some(head_index),
                importance_score: importance,
                access_count: 1,
                last_accessed: chrono::Utc::now(),
                token_positions: Some((token_start as u32..token_end as u32).collect()),
                embedding: None,
                size_bytes: (token_end - token_start) * 128,
                is_persistent: false,
            });
        }

        debug!(
            "Extracted {} KV bucket entries from slot 0 ({} tokens)",
            entries.len(), slot.n_past
        );
        Ok(entries)
    }

    /// Restore the KV cache for slot 0 from a previously saved file.
    ///
    /// Uses llama-server's `POST /slots/0` with `{"action": "restore", "filename": "..."}`.
    pub async fn inject_kv_entries(&self, entries: &[KVEntry]) -> anyhow::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // The llama-server file-based restore API expects a filename that was
        // produced by a previous "save" action. We use the first entry's key_hash
        // as a logical identifier to locate the file.
        let filename = format!(
            "kvcache_{}.bin",
            entries.first().map(|e| e.key_hash.as_str()).unwrap_or("default")
        );

        let Some(url) = self.url("/slots/0") else {
            debug!("inject_kv_entries: no backend URL configured, skipping");
            return Ok(());
        };

        let body = SlotActionRequest {
            action: "restore".to_string(),
            filename: Some(filename.clone()),
        };

        match self.http_client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Restored KV cache from {} ({} entries)", filename, entries.len());
            }
            Ok(resp) => {
                // Non-fatal: server may not have the file or may not support the action
                warn!("KV restore returned {}: continuing without restored cache", resp.status());
            }
            Err(e) => {
                warn!("KV restore request failed: {} — continuing without restored cache", e);
            }
        }

        Ok(())
    }

    /// Erase the KV cache for slot 0 via `POST /slots/0` with `{"action": "erase"}`.
    ///
    /// `layer_indices` and `head_indices` are accepted for API compatibility but the
    /// llama-server erase action clears the entire slot (partial-layer erase is not
    /// supported via HTTP).
    pub async fn clear_cache_entries(
        &self,
        layer_indices: &[i32],
        _head_indices: &[Option<i32>],
    ) -> anyhow::Result<()> {
        let Some(url) = self.url("/slots/0") else {
            debug!("clear_cache_entries: no backend URL configured, skipping");
            return Ok(());
        };

        let body = SlotActionRequest {
            action: "erase".to_string(),
            filename: None,
        };

        match self.http_client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "Erased KV cache slot 0 (requested {} layer(s))",
                    layer_indices.len()
                );
            }
            Ok(resp) => {
                warn!("KV erase returned {}: slot may already be empty", resp.status());
            }
            Err(e) => {
                warn!("KV erase request failed: {}", e);
            }
        }

        Ok(())
    }

    /// Estimate memory used by the KV cache from slot state.
    ///
    /// Uses the same heuristic as `get_current_cache_state`: 32 KB per token.
    pub async fn get_cache_memory_usage(&self) -> anyhow::Result<usize> {
        let slots = self.fetch_slots().await;
        let n_past = slots.iter().find(|s| s.id == 0).map(|s| s.n_past).unwrap_or(0);
        let bytes_per_token = 32 * 128 * 2 * 4; // 32 layers × head_dim × K+V × f32
        Ok(n_past * bytes_per_token)
    }

    /// Estimate what fraction of the context window is filled (0.0–1.0).
    pub async fn estimate_cache_capacity(&self) -> anyhow::Result<f32> {
        let slots = self.fetch_slots().await;
        if let Some(slot) = slots.iter().find(|s| s.id == 0) {
            if slot.n_ctx > 0 {
                return Ok((slot.n_past as f32 / slot.n_ctx as f32).clamp(0.0, 1.0));
            }
        }
        Ok(0.0)
    }
}

impl Default for LlamaKVCacheInterface {
    fn default() -> Self {
        Self::new()
    }
}
