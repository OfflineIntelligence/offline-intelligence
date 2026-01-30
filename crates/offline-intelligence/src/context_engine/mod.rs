pub mod retrieval_planner;
pub mod tier_manager;
pub mod context_builder;
pub mod orchestrator;
pub use retrieval_planner::{RetrievalPlanner, RetrievalPlan};
pub use tier_manager::{TierManager, TierManagerConfig, TierStats};
pub use context_builder::{ContextBuilder, ContextBuilderConfig};
pub use orchestrator::{ContextOrchestrator, OrchestratorConfig, SessionStats, CleanupStats};
/
pub async fn create_default_orchestrator(
    database: std::sync::Arc<crate::memory_db::MemoryDatabase>,
) -> anyhow::Result<ContextOrchestrator> {
    let config = OrchestratorConfig::default();
    ContextOrchestrator::new(database, config).await
}

