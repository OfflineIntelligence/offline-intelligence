//! Cache worker thread implementation
//!
//! Handles KV cache operations in a dedicated thread.

use std::sync::Arc;
use tracing::{info, debug};

use crate::{
    shared_state::SharedState,
    cache_management::cache_extractor::KVEntry,
};

pub struct CacheWorker {
    shared_state: Arc<SharedState>,
}

impl CacheWorker {
    pub fn new(shared_state: Arc<SharedState>) -> Self {
        Self { shared_state }
    }
    
    /// Persist cache entries for a session by creating a KV snapshot.
    pub async fn update_cache(
        &self,
        session_id: String,
        entries: Vec<KVEntry>,
    ) -> anyhow::Result<()> {
        debug!("Cache worker updating cache for session: {}", session_id);

        let cache_guard = self.shared_state.cache_manager.read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire cache manager read lock"))?;
        if let Some(cache_manager) = &*cache_guard {
            let snapshot_id = cache_manager.flush_to_database(&session_id, &entries).await?;
            info!("Persisted cache for session {} with {} entries (snapshot: {:?})",
                  session_id, entries.len(), snapshot_id);
            self.shared_state.counters.inc_cache_hit();
        } else {
            debug!("Cache manager not available");
            self.shared_state.counters.inc_cache_miss();
        }

        Ok(())
    }

    /// Retrieve the most-recently persisted KV cache entries for a session.
    pub async fn get_cache_entries(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<KVEntry>> {
        debug!("Cache worker retrieving entries for session: {}", session_id);

        // Look up the latest KV snapshot persisted for this session
        let snapshots = self.shared_state.database_pool
            .get_recent_kv_snapshots(session_id, 1)
            .await?;

        if let Some(snapshot) = snapshots.first() {
            let entries = self.shared_state.database_pool
                .get_kv_snapshot_entries(snapshot.id)
                .await?;
            info!("Retrieved {} cache entries for session {}", entries.len(), session_id);
            Ok(entries)
        } else {
            debug!("No KV snapshots found for session {}", session_id);
            Ok(Vec::new())
        }
    }
    
    /// Create KV snapshot
    pub async fn create_snapshot(
        &self,
        session_id: &str,
        entries: &[KVEntry],
    ) -> anyhow::Result<i64> {
        debug!("Creating KV snapshot for session: {}", session_id);
        
        // Use database pool from shared state
        let snapshot_id = self.shared_state.database_pool
            .create_kv_snapshot(session_id, entries)
            .await?;
            
        info!("Created KV snapshot {} for session {}", snapshot_id, session_id);
        Ok(snapshot_id)
    }
}