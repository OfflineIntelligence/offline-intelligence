use serde::{Deserialize, Serialize};
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVCacheConfig {
    /
    pub enabled: bool,

    /
    pub retrieval_enabled: bool,

    /
    pub clear_after_conversations: usize,

    /
    pub memory_threshold_percent: f32,

    /
    pub bridge_enabled: bool,

    /
    pub max_cache_entries: usize,

    /
    pub min_importance_to_preserve: f32,

    /
    pub generate_cache_embeddings: bool,

    /
    pub retrieval_strategy: RetrievalStrategy,

    /
    pub preserve_system_prompts: bool,

    /
    pub preserve_code_entries: bool,

    /
    pub snapshot_strategy: SnapshotStrategy,
}
impl Default for KVCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retrieval_enabled: true,
            clear_after_conversations: 16,
            memory_threshold_percent: 0.6,
            bridge_enabled: true,
            max_cache_entries: 1000,
            min_importance_to_preserve: 0.7,
            generate_cache_embeddings: true,
            retrieval_strategy: RetrievalStrategy::KeywordThenSemantic,
            preserve_system_prompts: true,
            preserve_code_entries: true,
            snapshot_strategy: SnapshotStrategy::Incremental {
                interval_conversations: 4,
                max_snapshots: 4,
            },
        }
    }
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetrievalStrategy {
    /
    KeywordOnly,
    /
    SemanticOnly,
    /
    KeywordThenSemantic,
    /
    SemanticThenKeyword,
    /
    Hybrid {
        keyword_weight: f32,
        semantic_weight: f32,
    },
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotStrategy {
    /
    None,
    /
    Full {
        interval_conversations: usize,
    },
    /
    Incremental {
        interval_conversations: usize,
        max_snapshots: usize,
    },
    /
    Adaptive {
        min_importance_threshold: f32,
        max_snapshots: usize,
    },
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePreservationConfig {
    /
    pub preserve_attention_keys: bool,

    /
    pub preserve_attention_values: bool,

    /
    pub preserve_ffn_keys: bool,

    /
    pub preserve_ffn_values: bool,

    /
    pub preserve_early_layers: bool,

    /
    pub preserve_late_layers: bool,

    /
    pub custom_patterns: Vec<String>,
}
impl Default for CachePreservationConfig {
    fn default() -> Self {
        Self {
            preserve_attention_keys: true,
            preserve_attention_values: true,
            preserve_ffn_keys: false,
            preserve_ffn_values: false,
            preserve_early_layers: true,
            preserve_late_layers: false,
            custom_patterns: Vec::new(),
        }
    }
}

