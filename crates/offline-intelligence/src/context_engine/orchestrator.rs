use crate::memory::Message;
use crate::memory_db::MemoryDatabase;
use crate::memory_db::schema::Embedding;
use crate::context_engine::{
    retrieval_planner::RetrievalPlan,
    retrieval_planner::RetrievalPlanner,
    tier_manager::{TierManager, TierManagerConfig},
    context_builder::{ContextBuilder, ContextBuilderConfig},
};
use crate::worker_threads::LLMWorker;
use std::sync::Arc;
use tracing::{info, debug, warn};
use tokio::sync::RwLock;
/
pub struct ContextOrchestrator {
    database: Arc<MemoryDatabase>,
    retrieval_planner: Arc<RwLock<RetrievalPlanner>>,
    tier_manager: Arc<RwLock<TierManager>>,
    context_builder: Arc<RwLock<ContextBuilder>>,
    config: OrchestratorConfig,
    /
    llm_worker: Option<Arc<LLMWorker>>,
}
/
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub enabled: bool,
    pub max_context_tokens: usize,
    pub auto_optimize: bool,
    pub enable_metrics: bool,
    pub session_timeout_seconds: u64,
}
impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_context_tokens: 4000,
            auto_optimize: true,
            enable_metrics: true,
            session_timeout_seconds: 3600,
        }
    }
}
impl ContextOrchestrator {
    /
    pub async fn new(
        database: Arc<MemoryDatabase>,
        config: OrchestratorConfig,
    ) -> anyhow::Result<Self> {

        let retrieval_planner = Arc::new(RwLock::new(RetrievalPlanner::new(database.clone())));


        let tier_manager_config = TierManagerConfig::default();
        let tier_manager = TierManager::new(
            database.clone(),
            tier_manager_config,
        );
        let tier_manager = Arc::new(RwLock::new(tier_manager));


        let context_builder_config = ContextBuilderConfig::default();
        let context_builder = Arc::new(RwLock::new(ContextBuilder::new(context_builder_config)));

        let orchestrator = Self {
            database,
            retrieval_planner,
            tier_manager,
            context_builder,
            config,
            llm_worker: None,
        };
        info!("Context orchestrator initialized successfully");
        Ok(orchestrator)
    }
    /
    pub fn set_llm_worker(&mut self, worker: Arc<LLMWorker>) {
        self.llm_worker = Some(worker);
        info!("Context orchestrator: LLM worker set for semantic search");
    }

    /
    pub fn database(&self) -> &Arc<MemoryDatabase> {
        &self.database
    }

    /
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


        if let Some(last_message) = messages.last() {
            if last_message.role == "user" {
                let tier_manager = self.tier_manager.read().await;
                if let Err(e) = tier_manager.store_tier3_content(session_id, std::slice::from_ref(last_message)).await {
                    warn!("Failed to persist user query to database: {}", e);
                } else {
                    info!("âœ… Persisted user query to database for session {}", session_id);
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


        let optimized_context = {
            let mut context_builder = self.context_builder.write().await;
            context_builder.build_context(
                messages,
                retrieved_content.tier1,
                retrieved_content.tier2,
                retrieved_content.tier3,
                retrieved_content.cross_session,
                user_query,
            ).await?
        };


        if let Some(query) = user_query {
            if let Some(response) = optimized_context.last() {
                if response.role == "assistant" {
                    self.update_engagement(query, &response.content).await;
                }
            }
        }

        info!(
            "Context optimization complete: {} -> {} messages",
            messages.len(),
            optimized_context.len()
        );

        Ok(optimized_context)
    }

    /
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

    /
    /
    /
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

        if plan.use_tier2 {
            let tier_manager = self.tier_manager.read().await;
            retrieved.tier2 = tier_manager.get_tier2_content(session_id).await;
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

    /
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
impl Clone for ContextOrchestrator {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            retrieval_planner: self.retrieval_planner.clone(),
            tier_manager: self.tier_manager.clone(),
            context_builder: self.context_builder.clone(),
            config: self.config.clone(),
            llm_worker: self.llm_worker.clone(),
        }
    }
}
#[derive(Debug, Default)]
struct RetrievedContent {
    tier1: Option<Vec<Message>>,
    tier2: Option<Vec<crate::memory_db::Summary>>,
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

