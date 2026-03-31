
use axum::extract::{State, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::config::Config;
use crate::metrics;
use crate::shared_state::SharedState;
use serde::{Deserialize, Serialize};
use tracing::info;
use std::sync::Arc;
use sysinfo::System;
use crate::cache_management::KVCacheManager;
use crate::memory_db::MemoryDatabase;

#[allow(dead_code)]
#[derive(Clone)]
pub struct AdminState {
    pub cfg: Config,
    pub shared_state: Arc<SharedState>,
}

#[derive(Deserialize)]
pub struct LoadModelRequest {
    pub model_path: String,
    pub ctx_size: Option<u32>,
    pub gpu_layers: Option<u32>,
    pub batch_size: Option<u32>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub current_model: Option<String>,
    pub current_port: Option<u16>,
    pub gpu_layers: Option<u32>,
    pub ctx_size: Option<u32>,        
    pub batch_size: Option<u32>,      
    pub is_healthy: bool,
    pub uptime_seconds: Option<u64>,
    pub memory_usage: Option<String>, 
}

pub async fn get_status(
    State(state): State<AdminState>,
) -> impl IntoResponse {
    
    let is_healthy = true; 
    
    let memory_usage = {
        let mut sys = System::new_all();
        sys.refresh_memory();
        let used = sys.used_memory();
        let total = sys.total_memory();
        Some(format!("{}/{} MB", used / 1024 / 1024, total / 1024 / 1024))
    };

    let response = StatusResponse {
        current_model: Some("direct-llm".to_string()),
        current_port: None,
        gpu_layers: Some(0),
        ctx_size: Some(state.cfg.ctx_size),
        batch_size: Some(state.cfg.batch_size),
        is_healthy,
        uptime_seconds: Some(0),
        memory_usage,
    };
    
    metrics::inc_request("admin_status", "ok");
    (StatusCode::OK, Json(response))
}

pub async fn load_model(
    State(_state): State<AdminState>,
    Json(req): Json<LoadModelRequest>,
) -> impl IntoResponse {
    info!("Received load model request for: {} with ctx_size: {:?}, gpu_layers: {:?}", 
          req.model_path, req.ctx_size, req.gpu_layers);
    
    metrics::inc_request("admin_load", "ok");
    (StatusCode::OK, format!("Model loading initiated: {}", req.model_path))
}

pub async fn stop_backend(
    State(state): State<crate::shared_state::UnifiedAppState>,
) -> impl IntoResponse {
    info!("Graceful shutdown initiated");

    let cache_mgr = state.shared_state.cache_manager.read()
        .ok()
        .and_then(|guard| guard.clone());

    if let Some(cache_manager) = cache_mgr {
        info!("Flushing KV cache to database...");
        if let Err(e) = cache_manager.flush_to_database("global_shutdown", &[]).await {
            info!("Cache flush during shutdown: {}", e);
        } else {
            info!("KV cache flushed to database");
        }
    }

    let rt_mgr = state.shared_state.runtime_manager.read()
        .ok()
        .and_then(|guard| guard.clone());

    if let Some(rt_mgr) = rt_mgr {
        info!("Shutting down runtime manager...");
        if let Err(e) = rt_mgr.shutdown().await {
            info!("Runtime shutdown: {}", e);
        } else {
            info!("Runtime manager shut down");
        }
    }

    metrics::inc_request("admin_stop", "ok");
    (StatusCode::OK, "System shutdown initiated".to_string())
}
