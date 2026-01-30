use crate::memory::Message;
use crate::memory_db::{MemoryDatabase, StoredMessage, Summary as DbSummary, SessionMetadata};
use moka::sync::Cache;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};
/
#[derive(Debug, Clone)]
pub struct TierManagerConfig {
    pub tier1_max_messages: usize,
    pub tier2_max_summaries: usize,
    pub tier2_cache_ttl_seconds: u64,
    pub enable_tier3_persistence: bool,
}
impl Default for TierManagerConfig {
    fn default() -> Self {
        Self {
            tier1_max_messages: 50,
            tier2_max_summaries: 20,
            tier2_cache_ttl_seconds: 3600,
            enable_tier3_persistence: true,
        }
    }
}
/
#[derive(Debug, Clone, Default)]
pub struct TierStats {
    pub tier1_count: usize,
    pub tier2_count: usize,
    pub tier3_count: usize,
}
pub struct TierManager {
    database: Arc<MemoryDatabase>,
    tier1_cache: Cache<String, (Vec<Message>, Instant)>,
    tier2_cache: Cache<String, (Vec<DbSummary>, Instant)>,
    pub config: TierManagerConfig,
}
impl TierManager {
    pub fn new(
        database: Arc<MemoryDatabase>,
        config: TierManagerConfig
    ) -> Self {
        Self {
            database,
            tier1_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_idle(Duration::from_secs(3600))
                .build(),
            tier2_cache: Cache::builder()
                .max_capacity(500)
                .time_to_idle(Duration::from_secs(config.tier2_cache_ttl_seconds))
                .build(),
            config,
        }
    }

    pub async fn store_tier1_content(&self, session_id: &str, messages: &[Message]) {

        let messages_to_store = if messages.len() > self.config.tier1_max_messages {
            &messages[messages.len() - self.config.tier1_max_messages..]
        } else {
            messages
        };

        self.tier1_cache.insert(session_id.to_string(), (messages_to_store.to_vec(), Instant::now()));
    }
    pub async fn get_tier1_content(&self, session_id: &str) -> Option<Vec<Message>> {
        self.tier1_cache.get(session_id).map(|(m, _)| m)
    }

    pub async fn get_tier2_content(&self, session_id: &str) -> Option<Vec<DbSummary>> {

        if let Some((summaries, _)) = self.tier2_cache.get(session_id) {
            return Some(summaries);
        }


        match self.database.summaries.get_session_summaries(session_id) {
            Ok(summaries) => {

                if !summaries.is_empty() {
                    self.tier2_cache.insert(session_id.to_string(), (summaries.clone(), Instant::now()));
                }
                Some(summaries)
            }
            Err(e) => {
                debug!("Error getting summaries from database: {}", e);
                None
            }
        }
    }

    pub async fn get_tier3_content(
        &self,
        session_id: &str,
        limit: Option<i32>,
        offset: Option<i32>
    ) -> anyhow::Result<Vec<StoredMessage>> {
        self.database.conversations.get_session_messages(session_id, limit, offset)
    }
    pub async fn search_tier3_content(
        &self,
        session_id: &str,
        query: &str,
        limit: usize
    ) -> anyhow::Result<Vec<StoredMessage>> {
        let messages = self.database.conversations.get_session_messages(session_id, Some(1000), None)?;
        let query_lower = query.to_lowercase();

        let filtered = messages.into_iter()
            .filter(|m| m.content.to_lowercase().contains(&query_lower))
            .take(limit)
            .collect();

        Ok(filtered)
    }
    pub async fn store_tier3_content(&self, session_id: &str, messages: &[Message]) -> anyhow::Result<()> {
        if !self.config.enable_tier3_persistence || messages.is_empty() {
            return Ok(());
        }


        self.ensure_session_exists(session_id, None).await?;


        let existing_messages = self.database.conversations.get_session_messages(
            session_id, Some(10000), Some(0)
        ).unwrap_or_else(|_| vec![]);


        let new_messages: Vec<&Message> = messages.iter()
            .filter(|new_msg| {
                !existing_messages.iter().any(|existing| {
                    existing.content == new_msg.content &&
                    existing.role == new_msg.role
                })
            })
            .collect();

        if new_messages.is_empty() {
            debug!("No new messages to save, all already exist in database");
            return Ok(());
        }

        let start_index = existing_messages.len() as i32;


        let batch_data: Vec<(String, String, i32, i32, f32)> = new_messages
            .iter()
            .enumerate()
            .map(|(offset, m)| (
                m.role.clone(),
                m.content.clone(),
                start_index + offset as i32,
                (m.content.len() / 4) as i32,
                0.5
            ))
            .collect();

        if !batch_data.is_empty() {
            self.database.conversations.store_messages_batch(session_id, &batch_data)?;
            info!("ðŸ“ Stored {} new messages to database for session {}", batch_data.len(), session_id);
        }

        Ok(())
    }

    /
    pub async fn search_cross_session_content(
        &self,
        current_session_id: &str,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<StoredMessage>> {

        let keywords = self.extract_keywords(query);

        if keywords.is_empty() {
            return Ok(vec![]);
        }

        self.database.conversations.search_messages_by_topic_across_sessions(
            &keywords,
            limit,
            Some(current_session_id),
        ).await
    }
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        words.iter()
            .filter(|w| w.len() > 3)
            .map(|w| w.to_lowercase())
            .filter(|w| !self.is_stop_word(w))
            .collect()
    }
    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "is", "am", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will", "would",
            "shall", "should", "may", "might", "must", "can", "could",
        ];
        stop_words.contains(&word)
    }

    pub async fn get_tier_stats(&self, session_id: &str) -> TierStats {
        let tier1_count = self.get_tier1_content(session_id).await
            .map(|m| m.len())
            .unwrap_or(0);

        let tier2_count = self.get_tier2_content(session_id).await
            .map(|s| s.len())
            .unwrap_or(0);

        let tier3_count = match self.database.conversations.get_session_messages(session_id, Some(10000), None) {
            Ok(messages) => messages.len(),
            Err(_) => 0,
        };
        TierStats {
            tier1_count,
            tier2_count,
            tier3_count
        }
    }
    pub async fn cleanup_cache(&self, _older_than_seconds: u64) -> usize {
        let count = self.tier1_cache.entry_count() + self.tier2_cache.entry_count();



        self.tier1_cache.invalidate_all();
        self.tier2_cache.invalidate_all();

        count as usize
    }
    /
    pub async fn ensure_session_exists(
        &self,
        session_id: &str,
        title: Option<String>
    ) -> anyhow::Result<()> {
        let exists = self.database.conversations.get_session(session_id)?;
        if exists.is_none() {

            let metadata = SessionMetadata {
                title,
                ..Default::default()
            };
            self.database.conversations.create_session_with_id(session_id, Some(metadata))?;
        }
        Ok(())
    }
}
impl Clone for TierManager {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            tier1_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_idle(Duration::from_secs(3600))
                .build(),
            tier2_cache: Cache::builder()
                .max_capacity(500)
                .time_to_idle(Duration::from_secs(self.config.tier2_cache_ttl_seconds))
                .build(),
            config: self.config.clone(),
        }
    }
}

