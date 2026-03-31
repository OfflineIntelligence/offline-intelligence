
use crate::memory::Message;
use crate::memory_db::MemoryDatabase;
use crate::context_engine::{
    retrieval_planner::RetrievalPlan,
    retrieval_planner::RetrievalPlanner,
    tier_manager::{TierManager, TierManagerConfig},
    context_builder::{ContextBuilder, ContextBuilderConfig},
    smart_retrieval::{SmartRetrieval, SmartRetrievalConfig},
};
use crate::worker_threads::LLMWorker;

use std::sync::Arc;
use tracing::{info, debug, warn};
use tokio::sync::RwLock;

pub struct ContextOrchestrator {
    database: Arc<MemoryDatabase>,
    retrieval_planner: Arc<RwLock<RetrievalPlanner>>,
    tier_manager: Arc<RwLock<TierManager>>,
    context_builder: Arc<RwLock<ContextBuilder>>,
    config: OrchestratorConfig,
    
    llm_worker: Option<Arc<LLMWorker>>,
    
    smart_retrieval: Option<Arc<SmartRetrieval>>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub enabled: bool,
    pub max_context_tokens: usize,
    pub auto_optimize: bool,
    pub enable_metrics: bool,
    pub session_timeout_seconds: u64,
    
    pub enable_smart_retrieval: bool,
    
    pub smart_retrieval_config: SmartRetrievalConfig,
    
    pub ctx_size: u32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_context_tokens: 4000,
            auto_optimize: true,
            enable_metrics: true,
            session_timeout_seconds: 3600,
            enable_smart_retrieval: true,  
            smart_retrieval_config: SmartRetrievalConfig::default(),
            ctx_size: 0,
        }
    }
}

impl OrchestratorConfig {
    
    pub fn from_ctx_size(ctx_size: u32) -> Self {
        let max_context_tokens = (ctx_size as f32 * 0.75) as usize;
        Self {
            max_context_tokens,
            smart_retrieval_config: SmartRetrievalConfig::from_ctx_size(ctx_size),
            ctx_size,
            ..Self::default()
        }
    }
}

impl ContextOrchestrator {
    
    pub async fn new(
        database: Arc<MemoryDatabase>,
        config: OrchestratorConfig,
    ) -> anyhow::Result<Self> {
        
        let retrieval_planner = Arc::new(RwLock::new(RetrievalPlanner::new(database.clone())));
        
        let tier_manager_config = if config.ctx_size > 0 {
            TierManagerConfig::from_ctx_size(config.ctx_size)
        } else {
            TierManagerConfig::default()
        };
        let tier_manager = TierManager::new(
            database.clone(),
            tier_manager_config,
        );
        let tier_manager = Arc::new(RwLock::new(tier_manager));

        let context_builder_config = if config.ctx_size > 0 {
            ContextBuilderConfig::from_ctx_size(config.ctx_size)
        } else {
            ContextBuilderConfig::default()
        };
        let context_builder = Arc::new(RwLock::new(ContextBuilder::new(context_builder_config)));
        
        let smart_retrieval = if config.enable_smart_retrieval {
            let smart_ret = SmartRetrieval::new(
                Arc::clone(&tier_manager),
                config.smart_retrieval_config.clone(),
            );
            info!("Smart retrieval initialized (enabled)");
            Some(Arc::new(smart_ret))
        } else {
            info!("Smart retrieval disabled");
            None
        };

        let orchestrator = Self {
            database,
            retrieval_planner,
            tier_manager,
            context_builder,
            config,
            llm_worker: None,
            smart_retrieval,
        };

        info!("Context orchestrator initialized successfully");

        Ok(orchestrator)
    }

    pub fn set_llm_worker(&mut self, worker: Arc<LLMWorker>) {
        self.llm_worker = Some(worker);
        info!("Context orchestrator: LLM worker set for semantic search");
    }
    
    pub fn database(&self) -> &Arc<MemoryDatabase> {
        &self.database
    }
    
    pub async fn process_conversation(
        &self,
        session_id: &str,
        messages: &[Message],
        user_query: Option<&str>,
    ) -> anyhow::Result<Vec<Message>> {
        if !self.config.enabled || messages.is_empty() {
            debug!("Context engine disabled or no messages");
            return Ok(messages.to_vec());
        }
        
        info!("Processing conversation for session {} ({} messages)", session_id, messages.len());
        
        {
            let tier_manager = self.tier_manager.write().await;
            tier_manager.store_tier1_content(session_id, messages).await;
        }

        let estimated_tokens: usize = messages.iter().map(|m| m.content.len() / 4).sum();
        let summary_threshold = (self.config.max_context_tokens as f32 * 0.60) as usize;
        if estimated_tokens >= summary_threshold {
            if let Some(worker) = self.llm_worker.clone() {
                let db = Arc::clone(&self.database);
                let sid = session_id.to_string();
                let msgs = messages.to_vec();
                tokio::spawn(async move {
                    generate_and_store_summary(&db, &worker, &sid, &msgs).await;
                });
            }
        }

        if let Some(last_message) = messages.last() {
            if last_message.role == "user" {
                let tier_manager = self.tier_manager.read().await;
                if let Err(e) = tier_manager.store_tier3_content(session_id, std::slice::from_ref(last_message)).await {
                    warn!("Failed to persist user query to database: {}", e);
                } else {
                    info!("✅ Persisted user query to database for session {}", session_id);
                }
            }
        }
        
        let plan = {
            let retrieval_planner = self.retrieval_planner.read().await;
            
            let has_past_refs = if let Some(query) = user_query {
                retrieval_planner.has_past_references_in_text(query)
            } else {
                false
            };
            
            retrieval_planner.create_plan(
                session_id,
                messages,
                self.config.max_context_tokens,
                user_query,
                has_past_refs, 
            ).await?
        };
        
        if !plan.needs_retrieval {
            debug!("No retrieval needed, returning current messages");
            return Ok(messages.to_vec());
        }
        
        let retrieved_content = self.execute_retrieval_plan(session_id, &plan, user_query).await?;

        let optimized_context = if let Some(ref smart_retrieval) = self.smart_retrieval {
            match smart_retrieval.retrieve(
                session_id,
                messages,
                retrieved_content.tier3.clone(),
                retrieved_content.cross_session.clone(),
            ).await {
                Ok(smart_result) => {
                    info!(
                        "🎯 Smart retrieval: Strategy={:?}, Tokens={}, Savings={:.1}%",
                        smart_result.strategy,
                        smart_result.retrieved_tokens,
                        smart_result.compute_savings * 100.0
                    );
                    smart_result.messages
                }
                Err(e) => {
                    warn!("Smart retrieval failed, falling back to standard: {}", e);
                    let mut context_builder = self.context_builder.write().await;
                    context_builder.build_context(
                        messages,
                        retrieved_content.tier1,
                        retrieved_content.tier3,
                        retrieved_content.cross_session,
                        user_query,
                    ).await?
                }
            }
        } else {
            
            let mut context_builder = self.context_builder.write().await;
            context_builder.build_context(
                messages,
                retrieved_content.tier1,
                retrieved_content.tier3,
                retrieved_content.cross_session,
                user_query,
            ).await?
        };
        
        let mut final_context = self.prepend_session_summary(session_id, optimized_context).await;

        if let Some(query) = user_query {
            if let Some(response) = final_context.last() {
                if response.role == "assistant" {
                    self.update_engagement(query, &response.content).await;
                }
            }
        }

        info!(
            "Context optimization complete: {} -> {} messages",
            messages.len(),
            final_context.len()
        );

        Ok(final_context)
    }
    
    async fn prepend_session_summary(
        &self,
        session_id: &str,
        mut context: Vec<Message>,
    ) -> Vec<Message> {
        match self.database.session_summaries.get(session_id) {
            Ok(Some(summary)) => {
                debug!(
                    "Prepending cumulative summary for session {} (clear #{}, {} tokens)",
                    session_id, summary.clear_count, summary.token_count
                );
                context.insert(0, Message {
                    role: "system".to_string(),
                    content: format!(
                        "[Conversation history summary — covers everything before this window:]\n{}",
                        summary.summary_text
                    ),
                });
                context
            }
            Ok(None) => context,
            Err(e) => {
                debug!("Could not fetch summary for session {}: {}", session_id, e);
                context
            }
        }
    }

    pub async fn save_assistant_response(
        &self,
        session_id: &str,
        response: &str,
    ) -> anyhow::Result<()> {
        let assistant_message = Message {
            role: "assistant".to_string(),
            content: response.to_string(),
        };
        
        let tier_manager = self.tier_manager.read().await;
        tier_manager.store_tier3_content(session_id, &[assistant_message]).await
    }
    
    async fn execute_retrieval_plan(
        &self,
        session_id: &str,
        plan: &RetrievalPlan,
        user_query: Option<&str>,
    ) -> anyhow::Result<RetrievedContent> {
        let mut retrieved = RetrievedContent::default();

        if plan.use_tier1 {
            let tier_manager = self.tier_manager.read().await;
            retrieved.tier1 = tier_manager.get_tier1_content(session_id).await;
        }

        let mut semantic_results: Vec<crate::memory_db::StoredMessage> = Vec::new();

        let has_embeddings = self.database.embeddings.get_stats()
            .map(|s| s.total_embeddings > 0)
            .unwrap_or(false);

        if plan.semantic_search && has_embeddings {
            if let (Some(ref llm_worker), Some(query)) = (&self.llm_worker, user_query) {
                match llm_worker.generate_embeddings(vec![query.to_string()]).await {
                    Ok(query_embeddings) if !query_embeddings.is_empty() => {
                        let query_vec = &query_embeddings[0];
                        
                        match self.database.embeddings.find_similar_embeddings(
                            query_vec,
                            "llama-server",
                            (plan.max_messages * 2) as i32,
                            0.3, 
                        ) {
                            Ok(similar) if !similar.is_empty() => {
                                info!("Semantic search found {} similar messages for context retrieval", similar.len());
                                
                                for (message_id, _similarity) in &similar {
                                    
                                    let conn = self.database.conversations.get_conn_public();
                                    if let Ok(conn) = conn {
                                        let mut stmt = conn.prepare(
                                            "SELECT id, session_id, message_index, role, content, tokens,
                                                    timestamp, importance_score, embedding_generated
                                             FROM messages WHERE id = ?1"
                                        ).ok();
                                        if let Some(ref mut stmt) = stmt {
                                            if let Ok(mut rows) = stmt.query([message_id]) {
                                                if let Ok(Some(row)) = rows.next() {
                                                    let ts_str: String = row.get(6).unwrap_or_default();
                                                    let ts = chrono::DateTime::parse_from_rfc3339(&ts_str)
                                                        .map(|dt| dt.with_timezone(&chrono::Utc))
                                                        .unwrap_or_else(|_| chrono::Utc::now());
                                                    semantic_results.push(crate::memory_db::StoredMessage {
                                                        id: row.get(0).unwrap_or(0),
                                                        session_id: row.get(1).unwrap_or_default(),
                                                        message_index: row.get(2).unwrap_or(0),
                                                        role: row.get(3).unwrap_or_default(),
                                                        content: row.get(4).unwrap_or_default(),
                                                        tokens: row.get(5).unwrap_or(0),
                                                        timestamp: ts,
                                                        importance_score: row.get(7).unwrap_or(0.5),
                                                        embedding_generated: row.get(8).unwrap_or(true),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(_) => debug!("Semantic search: no results above threshold"),
                            Err(e) => debug!("Semantic search failed: {}", e),
                        }
                    }
                    Ok(_) => debug!("Empty embedding response for query"),
                    Err(e) => debug!("Query embedding generation failed (semantic search skipped): {}", e),
                }
            }
        }

        if plan.use_tier3 {
            let tier_manager = self.tier_manager.read().await;
            if plan.keyword_search && !plan.search_topics.is_empty() {
                for topic in &plan.search_topics {
                    let limit_per_topic = plan.max_messages / plan.search_topics.len().max(1);

                    if let Ok(results) = tier_manager.search_tier3_content(
                        session_id,
                        topic,
                        limit_per_topic,
                    ).await {
                        
                        let semantic_ids: std::collections::HashSet<i64> = semantic_results.iter().map(|m| m.id).collect();
                        let mut merged = semantic_results.clone();
                        for msg in results {
                            if !semantic_ids.contains(&msg.id) {
                                merged.push(msg);
                            }
                        }
                        retrieved.tier3 = Some(merged);
                        break;
                    }
                }
                
                if retrieved.tier3.is_none() && !semantic_results.is_empty() {
                    retrieved.tier3 = Some(semantic_results.clone());
                }
            } else {
                if !semantic_results.is_empty() {
                    
                    retrieved.tier3 = Some(semantic_results.clone());
                } else {
                    retrieved.tier3 = tier_manager.get_tier3_content(
                        session_id,
                        Some((plan.max_messages as i64).min(i32::MAX as i64) as i32),
                        Some(0),
                    ).await.ok();
                }
            }
        } else if !semantic_results.is_empty() {
            
            retrieved.tier3 = Some(semantic_results);
        }

        if plan.cross_session_search && !plan.search_topics.is_empty() {
            let tier_manager = self.tier_manager.read().await;
            if let Ok(cross_session_results) = tier_manager.search_cross_session_content(
                session_id,
                &plan.search_topics.join(" "),
                10,
            ).await {
                retrieved.cross_session = Some(cross_session_results);
            }
        }

        Ok(retrieved)
    }
    
    async fn update_engagement(&self, user_query: &str, assistant_response: &str) {
        debug!("Engagement updated for query: {} (response length: {})", 
               user_query, assistant_response.len());
    }
    
    pub async fn get_session_stats(&self, session_id: &str) -> anyhow::Result<SessionStats> {
        let tier_manager = self.tier_manager.read().await;
        let tier_stats = tier_manager.get_tier_stats(session_id).await;
        let db_stats = self.database.get_stats()?;
        
        Ok(SessionStats {
            session_id: session_id.to_string(),
            tier_stats,
            database_stats: db_stats,
        })
    }
    
    pub async fn cleanup(&self, older_than_seconds: u64) -> anyhow::Result<CleanupStats> {
        info!("Starting cleanup of old data");
        let db_cleaned = self.database.cleanup_old_data((older_than_seconds / 86400) as i32)?;
        let tier_manager = self.tier_manager.read().await;
        let cache_cleaned = tier_manager.cleanup_cache(older_than_seconds).await;
        
        Ok(CleanupStats {
            sessions_cleaned: db_cleaned,
            cache_entries_cleaned: cache_cleaned,
        })
    }
    
    pub async fn search_messages(
        &self,
        session_id: Option<&str>,
        keywords: &[String],
        limit: usize,
    ) -> anyhow::Result<Vec<crate::memory_db::StoredMessage>> {
        if keywords.is_empty() {
            return Ok(Vec::new());
        }
        
        if let Some(sid) = session_id {
            
            self.database.search_messages_by_keywords(sid, keywords, limit).await
        } else {
            
            Ok(Vec::new())
        }
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        info!("Context engine {}", if enabled { "enabled" } else { "disabled" });
    }
    
    pub fn update_config(&mut self, config: OrchestratorConfig) {
        self.config = config;
        info!("Context engine configuration updated");
    }
    
    pub fn get_config(&self) -> &OrchestratorConfig {
        &self.config
    }

    pub fn tier_manager(&self) -> &Arc<RwLock<TierManager>> {
        &self.tier_manager
    }
}

async fn generate_and_store_summary(
    database: &Arc<crate::memory_db::MemoryDatabase>,
    llm_worker: &Arc<LLMWorker>,
    session_id: &str,
    messages: &[Message],
) {
    if messages.len() < 4 {
        return;
    }

    let existing = database.session_summaries.get(session_id).unwrap_or(None);

    let system_content = match &existing {
        Some(prev) => format!(
            "You are a concise summarizer. You have a running summary of a conversation \
             and new messages that occurred since that summary. Produce ONE updated summary \
             covering EVERYTHING — the prior summary and the new messages combined. \
             Target under 400 tokens. Include key facts, decisions, code, numbers, names. \
             No commentary.\n\nPRIOR SUMMARY:\n{}",
            prev.summary_text
        ),
        None => "You are a concise summarizer. Summarize the following conversation \
                 into key facts, decisions, code snippets, and figures. \
                 Target under 300 tokens. No commentary.".to_string(),
    };

    let mut context: Vec<Message> = vec![Message {
        role: "system".to_string(),
        content: system_content,
    }];

    let tail = if messages.len() > 40 { &messages[messages.len() - 40..] } else { messages };
    context.extend_from_slice(tail);

    let user_prompt = if existing.is_some() {
        "Produce the updated cumulative summary now, covering both the prior summary and these new messages."
    } else {
        "Summarize the conversation above."
    };
    context.push(Message { role: "user".to_string(), content: user_prompt.to_string() });

    match llm_worker.generate_response(session_id.to_string(), context).await {
        Ok(summary) if !summary.trim().is_empty() => {
            let token_estimate = (summary.len() / 4) as i32;
            let clear_num = existing.as_ref().map(|s| s.clear_count + 1).unwrap_or(1);
            match database.session_summaries.upsert(
                session_id, &summary, token_estimate, messages.len() as i32,
            ) {
                Ok(_) => info!(
                    "Background: updated cumulative summary #{} for session {} ({} tokens)",
                    clear_num, session_id, token_estimate
                ),
                Err(e) => info!("Background: could not persist summary for {}: {}", session_id, e),
            }
        }
        Ok(_) => debug!("Background: summary was empty for session {}", session_id),
        Err(e) => debug!("Background: summary skipped for {}: {}", session_id, e),
    }
}

impl Clone for ContextOrchestrator {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            retrieval_planner: self.retrieval_planner.clone(),
            tier_manager: self.tier_manager.clone(),
            context_builder: self.context_builder.clone(),
            config: self.config.clone(),
            llm_worker: self.llm_worker.clone(),
            smart_retrieval: self.smart_retrieval.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct RetrievedContent {
    tier1: Option<Vec<Message>>,
    tier3: Option<Vec<crate::memory_db::StoredMessage>>,
    cross_session: Option<Vec<crate::memory_db::StoredMessage>>,
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub tier_stats: crate::context_engine::tier_manager::TierStats,
    pub database_stats: crate::memory_db::schema::DatabaseStats,
}

#[derive(Debug, Clone)]
pub struct CleanupStats {
    pub sessions_cleaned: usize,
    pub cache_entries_cleaned: usize,
}
