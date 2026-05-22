// src/cache_management/mod.rs

//! KV Cache management system for efficient context preservation and retrieval

pub mod cache_bridge;
pub mod cache_config;
pub mod cache_extractor;
pub mod cache_manager;
pub mod cache_scorer;
pub mod llama_cache_interface;

// Re-exports
pub use cache_bridge::{CacheContextBridge, CacheBridgeStats, CacheTransition, TransitionType};
pub use cache_config::{KVCacheConfig, RetrievalStrategy, SnapshotStrategy, CachePreservationConfig};
pub use cache_extractor::{CacheExtractor, CacheExtractorConfig, ExtractedCacheEntry, CacheEntryType, KVEntry};
pub use cache_manager::{
    KVCacheManager, SessionCacheState, CacheStatistics, CacheOperation, CacheOperationType,
    ClearReason, CacheClearResult, RetrievalResult, RetrievedEntry, CacheProcessingResult,
    CacheStatisticsExport, MaintenanceResult
};
pub use cache_scorer::{CacheEntryScorer, CacheScoringConfig};
pub use llama_cache_interface::{LlamaKVCacheInterface, LlamaKVCacheState};

/// Create a default KV cache manager.
/// Pass `llm_worker` to enable pre-clear summarization.
pub fn create_default_cache_manager(
    config: KVCacheConfig,
    database: std::sync::Arc<crate::memory_db::MemoryDatabase>,
    llm_worker: Option<std::sync::Arc<crate::worker_threads::LLMWorker>>,
) -> anyhow::Result<KVCacheManager> {
    KVCacheManager::new(config, database, llm_worker)
}