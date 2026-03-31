
pub mod retrieval_planner;
pub mod tier_manager;
pub mod context_builder;
pub mod orchestrator;
pub mod smart_retrieval;

pub use retrieval_planner::{RetrievalPlanner, RetrievalPlan};
pub use tier_manager::{TierManager, TierManagerConfig, TierStats};
pub use context_builder::{ContextBuilder, ContextBuilderConfig};
pub use orchestrator::{ContextOrchestrator, OrchestratorConfig, SessionStats, CleanupStats};
pub use smart_retrieval::{SmartRetrieval, SmartRetrievalConfig, RetrievalResult, RetrievalStrategy};

pub async fn create_default_orchestrator(
    database: std::sync::Arc<crate::memory_db::MemoryDatabase>,
    ctx_size: u32,
) -> anyhow::Result<ContextOrchestrator> {
    let config = if ctx_size > 0 {
        OrchestratorConfig::from_ctx_size(ctx_size)
    } else {
        OrchestratorConfig::default()
    };
    ContextOrchestrator::new(database, config).await
}
