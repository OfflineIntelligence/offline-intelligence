
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};
use anyhow::anyhow;

use crate::{
    config::Config,
    shared_state::{SharedState, UnifiedAppState},
    thread_pool::{ThreadPool, ThreadPoolConfig},
    worker_threads::{ContextWorker, CacheWorker, DatabaseWorker, LLMWorker},
    memory_db::MemoryDatabase,
    model_management::ModelManager,
};

#[derive(Clone)]
pub struct ThreadBasedAppState {
    pub shared_state: Arc<SharedState>,
    pub thread_pool: Arc<RwLock<Option<ThreadPool>>>,
    pub context_worker: Arc<ContextWorker>,
    pub cache_worker: Arc<CacheWorker>,
    pub database_worker: Arc<DatabaseWorker>,
    pub llm_worker: Arc<LLMWorker>,
}

pub async fn run_thread_server(cfg: Config, port_tx: Option<std::sync::mpsc::Sender<u16>>) -> anyhow::Result<()> {
    crate::telemetry::init_tracing();
    crate::metrics::init_metrics();
    cfg.print_config();

    info!("Starting thread-based server architecture");

    let memory_db_path = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("Aud.io")
        .join("data")
        .join("memory.db");

    if let Some(parent) = memory_db_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!("Failed to create data directory {:?}: {}", parent, e);
        } else {
            info!("Created data directory: {:?}", parent);
        }
    }

    let memory_database = match MemoryDatabase::new(&memory_db_path) {
        Ok(db) => {
            info!("Memory database initialized at: {}", memory_db_path.display());
            Arc::new(db)
        }
        Err(e) => {
            
            error!(
                "❌ CRITICAL: Failed to open SQLite database at {}: {}\n\
                 Falling back to IN-MEMORY storage — all user data will be lost on exit.\n\
                 Check that the directory is writable and no other process holds a lock on the file.",
                memory_db_path.display(), e
            );
            Arc::new(MemoryDatabase::new_in_memory()?)
        }
    };

    let mut shared_state = SharedState::new(cfg.clone(), memory_database.clone())?;

    info!("📦 Initializing Model Manager");
    match ModelManager::new() {
        Ok(model_manager) => {
            let model_manager_arc = Arc::new(model_manager);
            if let Err(e) = model_manager_arc.initialize(&cfg).await {
                warn!("⚠️  Model manager initialization failed: {}", e);
                shared_state.model_manager = Some(model_manager_arc);
            } else {
                info!("✅ Model manager initialized successfully");
                shared_state.model_manager = Some(model_manager_arc);
            }
        }
        Err(e) => warn!("⚠️  Failed to create model manager: {}", e),
    }

    info!("⚙️  Initializing Engine Manager (blocking until ready)...");
    match crate::engine_management::EngineManager::new() {
        Ok(engine_manager) => {
            let engine_manager_arc = Arc::new(engine_manager);
            match engine_manager_arc.initialize(&cfg).await {
                Ok(true) => {
                    info!("✅ Engine manager initialized with engine ready");
                    shared_state.engine_manager = Some(engine_manager_arc.clone());
                    shared_state.engine_available.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(false) => {
                    
                    info!("⏳ No engine found — starting in online-only mode. Download from the Models page.");
                    shared_state.engine_manager = Some(engine_manager_arc);
                    shared_state.engine_available.store(false, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => {
                    warn!("⚠️  Engine manager scan failed: {}", e);
                    shared_state.engine_manager = Some(engine_manager_arc);
                    shared_state.engine_available.store(false, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to create engine manager: {}", e);
            shared_state.engine_available.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    let shared_state = Arc::new(shared_state);

    let _context_worker: Arc<ContextWorker> = Arc::new(ContextWorker::new(shared_state.clone()));
    let _cache_worker: Arc<CacheWorker> = Arc::new(CacheWorker::new(shared_state.clone()));
    let _database_worker: Arc<DatabaseWorker> = Arc::new(DatabaseWorker::new(shared_state.clone()));
    let _llm_worker = shared_state.llm_worker.clone();

    let cache_manager = match crate::cache_management::create_default_cache_manager(
        crate::cache_management::KVCacheConfig::from_ctx_size(cfg.ctx_size),
        memory_database.clone(),
        Some(shared_state.llm_worker.clone()),
    ) {
        Ok(manager) => { info!("Cache manager initialized"); Some(Arc::new(manager)) }
        Err(e) => { warn!("Cache manager failed: {}, disabled", e); None }
    };
    {
        let mut g = shared_state.cache_manager.write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire cache manager write lock"))?;
        *g = cache_manager;
    }

    if let Err(e) = shared_state.database_pool.embeddings.initialize_index("llama-server") {
        debug!("Embedding index init: {} (will build on first store)", e);
    } else {
        info!("Embedding HNSW index loaded from existing data");
    }

    let thread_pool_config = ThreadPoolConfig::new(&cfg);
    let mut thread_pool = ThreadPool::new(thread_pool_config, shared_state.clone());
    thread_pool.start().await?;

    let unified_state = UnifiedAppState::new(shared_state.clone());
    let app = build_compatible_router(unified_state);

    let (listener, selected_port) = match try_bind_port(&cfg.api_host, cfg.api_port).await {
        Ok(listener) => {
            let port = listener.local_addr()?.port();
            info!("✅ HTTP server bound to {}:{}", cfg.api_host, port);
            (listener, port)
        }
        Err(e) => {
            warn!("⚠️ Failed to bind to port {}: {}", cfg.api_port, e);
            warn!("🔄 Scanning 8002-8999 for available port...");
            let mut last_error = None;
            let mut found_listener = None;
            let mut found_port = 0u16;
            for port in 8002u16..=8999 {
                match try_bind_port(&cfg.api_host, port).await {
                    Ok(listener) => {
                        found_port = listener.local_addr()?.port();
                        info!("✅ HTTP server bound to alternative port {}", found_port);
                        if let Ok(mut g) = shared_state.http_port.write() { *g = found_port; }
                        found_listener = Some(listener);
                        break;
                    }
                    Err(e) => { last_error = Some(e); }
                }
            }
            let listener = found_listener.ok_or_else(|| anyhow!(
                "Failed to find available port after scanning 8002-8999.\n  Last error: {:?}\n  Hints: disable firewall, close other Aud.io instances, or run as Administrator.",
                last_error
            ))?;
            (listener, found_port)
        }
    };

    if let Some(ref tx) = port_tx {
        if let Err(e) = tx.send(selected_port) {
            warn!("Failed to send port to main thread: {}", e);
        } else {
            info!("✅ Port {} communicated to main thread", selected_port);
        }
    }
    info!("🌐 Server will accept connections on port {}", selected_port);

    {
        let shared_state_bg = shared_state.clone();
        let cfg_bg = cfg.clone();
        let memory_database_bg = memory_database.clone();
        tokio::spawn(async move {
            
            shared_state_bg.mark_initialization_complete();
            info!("✅ Backend marked as initialized — frontend may proceed");

            let context_orchestrator = match crate::context_engine::create_default_orchestrator(
                memory_database_bg,
                cfg_bg.ctx_size,
            ).await {
                Ok(mut orchestrator) => {
                    orchestrator.set_llm_worker(shared_state_bg.llm_worker.clone());
                    info!("Context orchestrator initialized");
                    Some(orchestrator)
                }
                Err(e) => {
                    warn!("Context orchestrator failed: {}. Memory features disabled.", e);
                    None
                }
            };
            {
                let mut g = shared_state_bg.context_orchestrator.write().await;
                *g = context_orchestrator;
            }

            info!("🚀 Initializing Runtime Manager");
            let runtime_manager = Arc::new(crate::model_runtime::RuntimeManager::new());
            let runtime_config = crate::model_runtime::RuntimeConfig {
                model_path: std::path::PathBuf::from(&cfg_bg.model_path),
                format: crate::model_runtime::ModelFormat::GGUF,
                host: cfg_bg.llama_host.clone(),
                port: cfg_bg.llama_port,
                context_size: cfg_bg.ctx_size,
                batch_size: cfg_bg.batch_size,
                threads: cfg_bg.threads,
                gpu_layers: cfg_bg.gpu_layers,
                parallel_slots: cfg_bg.parallel_slots,
                ubatch_size: cfg_bg.ubatch_size,
                runtime_binary: if cfg_bg.llama_bin.is_empty() { None } else { Some(std::path::PathBuf::from(&cfg_bg.llama_bin)) },
                draft_model_path: if cfg_bg.draft_model_path == "none" || cfg_bg.draft_model_path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&cfg_bg.draft_model_path))
                },
                speculative_draft_max: cfg_bg.speculative_draft_max,
                speculative_draft_p_min: cfg_bg.speculative_draft_p_min,
                extra_config: serde_json::json!({}),
            };

            let llama_bin_exists = !cfg_bg.llama_bin.is_empty()
                && std::path::Path::new(&cfg_bg.llama_bin).exists();
            let has_engine = if let Some(ref em) = shared_state_bg.engine_manager {
                let reg = em.registry.read().await;
                reg.get_default_engine_binary_path().is_some() || llama_bin_exists
            } else {
                llama_bin_exists
            };

            if let Err(e) = shared_state_bg.set_runtime_manager(runtime_manager.clone()) {
                error!("❌ Failed to set runtime manager: {}", e);
            }
            shared_state_bg.llm_worker.set_runtime_manager(runtime_manager.clone());
            info!("🔗 LLM worker linked to runtime manager");

            if has_engine {
                
                let last_model_loaded = 'load: {
                    let Some(data_dir) = dirs::data_dir() else { break 'load false; };
                    let last_model_path = data_dir.join("Aud.io").join("last_model.txt");
                    let Ok(last_model_id_raw) = std::fs::read_to_string(&last_model_path) else {
                        info!("ℹ️  No last used model found");
                        break 'load false;
                    };
                    let last_model_id = last_model_id_raw.trim().to_string();
                    info!("🔄 Found last used model: {}", last_model_id);

                    let Some(ref model_manager) = shared_state_bg.model_manager else {
                        info!("ℹ️  Model manager not available - skipping auto-load");
                        break 'load false;
                    };
                    let registry = model_manager.registry.read().await;
                    let Some(model_info) = registry.get_model(&last_model_id) else {
                        drop(registry);
                        warn!("⚠️  Last used model not found in registry: {}", last_model_id);
                        break 'load false;
                    };
                    if model_info.status != crate::model_management::registry::ModelStatus::Installed {
                        drop(registry);
                        info!("ℹ️  Last used model not installed");
                        break 'load false;
                    }
                    let Some(ref filename) = model_info.filename else {
                        drop(registry);
                        warn!("⚠️  Last used model has no filename");
                        break 'load false;
                    };
                    let model_path_for_runtime = model_manager.storage.model_path(&last_model_id, filename);
                    drop(registry);

                    if !model_path_for_runtime.exists() {
                        warn!("⚠️  Last used model file not found: {}", model_path_for_runtime.display());
                        break 'load false;
                    }
                    info!("✅ Auto-loading last used model from: {}", model_path_for_runtime.display());

                    let default_engine = if let Some(ref em) = shared_state_bg.engine_manager {
                        let reg = em.registry.read().await;
                        reg.get_default_engine_binary_path()
                            .or_else(|| if !cfg_bg.llama_bin.is_empty() { Some(std::path::PathBuf::from(&cfg_bg.llama_bin)) } else { None })
                    } else if !cfg_bg.llama_bin.is_empty() {
                        Some(std::path::PathBuf::from(&cfg_bg.llama_bin))
                    } else { None };

                    let mut updated_config = runtime_config.clone();
                    updated_config.model_path = model_path_for_runtime;
                    updated_config.runtime_binary = default_engine;

                    match runtime_manager.initialize_auto(updated_config).await {
                        Ok(base_url) => {
                            info!("✅ Last used model auto-loaded at {}", base_url);
                            match runtime_manager.health_check().await {
                                Ok(status) => { info!("✅ Runtime health check passed: {}", status); true }
                                Err(e) => { warn!("⚠️  Runtime health check failed: {}", e); false }
                            }
                        }
                        Err(e) => { warn!("⚠️  Failed to auto-load last used model: {}", e); false }
                    }
                };

                if !last_model_loaded && !cfg_bg.model_path.is_empty() {
                    let default_engine = if let Some(ref em) = shared_state_bg.engine_manager {
                        let reg = em.registry.read().await;
                        reg.get_default_engine_binary_path()
                            .or_else(|| if !cfg_bg.llama_bin.is_empty() { Some(std::path::PathBuf::from(&cfg_bg.llama_bin)) } else { None })
                    } else if !cfg_bg.llama_bin.is_empty() {
                        Some(std::path::PathBuf::from(&cfg_bg.llama_bin))
                    } else { None };
                    let mut updated_config = runtime_config;
                    updated_config.runtime_binary = default_engine;
                    info!("🚀 Initializing runtime with config model path...");
                    match runtime_manager.initialize_auto(updated_config).await {
                        Ok(base_url) => {
                            info!("✅ Runtime initialized at {}", base_url);
                            shared_state_bg.llm_worker.set_runtime_manager(runtime_manager);
                        }
                        Err(e) => warn!("⚠️  Runtime initialization failed: {}. Online-only mode.", e),
                    }
                }
            } else {
                info!("⏳ No engine found - starting in online-only mode");
            }

            info!("✅ Background initialization complete");
        });
    }

    {
        let cache = shared_state.attachment_cache.clone();
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(300); 
            loop {
                tokio::time::sleep(interval).await;
                let before = cache.len();
                cache.retain(|_, v: &mut crate::shared_state::PreExtracted| {
                    !v.is_stale(crate::api::attachment_api::CACHE_TTL_SECS)
                });
                let removed = before - cache.len();
                if removed > 0 {
                    info!("Attachment cache eviction: removed {} stale entries", removed);
                }
            }
        });
    }

    info!("🟢 Axum server starting on port {}...", selected_port);
    if let Err(e) = axum::serve(listener, app).await {
        error!("Axum server error: {}", e);
    }
    
    info!("Axum server stopped");
    Ok(())
}

async fn try_bind_port(host: &str, port: u16) -> anyhow::Result<tokio::net::TcpListener> {
    let addr = format!("{}:{}", host, port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => Ok(listener),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            Err(anyhow::anyhow!("Port {} is already in use", port))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to bind to {}: {}", addr, e)),
    }
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: String,  
    runtime_ready: bool,
    message: Option<String>,
}

async fn health_check(axum::extract::State(state): axum::extract::State<UnifiedAppState>) -> axum::response::Response {
    use axum::Json;
    use axum::response::IntoResponse;

    if !state.shared_state.is_initialization_complete() {
        return Json(HealthResponse {
            status: "initializing".to_string(),
            runtime_ready: false,
            message: Some("Backend initializing...".to_string()),
        })
        .into_response();
    }

    let runtime_ready = state.shared_state.llm_worker.is_runtime_ready().await;

    let (status, message) = if runtime_ready {
        ("ready", None)
    } else {
        (
            "degraded",
            Some("No model loaded. Please activate a model from the Models page.".to_string())
        )
    };

    Json(HealthResponse {
        status: status.to_string(),
        runtime_ready,
        message,
    })
    .into_response()
}

fn build_compatible_router(mut state: UnifiedAppState) -> axum::Router {
    use axum::{
        Router,
        routing::{get, post, put, delete},
        extract::DefaultBodyLimit,
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

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "aud-io-default-secret-change-in-production".to_string());

    let users_store = state.shared_state.database_pool.users.clone();

    let google_oauth = {
        let client_id = option_env!("GOOGLE_CLIENT_ID")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty()));

        let client_secret = option_env!("GOOGLE_CLIENT_SECRET")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("GOOGLE_CLIENT_SECRET").ok().filter(|s| !s.is_empty()));

        match (client_id, client_secret) {
            (Some(id), Some(secret)) => {
                tracing::info!(
                    "Google OAuth configured (client_id: {}...)",
                    &id[..id.len().min(12)]
                );
                Some(crate::api::auth_api::GoogleOAuthPending {
                    states: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
                    client_id: id,
                    client_secret: secret,
                })
            }
            _ => {
                tracing::info!(
                    "Google OAuth not configured — set GOOGLE_CLIENT_ID + GOOGLE_CLIENT_SECRET before building"
                );
                None
            }
        }
    };

    state.auth_state = Some(Arc::new(crate::api::auth_api::AuthState {
        users: users_store,
        jwt_secret,
        google: google_oauth,
    }));

    Router::new()
        
        .route("/auth/signup", post(crate::api::auth_api::signup))
        .route("/auth/login", post(crate::api::auth_api::login))
        .route("/auth/verify-email", post(crate::api::auth_api::verify_email))
        .route("/auth/me", post(crate::api::auth_api::get_current_user))
        
        .route("/auth/google/init", post(crate::api::auth_api::google_init))
        .route("/auth/google/callback", get(crate::api::auth_api::google_callback))
        .route("/auth/google/status", get(crate::api::auth_api::google_status))
        
        .route("/generate/stream", post(crate::api::stream_api::generate_stream))
        
        .route("/online/stream", post(crate::api::online_api::online_stream))
        
        .route("/generate/title", post(crate::api::title_api::generate_title))
        
        .route("/conversations", get(crate::api::conversation_api::get_conversations))
        .route("/conversations/db-stats", get(crate::api::conversation_api::get_conversations_db_stats))
        .route("/conversations/:id", get(crate::api::conversation_api::get_conversation))
        .route("/conversations/:id/title", put(crate::api::conversation_api::update_conversation_title))
        .route("/conversations/:id/pinned", post(crate::api::conversation_api::update_conversation_pinned))
        .route("/conversations/:id", delete(crate::api::conversation_api::delete_conversation))
        
        .route("/models", get(crate::api::model_api::list_models))
        .route("/models/by-mode", get(crate::api::model_api::list_models_by_mode))
        .route("/models/active", get(crate::api::model_api::get_active_model))
        .route("/models/search", get(crate::api::model_api::search_models))
        .route("/models/install", post(crate::api::model_api::install_model))
        .route("/models/remove", delete(crate::api::model_api::remove_model))
        .route("/models/progress", get(crate::api::model_api::get_download_progress))
        .route("/models/downloads", get(crate::api::model_api::get_active_downloads))
        .route("/models/downloads/cancel", post(crate::api::model_api::cancel_download))
        .route("/models/downloads/pause", post(crate::api::model_api::pause_download))
        .route("/models/downloads/resume", post(crate::api::model_api::resume_download))
        .route("/models/recommendations", get(crate::api::model_api::get_recommended_models))
        .route("/models/preferences", post(crate::api::model_api::update_preferences))
        .route("/models/refresh", post(crate::api::model_api::refresh_models))
        .route("/models/switch", post(crate::api::model_api::switch_model))
        
        .route("/models/hf/access", get(crate::api::model_api::check_hf_access))
        
        .route("/models/openrouter/catalog", get(crate::api::model_api::openrouter_catalog))
        .route("/models/openrouter/quota", get(crate::api::model_api::openrouter_quota))
        .route("/hardware/recommendations", get(crate::api::model_api::get_hardware_recommendations))
        .route("/hardware/info", get(crate::api::model_api::get_hardware_info))
        .route("/metrics/system", get(crate::api::model_api::get_system_metrics))
        .route("/storage/metadata", get(crate::api::model_api::get_storage_metadata))
        
        .route("/api-keys", post(crate::api::api_keys_api::save_api_key))
        .route("/api-keys", get(crate::api::api_keys_api::get_api_key))
        .route("/api-keys/all", get(crate::api::api_keys_api::get_all_api_keys))
        .route("/api-keys", delete(crate::api::api_keys_api::delete_api_key))
        .route("/api-keys/mark-used", post(crate::api::api_keys_api::mark_key_used))
        .route("/api-keys/verify", post(crate::api::api_keys_api::verify_api_key))
        
        .route("/mode/switch", post(crate::api::mode_api::switch_mode))
        .route("/mode/status", get(crate::api::mode_api::get_mode_status))
        
        .route("/files", get(crate::api::files_api::get_files))
        .route("/files/all", get(crate::api::files_api::get_all_files))
        .route("/files/search", get(crate::api::files_api::search_files))
        .route("/files/folder", post(crate::api::files_api::create_folder))
        .route("/files/upload", post(crate::api::files_api::upload_file))
        .route("/files/sync", post(crate::api::files_api::sync_files))
        .route("/files/resync", post(crate::api::files_api::resync_files))
        .route("/files/:id", get(crate::api::files_api::get_file_by_id))
        .route("/files/:id/content", get(crate::api::files_api::get_file_content))
        .route("/files/:id", delete(crate::api::files_api::delete_file_by_id))
        .route("/files", delete(crate::api::files_api::delete_file))
        
        .route("/all-files", get(crate::api::all_files_api::get_all_files))
        .route("/all-files/all", get(crate::api::all_files_api::get_all_files_flat))
        .route("/all-files/search", get(crate::api::all_files_api::search_all_files))
        .route("/all-files/folder", post(crate::api::all_files_api::create_all_files_folder))
        .route("/all-files/upload", post(crate::api::all_files_api::upload_all_file))
        .route("/all-files/upload-structure", post(crate::api::all_files_api::upload_all_files_structure))
        .route("/all-files/:id", get(crate::api::all_files_api::get_all_file_by_id))
        .route("/all-files/:id/content", get(crate::api::all_files_api::get_all_file_content))
        .route("/all-files/:id", delete(crate::api::all_files_api::delete_all_file_by_id))
        .route("/all-files", delete(crate::api::all_files_api::delete_all_file))
        
        .route("/feedback", post(crate::api::feedback_api::submit_feedback))
        
        .route("/notify-login", post(crate::api::login_notification_api::notify_user_login))
        
        .route("/attachments/preprocess", post(crate::api::attachment_api::preprocess_attachments))
        
        .route("/metrics", get(crate::metrics::get_metrics))
        .route("/healthz", get(health_check))
        .route("/admin/shutdown", post(crate::admin::stop_backend))
.layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(600)))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .with_state(state)
}
