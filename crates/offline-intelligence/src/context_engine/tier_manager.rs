//! Manages the three-tier memory system with robust persistence and indexing

use crate::memory::Message;
use crate::memory_db::{MemoryDatabase, StoredMessage, SessionMetadata};
use crate::cache_management::cache_scorer::score_message_importance;
use moka::sync::Cache;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Configuration for tier management
#[derive(Debug, Clone)]
pub struct TierManagerConfig {
    /// Maximum number of messages stored in Tier 1 hot cache per session.
    /// Derived from CTX_SIZE: 40% of context window / avg 20 tokens per message.
    pub tier1_max_messages: usize,
    pub enable_tier3_persistence: bool,
}

impl Default for TierManagerConfig {
    fn default() -> Self {
        Self {
            tier1_max_messages: 50,
            enable_tier3_persistence: true,
        }
    }
}

impl TierManagerConfig {
    /// Derive Tier 1 limits from the model's context window.
    /// 40% of CTX_SIZE is reserved for current-session hot messages.
    /// Divided by an average of 20 tokens per message to get the message count cap.
    pub fn from_ctx_size(ctx_size: u32) -> Self {
        let tier1_token_budget = (ctx_size as f32 * 0.40) as usize;
        let avg_tokens_per_message = 20;
        Self {
            tier1_max_messages: (tier1_token_budget / avg_tokens_per_message).max(10),
            enable_tier3_persistence: true,
        }
    }
}

/// Statistics about tier usage
#[derive(Debug, Clone, Default)]
pub struct TierStats {
    pub tier1_count: usize,
    pub tier3_count: usize,
}

pub struct TierManager {
    database: Arc<MemoryDatabase>,
    tier1_cache: Cache<String, (Vec<Message>, Instant)>,
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
            config,
        }
    }

    // --- Tier 1 (Cache) Methods ---

    pub async fn store_tier1_content(&self, session_id: &str, messages: &[Message]) {
        // Apply tier1 max messages limit
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

    // --- Tier 3 (Database) Methods ---

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
        
        // Ensure session exists in database
        self.ensure_session_exists(session_id, None).await?;
        
        // Get existing messages to find the next index AND check for duplicates
        let existing_messages = self.database.conversations.get_session_messages(
            session_id, Some(10000), Some(0)
        ).unwrap_or_else(|_| vec![]);
        
        // Filter out messages that already exist (simple content-based deduplication)
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
            return Ok(()); // Nothing new to save
        }
        
        let start_index = existing_messages.len() as i32;
        
        // Create batch data for ONLY new messages
        let batch_data: Vec<(String, String, i32, i32, f32)> = new_messages
            .iter()
            .enumerate()
            .map(|(offset, m)| (
                m.role.clone(),
                m.content.clone(),
                start_index + offset as i32, // Ensure unique index
                (m.content.len() / 4) as i32,
                score_message_importance(&m.role, &m.content)
            ))
            .collect();
        
        if !batch_data.is_empty() {
            self.database.conversations.store_messages_batch(session_id, &batch_data)?;
            info!("📝 Stored {} new messages to database for session {}", batch_data.len(), session_id);
        }
        
        Ok(())
    }

    // --- Cross-Session Content Methods ---

    /// Searches across all sessions except the current one based on keyword extraction
    pub async fn search_cross_session_content(
        &self,
        current_session_id: &str,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<StoredMessage>> {
        // Extract keywords from query
        let keywords = self.extract_keywords(query);
        
        if keywords.is_empty() {
            return Ok(vec![]);
        }

        // Search across ALL sessions except current one
        self.database.conversations.search_messages_by_topic_across_sessions(
            &keywords,
            limit,
            Some(current_session_id), // Exclude current session
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

    // --- Maintenance & Stats ---

    pub async fn get_tier_stats(&self, session_id: &str) -> TierStats {
        let tier1_count = self.get_tier1_content(session_id).await
            .map(|m| m.len())
            .unwrap_or(0);

        let tier3_count = match self.database.conversations.get_session_messages(session_id, Some(10000), None) {
            Ok(messages) => messages.len(),
            Err(_) => 0,
        };

        TierStats { tier1_count, tier3_count }
    }

    pub async fn cleanup_cache(&self, _older_than_seconds: u64) -> usize {
        let count = self.tier1_cache.entry_count();
        self.tier1_cache.invalidate_all();
        count as usize
    }

    /// Chat persistence: Ensure session exists in database with provided ID (no auto-generated placeholders)
    pub async fn ensure_session_exists(
        &self, 
        session_id: &str, 
        title: Option<String>
    ) -> anyhow::Result<()> {
        let exists = self.database.conversations.get_session(session_id)?;
        if exists.is_none() {
            // Create session with null title initially - title set via API after generation
            let metadata = SessionMetadata {
                title, // None initially; title updated later via update_conversation_title API
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
            config: self.config.clone(),
        }
    }
}