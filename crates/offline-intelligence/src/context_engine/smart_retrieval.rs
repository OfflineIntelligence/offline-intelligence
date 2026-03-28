//! Smart retrieval with two-tier context optimization
//!
//! This module implements intelligent retrieval that minimizes recomputation cost
//! by enforcing strict token budgets and importance filtering.
//!
//! Key optimizations:
//! - Tier 1 (hot cache) → O(1) return, 100% compute savings
//! - Tier 3 (cold storage) → Importance-filtered, token-budgeted SQLite retrieval

use crate::memory::Message;
use crate::memory_db::StoredMessage;
use crate::context_engine::tier_manager::TierManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Configuration for smart retrieval
#[derive(Debug, Clone)]
pub struct SmartRetrievalConfig {
    /// Maximum tokens for retrieved historical context (excludes current messages)
    pub max_retrieved_tokens: usize,

    /// Minimum importance score to include a message (0.0-1.0)
    pub importance_threshold: f32,

    /// Group contiguous messages into chunks for better llama.cpp caching
    pub chunk_contiguous_messages: bool,

    /// Enable smart retrieval (can be disabled to fall back to original behavior)
    pub enabled: bool,
}

impl Default for SmartRetrievalConfig {
    fn default() -> Self {
        Self {
            max_retrieved_tokens: 1000,
            importance_threshold: 0.5,
            chunk_contiguous_messages: true,
            enabled: true,
        }
    }
}

impl SmartRetrievalConfig {
    /// Derive the historical retrieval budget from the model's context window.
    /// 25% of CTX_SIZE is allocated to retrieved history (summaries + cold SQLite),
    /// ensuring the current conversation always gets the lion's share.
    pub fn from_ctx_size(ctx_size: u32) -> Self {
        Self {
            max_retrieved_tokens: (ctx_size as f32 * 0.25) as usize,
            ..Self::default()
        }
    }
}

/// Result of smart retrieval operation
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// Strategy used for retrieval
    pub strategy: RetrievalStrategy,

    /// Optimized messages to send to LLM
    pub messages: Vec<Message>,

    /// Estimated computation cost saved (0.0-1.0)
    pub compute_savings: f32,

    /// Token count of retrieved context
    pub retrieved_tokens: usize,

    /// Sessions referenced in the retrieval
    pub sessions_referenced: Vec<String>,
}

/// Strategy used for retrieval
#[derive(Debug, Clone, PartialEq)]
pub enum RetrievalStrategy {
    /// Current session already in Tier 1 hot cache
    HotCacheHit,

    /// Retrieved chunks with importance filtering
    ImportanceFiltered,

    /// Full retrieval (fallback, no optimization)
    FullRetrieval,

    /// No retrieval needed (fresh context)
    NoRetrieval,
}

/// Smart retrieval orchestrator
pub struct SmartRetrieval {
    tier_manager: Arc<RwLock<TierManager>>,
    config: SmartRetrievalConfig,
}

impl SmartRetrieval {
    /// Create a new smart retrieval instance
    pub fn new(tier_manager: Arc<RwLock<TierManager>>, config: SmartRetrievalConfig) -> Self {
        Self {
            tier_manager,
            config,
        }
    }

    /// Main retrieval function with smart optimization
    pub async fn retrieve(
        &self,
        session_id: &str,
        current_messages: &[Message],
        tier3_messages: Option<Vec<StoredMessage>>,
        cross_session_messages: Option<Vec<StoredMessage>>,
    ) -> anyhow::Result<RetrievalResult> {
        if !self.config.enabled {
            debug!("Smart retrieval disabled, using fallback");
            return self.fallback_retrieval(current_messages);
        }

        // Step 1: Check Tier 1 hot cache
        let tier_manager = self.tier_manager.read().await;
        if let Some(hot_messages) = tier_manager.get_tier1_content(session_id).await {
            let retrieved_tokens = self.count_tokens(&hot_messages);
            info!("🚀 Smart retrieval: Tier 1 hot cache hit for session {}", session_id);
            return Ok(RetrievalResult {
                strategy: RetrievalStrategy::HotCacheHit,
                messages: hot_messages,
                compute_savings: 1.0,
                retrieved_tokens,
                sessions_referenced: vec![session_id.to_string()],
            });
        }
        drop(tier_manager);

        // Step 2: Check if we have any historical content
        let has_tier3 = tier3_messages.as_ref().map(|m| !m.is_empty()).unwrap_or(false);
        let has_cross_session = cross_session_messages.as_ref().map(|m| !m.is_empty()).unwrap_or(false);

        if !has_tier3 && !has_cross_session {
            debug!("No historical content available, returning current messages");
            return Ok(RetrievalResult {
                strategy: RetrievalStrategy::NoRetrieval,
                messages: current_messages.to_vec(),
                compute_savings: 0.0,
                retrieved_tokens: 0,
                sessions_referenced: vec![],
            });
        }

        // Step 3: Build optimized context from Tier 1 (hot) and Tier 3 (cold) only
        let optimized_context = self.build_context_from_tiers(
            current_messages,
            tier3_messages.as_ref(),
            cross_session_messages.as_ref(),
        ).await?;

        let strategy = if self.config.importance_threshold > 0.0 {
            RetrievalStrategy::ImportanceFiltered
        } else {
            RetrievalStrategy::FullRetrieval
        };

        let compute_savings = self.estimate_compute_savings(&strategy, &optimized_context.messages);

        info!(
            "Smart retrieval complete: Strategy={:?}, Tokens={}, Savings={:.1}%",
            strategy,
            optimized_context.retrieved_tokens,
            compute_savings * 100.0
        );

        Ok(optimized_context)
    }

    /// Build context from Tier 1 (hot) and Tier 3 (cold storage) with importance filtering
    async fn build_context_from_tiers(
        &self,
        current_messages: &[Message],
        tier3_messages: Option<&Vec<StoredMessage>>,
        cross_session_messages: Option<&Vec<StoredMessage>>,
    ) -> anyhow::Result<RetrievalResult> {
        let mut context = Vec::new();
        let mut retrieved_tokens = 0;
        let mut sessions_referenced = Vec::new();

        let current_tokens: usize = current_messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum();

        let budget_for_history = self.config.max_retrieved_tokens.saturating_sub(current_tokens);

        // Add cross-session context (highest priority, 1/3 of budget)
        if let Some(cross_msgs) = cross_session_messages {
            if !cross_msgs.is_empty() {
                let cross_context = self.add_cross_session_context(cross_msgs, budget_for_history / 3);
                retrieved_tokens += self.count_tokens(&cross_context);

                for msg in cross_msgs.iter().take(3) {
                    if !sessions_referenced.contains(&msg.session_id) {
                        sessions_referenced.push(msg.session_id.clone());
                    }
                }

                context.extend(cross_context);
            }
        }

        // Add importance-filtered messages from Tier 3 (cold storage)
        if let Some(tier3_msgs) = tier3_messages {
            let remaining_budget = budget_for_history.saturating_sub(retrieved_tokens);
            let detail_context = self.add_important_details(tier3_msgs, remaining_budget);
            retrieved_tokens += self.count_tokens(&detail_context);
            context.extend(detail_context);
        }

        // Always append current messages last
        context.extend_from_slice(current_messages);

        Ok(RetrievalResult {
            strategy: RetrievalStrategy::ImportanceFiltered,
            messages: context,
            compute_savings: 0.0,
            retrieved_tokens,
            sessions_referenced,
        })
    }

    /// Add cross-session context with budget enforcement
    fn add_cross_session_context(
        &self,
        cross_messages: &[StoredMessage],
        token_budget: usize,
    ) -> Vec<Message> {
        let mut context = Vec::new();
        let mut used_tokens = 0;

        // Add bridge message
        context.push(Message {
            role: "system".to_string(),
            content: "[Context from previous conversations]".to_string(),
        });
        used_tokens += 8;

        // Add top 3 most important cross-session messages
        let mut scored: Vec<_> = cross_messages.iter()
            .map(|m| (m, m.importance_score))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (msg, _score) in scored.iter().take(3) {
            let msg_tokens = msg.tokens as usize;
            if used_tokens + msg_tokens > token_budget {
                break;
            }

            context.push(Message {
                role: msg.role.clone(),
                content: format!("[From earlier: {}]", msg.content),
            });
            used_tokens += msg_tokens;
        }

        debug!("Added {} cross-session messages ({} tokens)", context.len() - 1, used_tokens);
        context
    }

    /// Add important details with importance filtering and budget
    fn add_important_details(
        &self,
        messages: &[StoredMessage],
        token_budget: usize,
    ) -> Vec<Message> {
        let mut context = Vec::new();
        let mut used_tokens = 0;

        // Filter by importance threshold
        let important: Vec<_> = messages.iter()
            .filter(|m| m.importance_score >= self.config.importance_threshold)
            .collect();

        if important.is_empty() {
            debug!("No messages meet importance threshold {}", self.config.importance_threshold);
            return context;
        }

        // Sort by importance score descending
        let mut scored = important.clone();
        scored.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap_or(std::cmp::Ordering::Equal));

        // Add messages until budget exhausted
        for msg in scored {
            let msg_tokens = msg.tokens as usize;
            if used_tokens + msg_tokens > token_budget {
                break;
            }

            context.push(Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
            used_tokens += msg_tokens;
        }

        info!("Added {} important messages ({} tokens, threshold={:.2})",
              context.len(),
              used_tokens,
              self.config.importance_threshold
        );

        context
    }

    /// Estimate compute savings based on strategy
    fn estimate_compute_savings(&self, strategy: &RetrievalStrategy, _messages: &[Message]) -> f32 {
        match strategy {
            RetrievalStrategy::HotCacheHit => 1.0,
            RetrievalStrategy::ImportanceFiltered => 0.6,
            RetrievalStrategy::FullRetrieval => 0.0,
            RetrievalStrategy::NoRetrieval => 0.0,
        }
    }

    /// Count total tokens in messages
    fn count_tokens(&self, messages: &[Message]) -> usize {
        messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum()
    }

    /// Estimate tokens for a message (rough approximation: 4 chars per token)
    fn estimate_message_tokens(&self, message: &Message) -> usize {
        message.content.len() / 4
    }

    /// Fallback retrieval (disabled smart retrieval)
    fn fallback_retrieval(&self, current_messages: &[Message]) -> anyhow::Result<RetrievalResult> {
        Ok(RetrievalResult {
            strategy: RetrievalStrategy::FullRetrieval,
            messages: current_messages.to_vec(),
            compute_savings: 0.0,
            retrieved_tokens: 0,
            sessions_referenced: vec![],
        })
    }
}

impl Clone for SmartRetrieval {
    fn clone(&self) -> Self {
        Self {
            tier_manager: Arc::clone(&self.tier_manager),
            config: self.config.clone(),
        }
    }
}
