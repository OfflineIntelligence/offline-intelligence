//! Engine Status API Endpoints
//!
//! Reports the status of the local llama.cpp engine.
//! Online/cloud mode (OpenRouter) has been removed.
//! All inference is performed locally.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::shared_state::UnifiedAppState;

/// Current engine status response
#[derive(Debug, Serialize)]
pub struct EngineStatusResponse {
    pub engine_available: bool,
    pub engine_status: String, // "running", "stopped", "unavailable"
    pub hf_token_set: bool,
    pub installed_models_count: usize,
}

/// `GET /mode/status` — Returns local engine and model availability.
///
/// Replaces the old online/offline mode endpoint. There is now only one mode:
/// local inference. This endpoint tells the frontend whether the engine is
/// running and how many models are installed.
pub async fn get_mode_status(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine_available = state.shared_state.engine_available.load(std::sync::atomic::Ordering::Relaxed);

    let runtime_manager = {
        let guard = state.shared_state.runtime_manager.read()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.clone()
    };

    let engine_status = if let Some(rm) = runtime_manager {
        if rm.is_ready().await {
            "running"
        } else {
            "stopped"
        }
    } else {
        "unavailable"
    };

    let hf_token_set = state.shared_state.database_pool.api_keys
        .get_key_plaintext(&crate::memory_db::ApiKeyType::HuggingFace)
        .ok()
        .flatten()
        .is_some();

    let installed_models_count = if let Some(ref model_manager) = state.shared_state.model_manager {
        let registry = model_manager.registry.read().await;
        registry.list_models()
            .iter()
            .filter(|m| matches!(m.status, crate::model_management::registry::ModelStatus::Installed))
            .count()
    } else {
        0
    };

    Ok(Json(EngineStatusResponse {
        engine_available,
        engine_status: engine_status.to_string(),
        hf_token_set,
        installed_models_count,
    }))
}
