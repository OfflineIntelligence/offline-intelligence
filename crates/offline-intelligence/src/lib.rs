// OFFLINE INTELLIGENCE LIBRARY
// Core Open-Source Components (80% Public API)
// Proprietary Extensions Available Separately

// Core Infrastructure Modules
pub mod config;
pub mod metrics;
pub mod telemetry;
pub mod utils;

// Core LLM Integration
pub mod llm_integration;

// Core API Interfaces
pub mod api;

// Core Memory Management (Base Layer)
pub mod memory;
pub mod memory_db;

// Core Proxy Functionality
pub mod proxy;

// Core Administration
pub mod admin;

// Core Resource Management
pub mod resources;

// PRIVATE COMPONENTS - Proprietary Extensions
// These modules are intentionally NOT exported publicly
// They can be accessed via extension crates or plugins
// mod context_engine;     // Advanced context management
// mod cache_management;   // KV cache system

// Public re-exports for core functionality
pub use config::*;
pub use llm_integration::*;
pub use metrics::*;
pub use proxy::*;
pub use admin::*;

use axum::{
    Router,
    routing::{get, post},
    extract::{State, FromRef, Path},
    response::IntoResponse,
    Json,
};
use axum::http::Method;
use std::{path::Path as StdPath, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    timeout::TimeoutLayer,
};
use tracing::{info, warn, error};

// Removed proprietary dependencies
// use context_engine::ContextOrchestrator;
use memory_db::MemoryDatabase;
// use cache_management::KVCacheManager;

// Simplified AppState without proprietary components
#[derive(Clone)]
pub struct UnifiedAppState {
    pub proxy: proxy::AppState,
    pub admin: admin::AdminState,
    pub llm_engine: Arc<LLMEngine>,
}

impl FromRef<UnifiedAppState> for proxy::AppState {
    fn from_ref(state: &UnifiedAppState) -> Self {
        state.proxy.clone()
    }
}

impl FromRef<UnifiedAppState> for admin::AdminState {
    fn from_ref(state: &UnifiedAppState) -> Self {
        state.admin.clone()
    }
}

impl FromRef<UnifiedAppState> for Arc<LLMEngine> {
    fn from_ref(state: &UnifiedAppState) -> Self {
        state.llm_engine.clone()
    }
}

async fn health_check() -> &'static str {
    "OK"
}

async fn ready_check() -> &'static str {
    "READY"
}

async fn get_status_wrapper(
    State(state): State<UnifiedAppState>,
) -> impl IntoResponse {
    admin::get_status(State(state.admin)).await
}

async fn load_model_wrapper(
    State(state): State<UnifiedAppState>,
    Json(req): Json<admin::LoadModelRequest>,
) -> impl IntoResponse {
    admin::load_model(State(state.admin), Json(req)).await
}

async fn stop_backend_wrapper(
    State(state): State<UnifiedAppState>,
) -> impl IntoResponse {
    admin::stop_backend(State(state.admin)).await
}

async fn memory_stats_wrapper(
    State(state): State<UnifiedAppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    api::memory_stats(State(state), Path(session_id)).await
}

async fn memory_optimize_wrapper(
    State(state): State<UnifiedAppState>,
    Json(req): Json<api::memory_api::MemoryOptimizeRequest>,
) -> impl IntoResponse {
    api::memory_optimize(State(state), Json(req)).await
}

async fn memory_cleanup_wrapper(
    State(state): State<UnifiedAppState>,
    Json(req): Json<api::memory_api::MemoryCleanupRequest>,
) -> impl IntoResponse {
    api::memory_cleanup(State(state), Json(req)).await
}

async fn init_cache_manager(
    _memory_database: Arc<MemoryDatabase>,
) -> anyhow::Result<Option<Arc<()>>> {
    // Cache manager removed from core library - proprietary feature
    // Available as separate extension
    Ok(None)
}

// Core server functionality - Public API
pub async fn run_server(cfg: Config) -> anyhow::Result<()> {
    telemetry::init_tracing();
    metrics::init_metrics();
    cfg.print_config();
    
    // Initialize LLM engine for direct integration
    let llm_engine = Arc::new(LLMEngine::new(cfg.clone()));
    
    info!("🚀 Initializing LLM engine...");
    match llm_engine.initialize().await {
        Ok(()) => {
            info!("✅ LLM engine initialized successfully");
        }
        Err(e) => {
            error!("❌ Failed to initialize LLM engine: {}", e);
            return Err(e);
        }
    }
    
    let admin_state = admin::AdminState {
        cfg: cfg.clone(),
        llm_engine: llm_engine.clone(),
    };
    
    let memory_db_path = StdPath::new("./data/conversations.db");
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

    // Removed proprietary components initialization
    // let cache_manager = Arc::new(RwLock::new(init_cache_manager(memory_database.clone()).await?));
    // let context_orchestrator = match context_engine::create_default_orchestrator(memory_database.clone()).await {
    //     Ok(orchestrator) => {
    //         info!("Context orchestrator initialized successfully");
    //         Arc::new(RwLock::new(Some(orchestrator)))
    //     }
    //     Err(e) => {
    //         warn!("Failed to initialize context orchestrator: {}. Memory features disabled.", e);
    //         Arc::new(RwLock::new(None))
    //     }
    // };

    let proxy_state_simple = proxy::AppState {
        llm_engine: llm_engine.clone(),
        cfg: cfg.clone(),
        // Removed proprietary context orchestrator
        // context_orchestrator: context_orchestrator.clone(),
    };

    let unified_state = UnifiedAppState {
        proxy: proxy_state_simple,
        admin: admin_state,
        // Removed proprietary components
        // context_orchestrator,
        // cache_manager,
        llm_engine,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/generate/stream", post(proxy::generate_stream_endpoint))
        .route("/healthz", get(health_check))
        .route("/readyz", get(ready_check))
        .route("/metrics", get(metrics::get_metrics))
        .route("/admin/status", get(get_status_wrapper))
        .route("/admin/load", post(load_model_wrapper))
        .route("/admin/stop", post(stop_backend_wrapper))
        .route("/memory/optimize", post(memory_optimize_wrapper))
        .route("/memory/stats/:session_id", get(memory_stats_wrapper))
        .route("/memory/cleanup", post(memory_cleanup_wrapper))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(ConcurrencyLimitLayer::new(cfg.max_concurrent_streams as usize))
        .layer(TimeoutLayer::new(Duration::from_secs(cfg.generate_timeout_seconds)))
        .with_state(unified_state);

    info!("Starting server on {}:{}", cfg.api_host, cfg.api_port);
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", cfg.api_host, cfg.api_port)).await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}