
use crate::cache_management::cache_extractor::KVEntry;
use crate::cache_management::cache_scorer::{CacheEntryScorer, CacheEntryParams, CacheScoringConfig};
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct SlotInfo {
    id: i32,
    
    #[serde(default)]
    state: i32,
    
    #[serde(default)]
    n_ctx: usize,
    
    #[serde(rename = "n_past", default)]
    n_past: usize,
}

#[derive(Debug, Serialize)]
struct SlotActionRequest {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

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

    fn url(&self, path: &str) -> Option<String> {
        self.backend_url.as_ref().map(|base| format!("{}{}", base.trim_end_matches('/'), path))
    }

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

    pub async fn extract_current_kv_entries(&self) -> anyhow::Result<Vec<KVEntry>> {
        let slots = self.fetch_slots().await;
        let slot = match slots.into_iter().find(|s| s.id == 0) {
            Some(s) if s.n_past > 0 => s,
            _ => return Ok(Vec::new()),
        };

        let scorer = CacheEntryScorer::new(CacheScoringConfig::default());
        let mut entries = Vec::new();

        let bucket_size = 64usize;
        let n_buckets = (slot.n_past + bucket_size - 1) / bucket_size;

        for bucket_idx in 0..n_buckets {
            let token_start = bucket_idx * bucket_size;
            let token_end = (token_start + bucket_size).min(slot.n_past);

            let layer_index = (bucket_idx % 32) as i32;
            let head_index = (bucket_idx % 8) as i32;

            let position_fraction = token_start as f32 / slot.n_past as f32;
            
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

    pub async fn inject_kv_entries(&self, entries: &[KVEntry]) -> anyhow::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

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
                
                warn!("KV restore returned {}: continuing without restored cache", resp.status());
            }
            Err(e) => {
                warn!("KV restore request failed: {} — continuing without restored cache", e);
            }
        }

        Ok(())
    }

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

    pub async fn get_cache_memory_usage(&self) -> anyhow::Result<usize> {
        let slots = self.fetch_slots().await;
        let n_past = slots.iter().find(|s| s.id == 0).map(|s| s.n_past).unwrap_or(0);
        let bytes_per_token = 32 * 128 * 2 * 4; 
        Ok(n_past * bytes_per_token)
    }

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
