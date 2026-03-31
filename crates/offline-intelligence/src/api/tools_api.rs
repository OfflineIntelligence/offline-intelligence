
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tracing::{info, warn};

use crate::shared_state::UnifiedAppState;
use crate::memory_db::ApiKeyType;

#[derive(Serialize)]
pub struct ToolsSettingsResponse {
    pub enabled: bool,
    pub has_brave_key: bool,
}

#[derive(Deserialize)]
pub struct UpdateToolsSettingsRequest {
    pub enabled: Option<bool>,
    pub brave_key: Option<String>,
}

pub async fn get_tools_settings(
    State(state): State<UnifiedAppState>,
) -> Json<ToolsSettingsResponse> {
    let enabled = state.shared_state.tools_enabled.load(Ordering::Relaxed);
    let has_brave_key = state
        .shared_state
        .database_pool
        .api_keys
        .key_exists(&ApiKeyType::BraveSearch)
        .unwrap_or(false);

    Json(ToolsSettingsResponse { enabled, has_brave_key })
}

pub async fn update_tools_settings(
    State(state): State<UnifiedAppState>,
    Json(req): Json<UpdateToolsSettingsRequest>,
) -> Result<Json<ToolsSettingsResponse>, (StatusCode, String)> {
    
    if let Some(enabled) = req.enabled {
        state.shared_state.tools_enabled.store(enabled, Ordering::Relaxed);
        info!("Web tools {}", if enabled { "enabled" } else { "disabled" });

        let val = if enabled { "1" } else { "0" };
        if let Err(e) = state
            .shared_state
            .database_pool
            .api_keys
            .save_key(ApiKeyType::WebToolsEnabled, val)
        {
            warn!("Failed to persist tools_enabled setting: {}", e);
        }
    }

    if let Some(ref key) = req.brave_key {
        if key.is_empty() {
            let _ = state
                .shared_state
                .database_pool
                .api_keys
                .delete_key(ApiKeyType::BraveSearch);
            info!("Brave Search key removed");
        } else {
            state
                .shared_state
                .database_pool
                .api_keys
                .save_key(ApiKeyType::BraveSearch, key)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            info!("Brave Search key saved");
        }
    }

    let enabled = state.shared_state.tools_enabled.load(Ordering::Relaxed);
    let has_brave_key = state
        .shared_state
        .database_pool
        .api_keys
        .key_exists(&ApiKeyType::BraveSearch)
        .unwrap_or(false);

    Ok(Json(ToolsSettingsResponse { enabled, has_brave_key }))
}
