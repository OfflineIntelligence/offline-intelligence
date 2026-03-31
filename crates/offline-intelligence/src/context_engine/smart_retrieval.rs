
use crate::memory::Message;
use crate::memory_db::StoredMessage;
use crate::context_engine::tier_manager::TierManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

#[derive(Debug, Clone)]
pub struct SmartRetrievalConfig {
    
    pub max_retrieved_tokens: usize,

    pub importance_threshold: f32,

    pub chunk_contiguous_messages: bool,

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
    
    pub fn from_ctx_size(ctx_size: u32) -> Self {
        Self {
            max_retrieved_tokens: (ctx_size as f32 * 0.25) as usize,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetrievalResult {
    
    pub strategy: RetrievalStrategy,

    pub messages: Vec<Message>,

    pub compute_savings: f32,

    pub retrieved_tokens: usize,

    pub sessions_referenced: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RetrievalStrategy {
    
    HotCacheHit,

    ImportanceFiltered,

    FullRetrieval,

    NoRetrieval,
}

pub struct SmartRetrieval {
    tier_manager: Arc<RwLock<TierManager>>,
    config: SmartRetrievalConfig,
}

impl SmartRetrieval {
    
    pub fn new(tier_manager: Arc<RwLock<TierManager>>, config: SmartRetrievalConfig) -> Self {
        Self {
            tier_manager,
            config,
        }
    }

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

        if let Some(tier3_msgs) = tier3_messages {
            let remaining_budget = budget_for_history.saturating_sub(retrieved_tokens);
            let detail_context = self.add_important_details(tier3_msgs, remaining_budget);
            retrieved_tokens += self.count_tokens(&detail_context);
            context.extend(detail_context);
        }

        context.extend_from_slice(current_messages);

        Ok(RetrievalResult {
            strategy: RetrievalStrategy::ImportanceFiltered,
            messages: context,
            compute_savings: 0.0,
            retrieved_tokens,
            sessions_referenced,
        })
    }

    fn add_cross_session_context(
        &self,
        cross_messages: &[StoredMessage],
        token_budget: usize,
    ) -> Vec<Message> {
        let mut context = Vec::new();
        let mut used_tokens = 0;

        context.push(Message {
            role: "system".to_string(),
            content: "[Context from previous conversations]".to_string(),
        });
        used_tokens += 8;

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

    fn add_important_details(
        &self,
        messages: &[StoredMessage],
        token_budget: usize,
    ) -> Vec<Message> {
        let mut context = Vec::new();
        let mut used_tokens = 0;

        let important: Vec<_> = messages.iter()
            .filter(|m| m.importance_score >= self.config.importance_threshold)
            .collect();

        if important.is_empty() {
            debug!("No messages meet importance threshold {}", self.config.importance_threshold);
            return context;
        }

        let mut scored = important.clone();
        scored.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap_or(std::cmp::Ordering::Equal));

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

    fn estimate_compute_savings(&self, strategy: &RetrievalStrategy, _messages: &[Message]) -> f32 {
        match strategy {
            RetrievalStrategy::HotCacheHit => 1.0,
            RetrievalStrategy::ImportanceFiltered => 0.6,
            RetrievalStrategy::FullRetrieval => 0.0,
            RetrievalStrategy::NoRetrieval => 0.0,
        }
    }

    fn count_tokens(&self, messages: &[Message]) -> usize {
        messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum()
    }

    fn estimate_message_tokens(&self, message: &Message) -> usize {
        message.content.len() / 4
    }

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
