//!
//! This module provides the server startup that uses thread-based
//! shared memory architecture. All API handlers access state through
//! Arc-wrapped shared memory (UnifiedAppState) â€” zero network hops
//! between components. The only network call is to the localhost llama-server.
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use crate::{
    config::Config,
    shared_state::{SharedState, UnifiedAppState},
    thread_pool::{ThreadPool, ThreadPoolConfig},
    worker_threads::{ContextWorker, CacheWorker, DatabaseWorker, LLMWorker},
    memory_db::MemoryDatabase,
};
/
#[derive(Clone)]
pub struct ThreadBasedAppState {
    pub shared_state: Arc<SharedState>,
    pub thread_pool: Arc<RwLock<Option<ThreadPool>>>,
    pub context_worker: Arc<ContextWorker>,
    pub cache_worker: Arc<CacheWorker>,
    pub database_worker: Arc<DatabaseWorker>,
    pub llm_worker: Arc<LLMWorker>,
}
/
pub async fn run_thread_server(cfg: Config) -> anyhow::Result<()> {
    crate::telemetry::init_tracing();
    crate::metrics::init_metrics();
    cfg.print_config();
    info!("Starting thread-based server architecture");

    let memory_db_path = std::path::Path::new("./data/conversations.db");
    let memory_database = match MemoryDatabase::new(memory_db_path) {
        Ok(db) => {
            info!("Memory database initialized at: {}", memory_db_path.display());
            Arc::new(db)
        }
        Err(e) => {
            warn!("Failed to initialize memory database: {}. Falling back to in-memory.", e);
            Arc::new(MemoryDatabase::new_in_memory()?)
        }
    };

    let shared_state = Arc::new(SharedState::new(cfg.clone(), memory_database.clone())?);

    info!("ðŸš€ Initializing Runtime Manager for multi-format model support");
    let runtime_manager = Arc::new(crate::model_runtime::RuntimeManager::new());


    let runtime_config = crate::model_runtime::RuntimeConfig {
        model_path: std::path::PathBuf::from(&cfg.model_path),
        format: crate::model_runtime::ModelFormat::GGUF,
        host: cfg.llama_host.clone(),
        port: cfg.llama_port,
        context_size: cfg.ctx_size,
        batch_size: cfg.batch_size,
        threads: cfg.threads,
        gpu_layers: cfg.gpu_layers,
        runtime_binary: Some(std::path::PathBuf::from(&cfg.llama_bin)),
        extra_config: serde_json::json!({}),
    };


    match runtime_manager.initialize_auto(runtime_config).await {
        Ok(base_url) => {
            info!("âœ… Model runtime initialized successfully");
            info!("   Runtime endpoint: {}", base_url);

        }
        Err(e) => {
            warn!("âš ï¸  Runtime initialization failed: {}", e);
            warn!("   The system will attempt to use the configured backend_url directly");
        }
    }

    let context_worker: Arc<ContextWorker> = Arc::new(ContextWorker::new(shared_state.clone()));
    let cache_worker: Arc<CacheWorker> = Arc::new(CacheWorker::new(shared_state.clone()));
    let database_worker: Arc<DatabaseWorker> = Arc::new(DatabaseWorker::new(shared_state.clone()));
    let llm_worker = shared_state.llm_worker.clone();

    let cache_manager = match crate::cache_management::create_default_cache_manager(
        crate::cache_management::KVCacheConfig::default(),
        memory_database.clone(),
    ) {
        Ok(manager) => {
            info!("Cache manager initialized successfully");
            Some(Arc::new(manager))
        }
        Err(e) => {
            warn!("Failed to initialize cache manager: {}, cache features disabled", e);
            None
        }
    };

    let context_orchestrator = match crate::context_engine::create_default_orchestrator(
        memory_database.clone(),
    ).await {
        Ok(mut orchestrator) => {


            orchestrator.set_llm_worker(shared_state.llm_worker.clone());
            info!("Context orchestrator initialized with semantic search support");
            Some(orchestrator)
        }
        Err(e) => {
            warn!("Failed to initialize context orchestrator: {}. Memory features disabled.", e);
            None
        }
    };

    let thread_pool_config = ThreadPoolConfig::new(&cfg);
    let mut thread_pool = ThreadPool::new(thread_pool_config, shared_state.clone());
    thread_pool.start().await?;

    {
        let mut cache_guard = shared_state.cache_manager.write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire cache manager write lock"))?;
        *cache_guard = cache_manager;


    }


    if let Err(e) = shared_state.database_pool.embeddings.initialize_index("llama-server") {
        debug!("Embedding index init: {} (will build on first embedding store)", e);
    } else {
        info!("Embedding HNSW index loaded from existing data");
    }

    {
        let mut orch_guard = shared_state.context_orchestrator.write().await;
        *orch_guard = context_orchestrator;
    }

    let unified_state = UnifiedAppState::new(shared_state.clone());

    info!("Starting HTTP server on {}:{}", cfg.api_host, cfg.api_port);
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", cfg.api_host, cfg.api_port)).await?;
    let app = build_compatible_router(unified_state);
    axum::serve(listener, app).await?;
    Ok(())
}
/
fn build_compatible_router(state: UnifiedAppState) -> axum::Router {
    use axum::{
        Router,
        routing::{get, post, put, delete},
    };
    use tower_http::{
        cors::{Any, CorsLayer},
        trace::TraceLayer,
        timeout::TimeoutLayer,
    };
    use std::time::Duration;
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
        .allow_headers(Any);
    Router::new()

        .route("/generate/stream", post(crate::api::stream_api::generate_stream))

        .route("/generate/title", post(crate::api::title_api::generate_title))

        .route("/conversations", get(crate::api::conversation_api::get_conversations))
        .route("/conversations/:id", get(crate::api::conversation_api::get_conversation))
        .route("/conversations/:id/title", put(crate::api::conversation_api::update_conversation_title))
        .route("/conversations/:id/pinned", post(crate::api::conversation_api::update_conversation_pinned))
        .route("/conversations/:id", delete(crate::api::conversation_api::delete_conversation))
        .route("/healthz", get(|| async { "OK" }))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(600)))
        .with_state(state)
}


