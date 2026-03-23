//! Smart retrieval with hierarchical context optimization
//!
//! This module implements intelligent retrieval that minimizes recomputation cost
//! by preferring summaries over raw messages and enforcing strict token budgets.
//!
//! Key optimizations:
//! - Tier 1 (hot cache) → O(1) return
//! - Tier 2 (summaries) → Compressed context (50 tokens vs 2000)
//! - Tier 3 (database) → Importance-filtered, token-budgeted
//! - Hierarchical assembly → Summary first, details on-demand

use crate::memory::Message;
use crate::memory_db::{StoredMessage, Summary as DbSummary};
use crate::context_engine::tier_manager::TierManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Configuration for smart retrieval
#[derive(Debug, Clone)]
pub struct SmartRetrievalConfig {
    /// Maximum tokens for retrieved historical context (excludes current messages)
    pub max_retrieved_tokens: usize,

    /// Prefer summaries over raw messages when available
    pub prefer_summaries: bool,

    /// Minimum importance score to include a message (0.0-1.0)
    pub importance_threshold: f32,

    /// Group contiguous messages into chunks for better llama.cpp caching
    pub chunk_contiguous_messages: bool,

    /// Use hierarchical context (summary first, then details)
    pub use_hierarchical_context: bool,

    /// Enable smart retrieval (can be disabled to fall back to original behavior)
    pub enabled: bool,
}

impl Default for SmartRetrievalConfig {
    fn default() -> Self {
        Self {
            max_retrieved_tokens: 1000,  // Budget for old context
            prefer_summaries: true,       // Always prefer compressed context
            importance_threshold: 0.5,    // Only retrieve important messages
            chunk_contiguous_messages: true,
            use_hierarchical_context: true,
            enabled: true,
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

    /// Used summaries to compress old context
    SummaryCompression,

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
        tier2_summaries: Option<Vec<DbSummary>>,
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
                compute_savings: 1.0,  // 100% savings - no recomputation
                retrieved_tokens,
                sessions_referenced: vec![session_id.to_string()],
            });
        }
        drop(tier_manager);

        // Step 2: Check if we have any historical content
        let has_summaries = tier2_summaries.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let has_tier3 = tier3_messages.as_ref().map(|m| !m.is_empty()).unwrap_or(false);
        let has_cross_session = cross_session_messages.as_ref().map(|m| !m.is_empty()).unwrap_or(false);

        if !has_summaries && !has_tier3 && !has_cross_session {
            debug!("No historical content available, returning current messages");
            return Ok(RetrievalResult {
                strategy: RetrievalStrategy::NoRetrieval,
                messages: current_messages.to_vec(),
                compute_savings: 0.0,
                retrieved_tokens: 0,
                sessions_referenced: vec![],
            });
        }

        // Step 3: Build optimized context based on available tiers
        let optimized_context = if self.config.use_hierarchical_context {
            self.build_hierarchical_context(
                current_messages,
                tier2_summaries.as_ref(),
                tier3_messages.as_ref(),
                cross_session_messages.as_ref(),
            ).await?
        } else {
            self.build_standard_context(
                current_messages,
                tier3_messages.as_ref(),
                cross_session_messages.as_ref(),
            ).await?
        };

        // Determine strategy based on what was used
        let strategy = if has_summaries && self.config.prefer_summaries {
            RetrievalStrategy::SummaryCompression
        } else if self.config.importance_threshold > 0.0 {
            RetrievalStrategy::ImportanceFiltered
        } else {
            RetrievalStrategy::FullRetrieval
        };

        // Calculate compute savings
        let compute_savings = self.estimate_compute_savings(&strategy, &optimized_context.messages);

        info!(
            "Smart retrieval complete: Strategy={:?}, Tokens={}, Savings={:.1}%",
            strategy,
            optimized_context.retrieved_tokens,
            compute_savings * 100.0
        );

        Ok(optimized_context)
    }

    /// Build hierarchical context (summary first, then details)
    async fn build_hierarchical_context(
        &self,
        current_messages: &[Message],
        tier2_summaries: Option<&Vec<DbSummary>>,
        tier3_messages: Option<&Vec<StoredMessage>>,
        cross_session_messages: Option<&Vec<StoredMessage>>,
    ) -> anyhow::Result<RetrievalResult> {
        let mut context = Vec::new();
        let mut retrieved_tokens = 0;
        let mut sessions_referenced = Vec::new();

        // Reserve budget for current messages
        let current_tokens: usize = current_messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum();

        let budget_for_history = self.config.max_retrieved_tokens.saturating_sub(current_tokens);

        // Step 1: Add cross-session context if available (highest priority)
        if let Some(cross_msgs) = cross_session_messages {
            if !cross_msgs.is_empty() {
                let cross_context = self.add_cross_session_context(
                    cross_msgs,
                    budget_for_history / 3,  // Allocate 1/3 budget for cross-session
                );
                retrieved_tokens += self.count_tokens(&cross_context);

                // Track unique sessions
                for msg in cross_msgs.iter().take(3) {
                    if !sessions_referenced.contains(&msg.session_id) {
                        sessions_referenced.push(msg.session_id.clone());
                    }
                }

                context.extend(cross_context);
            }
        }

        // Step 2: Add summaries if available and preferred
        if self.config.prefer_summaries {
            if let Some(summaries) = tier2_summaries {
                if !summaries.is_empty() {
                    let summary_context = self.add_summary_context(
                        summaries,
                        budget_for_history.saturating_sub(retrieved_tokens),
                    );
                    retrieved_tokens += self.count_tokens(&summary_context);
                    context.extend(summary_context);

                    info!("📋 Used {} summaries (compressed context)", summaries.len());
                }
            }
        }

        // Step 3: Add important details from Tier 3 if budget allows
        if retrieved_tokens < budget_for_history {
            if let Some(tier3_msgs) = tier3_messages {
                let remaining_budget = budget_for_history.saturating_sub(retrieved_tokens);
                let detail_context = self.add_important_details(
                    tier3_msgs,
                    remaining_budget,
                );
                retrieved_tokens += self.count_tokens(&detail_context);
                context.extend(detail_context);
            }
        }

        // Step 4: Add current messages (always included, full detail)
        context.extend_from_slice(current_messages);

        Ok(RetrievalResult {
            strategy: RetrievalStrategy::SummaryCompression,
            messages: context,
            compute_savings: 0.0,  // Will be calculated by caller
            retrieved_tokens,
            sessions_referenced,
        })
    }

    /// Build standard context (importance-filtered only)
    async fn build_standard_context(
        &self,
        current_messages: &[Message],
        tier3_messages: Option<&Vec<StoredMessage>>,
        cross_session_messages: Option<&Vec<StoredMessage>>,
    ) -> anyhow::Result<RetrievalResult> {
        let mut context = Vec::new();
        let mut retrieved_tokens = 0;
        let mut sessions_referenced = Vec::new();

        // Calculate budget
        let current_tokens: usize = current_messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum();

        let budget_for_history = self.config.max_retrieved_tokens.saturating_sub(current_tokens);

        // Add cross-session messages
        if let Some(cross_msgs) = cross_session_messages {
            if !cross_msgs.is_empty() {
                let cross_context = self.add_cross_session_context(cross_msgs, budget_for_history / 2);
                retrieved_tokens += self.count_tokens(&cross_context);

                for msg in cross_msgs.iter().take(3) {
                    if !sessions_referenced.contains(&msg.session_id) {
                        sessions_referenced.push(msg.session_id.clone());
                    }
                }

                context.extend(cross_context);
            }
        }

        // Add filtered tier3 messages
        if let Some(tier3_msgs) = tier3_messages {
            let remaining_budget = budget_for_history.saturating_sub(retrieved_tokens);
            let detail_context = self.add_important_details(tier3_msgs, remaining_budget);
            retrieved_tokens += self.count_tokens(&detail_context);
            context.extend(detail_context);
        }

        // Add current messages
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

    /// Add summary context with compression
    fn add_summary_context(
        &self,
        summaries: &[DbSummary],
        token_budget: usize,
    ) -> Vec<Message> {
        let mut context = Vec::new();
        let mut used_tokens = 0;

        // Sort summaries by relevance (compression ratio as proxy for importance)
        let mut scored: Vec<_> = summaries.iter()
            .map(|s| (s, s.compression_ratio))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Track best compression for logging
        let best_compression = scored.first().map(|(s, _)| s.compression_ratio).unwrap_or(1.0);

        for (summary, _score) in scored.iter() {
            let summary_tokens = summary.summary_text.len() / 4;
            if used_tokens + summary_tokens > token_budget {
                break;
            }

            context.push(Message {
                role: "system".to_string(),
                content: format!("[Summary: {}]", summary.summary_text),
            });
            used_tokens += summary_tokens;
        }

        info!("Added {} summaries ({} tokens, compression saved {}%)",
              context.len(),
              used_tokens,
              (1.0 - best_compression) * 100.0
        );

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
    fn estimate_compute_savings(&self, strategy: &RetrievalStrategy, messages: &[Message]) -> f32 {
        match strategy {
            RetrievalStrategy::HotCacheHit => 1.0,  // 100% savings (cached in RAM)
            RetrievalStrategy::SummaryCompression => {
                // Estimate based on token reduction
                // If we compressed 2000 tokens to 50, that's 97.5% savings
                let total_tokens = self.count_tokens(messages);
                if total_tokens < 100 {
                    0.95  // High compression
                } else if total_tokens < 500 {
                    0.75  // Medium compression
                } else {
                    0.5   // Low compression
                }
            }
            RetrievalStrategy::ImportanceFiltered => {
                // Savings from filtering out unimportant messages
                0.6  // ~60% savings from importance filtering
            }
            RetrievalStrategy::FullRetrieval => 0.0,  // No savings
            RetrievalStrategy::NoRetrieval => 0.0,    // No savings (but also no cost)
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
