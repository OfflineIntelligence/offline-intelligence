//! Mode Management API Endpoints
//!
//! Handles switching between online/offline modes:
//! - Offline: Uses local llama.cpp engine + downloaded HuggingFace models
//! - Online: Uses OpenRouter API for inference

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    memory_db::ApiKeyType,
    shared_state::UnifiedAppState,
};

/// Request to switch mode
#[derive(Debug, Deserialize)]
pub struct SwitchModeRequest {
    pub mode: String,  // "offline" or "online"
}

/// Response after switching mode
#[derive(Debug, Serialize)]
pub struct SwitchModeResponse {
    pub success: bool,
    pub mode: String,
    pub engine_status: String,  // "running", "stopped", "unavailable"
    pub message: String,
    /// True when the user has no OpenRouter API key stored and needs to add one.
    /// Frontend should show the API key modal when this is true.
    #[serde(default)]
    pub requires_api_key: bool,
}

/// Get current mode status
#[derive(Debug, Serialize)]
pub struct ModeStatusResponse {
    pub current_mode: String,  // Inferred from runtime state
    pub engine_available: bool,
    pub engine_status: String,
    pub hf_token_set: bool,
    pub openrouter_key_set: bool,
    pub installed_models_count: usize,
    pub openrouter_models_count: usize,
}

/// Switch between online/offline modes
pub async fn switch_mode(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<SwitchModeRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let mode = payload.mode.to_lowercase();

    match mode.as_str() {
        "offline" => {
            info!("Switching to OFFLINE mode - starting local engine");

            // Check if engine is available
            let engine_available = state.shared_state.engine_available.load(std::sync::atomic::Ordering::Relaxed);
            if !engine_available {
                return Ok(Json(SwitchModeResponse {
                    success: false,
                    mode: "offline".to_string(),
                    engine_status: "unavailable".to_string(),
                    message: "No llama.cpp engine installed. Please wait for auto-download or install manually.".to_string(),
                    requires_api_key: false,
                }));
            }

            // Mark HuggingFace key as used in offline mode (if exists)
            if let Ok(Some(_)) = state.shared_state.database_pool.api_keys.get_key_plaintext(&ApiKeyType::HuggingFace) {
                let _ = state.shared_state.database_pool.api_keys.mark_used(ApiKeyType::HuggingFace, "offline");
            }

            // Runtime should already be running - just verify
            let runtime_manager = {
                let guard = state.shared_state.runtime_manager.read()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                guard.clone()
            };

            if let Some(rm) = runtime_manager {
                let is_ready = rm.is_ready().await;
                Ok(Json(SwitchModeResponse {
                    success: true,
                    mode: "offline".to_string(),
                    engine_status: if is_ready { "running" } else { "starting" }.to_string(),
                    message: "Switched to offline mode. Using local engine.".to_string(),
                    requires_api_key: false,
                }))
            } else {
                Ok(Json(SwitchModeResponse {
                    success: false,
                    mode: "offline".to_string(),
                    engine_status: "unavailable".to_string(),
                    message: "Runtime manager not initialized".to_string(),
                    requires_api_key: false,
                }))
            }
        }
        "online" => {
            info!("Switching to ONLINE mode - using OpenRouter API");

            // Check if an OpenRouter key is stored; signal the frontend when missing
            let openrouter_key_exists = state.shared_state.database_pool.api_keys
                .get_key_plaintext(&ApiKeyType::OpenRouter)
                .ok()
                .flatten()
                .is_some();

            // Mark OpenRouter key as used in online mode (if exists)
            if openrouter_key_exists {
                let _ = state.shared_state.database_pool.api_keys.mark_used(ApiKeyType::OpenRouter, "online");
            }

            // In online mode, we optionally stop the runtime to save resources
            // But keep it simple for now - just acknowledge the mode switch
            Ok(Json(SwitchModeResponse {
                success: true,
                mode: "online".to_string(),
                engine_status: "not_needed".to_string(),
                message: if openrouter_key_exists {
                    "Switched to online mode. Using OpenRouter API.".to_string()
                } else {
                    "Switched to online mode. Add your OpenRouter API key to start chatting.".to_string()
                },
                requires_api_key: !openrouter_key_exists,
            }))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get current mode status
pub async fn get_mode_status(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // Check engine availability
    let engine_available = state.shared_state.engine_available.load(std::sync::atomic::Ordering::Relaxed);

    // Check if runtime is running
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

    // Infer current mode based on runtime state
    let current_mode = if engine_status == "running" {
        "offline"
    } else {
        "online"  // Default to online if engine not running
    };

    // Check if API keys are set
    let hf_token_set = state.shared_state.database_pool.api_keys
        .get_key_plaintext(&ApiKeyType::HuggingFace)
        .ok()
        .flatten()
        .is_some();

    let openrouter_key_set = state.shared_state.database_pool.api_keys
        .get_key_plaintext(&ApiKeyType::OpenRouter)
        .ok()
        .flatten()
        .is_some();

    // Count models
    let (installed_models_count, openrouter_models_count) = if let Some(ref model_manager) = state.shared_state.model_manager {
        let registry = model_manager.registry.read().await;
        let all_models = registry.list_models();
        let installed = all_models.iter().filter(|m|
            matches!(m.status, crate::model_management::registry::ModelStatus::Installed)
            && m.download_source.as_deref() != Some("openrouter")
        ).count();
        let openrouter = all_models.iter().filter(|m|
            m.download_source.as_deref() == Some("openrouter")
        ).count();
        (installed, openrouter)
    } else {
        (0, 0)
    };

    Ok(Json(ModeStatusResponse {
        current_mode: current_mode.to_string(),
        engine_available,
        engine_status: engine_status.to_string(),
        hf_token_set,
        openrouter_key_set,
        installed_models_count,
        openrouter_models_count,
    }))
}
