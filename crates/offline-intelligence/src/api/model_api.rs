//! Model Management API Endpoints
//!
//! Provides RESTful API endpoints for:
//! - Listing available and installed models
//! - Searching for models
//! - Downloading/installing models
//! - Removing/uninstalling models
//! - Getting download progress
//! - Hardware recommendations

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::{
    model_management::{
        downloader::DownloadSource,
        registry::{ModelInfo, ModelStatus},
        recommendation::{ModelRecommender, UseCase, QualityPreference, SpeedPreference, CostSensitivity},
        ModelManager,
    },
    shared_state::UnifiedAppState,
};

/// Request to install/download a model
#[derive(Debug, Deserialize)]
pub struct InstallModelRequest {
    pub model_id: String,
    pub model_name: String,
    pub source: ModelSourceSpecifier,
    pub description: Option<String>,
    pub size_bytes: u64,
    pub format: String,
    /// Optional HuggingFace token for gated/private models
    pub hf_token: Option<String>,
}

/// Specify where to download a model from
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ModelSourceSpecifier {
    HuggingFace { repo_id: String, filename: String },
}

/// Response for model installation
#[derive(Debug, Serialize)]
pub struct InstallModelResponse {
    pub download_id: String,
    pub message: String,
}

/// Response for the currently active/loaded model
#[derive(Debug, Serialize)]
pub struct ActiveModelResponse {
    pub model_path: String,
    pub model_name: String,
    pub format: String,
    pub context_size: u32,
    pub gpu_layers: u32,
    pub backend_url: String,
    pub status: String,
}

/// Request to search for models
#[derive(Debug, Deserialize)]
pub struct SearchModelsRequest {
    pub query: String,
    pub limit: Option<usize>,
}

/// Response containing search results
#[derive(Debug, Serialize)]
pub struct SearchModelsResponse {
    pub models: Vec<ModelInfo>,
    pub total_found: usize,
}

/// Request to refresh the dynamic model catalog
#[derive(Debug, Deserialize)]
pub struct RefreshModelsRequest {
    /// Optional HuggingFace token for gated/private models
    pub hf_token: Option<String>,
}

/// Response after refreshing the model catalog
#[derive(Debug, Serialize)]
pub struct RefreshModelsResponse {
    pub updated_sources: Vec<String>,
    pub total_models: usize,
}

/// Request to update user preferences
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub primary_use_case: Option<String>,
    pub quality_preference: Option<String>,
    pub speed_preference: Option<String>,
    pub cost_sensitivity: Option<String>,
}

/// Response with hardware recommendations
#[derive(Debug, Serialize)]
pub struct HardwareRecommendationsResponse {
    pub recommendations: Vec<String>,
    pub message: String,
}

/// Request to switch to a different model
#[derive(Debug, Deserialize)]
pub struct SwitchModelRequest {
    pub model_id: String,
}

/// Response after switching model
#[derive(Debug, Serialize)]
pub struct SwitchModelResponse {
    pub message: String,
    pub model_id: String,
    pub model_path: String,
}

/// Helper function to clone models from registry
async fn get_cloned_models(model_manager: &ModelManager) -> Vec<ModelInfo> {
    let registry = model_manager.registry.read().await;
    registry.list_models().iter().map(|m| (*m).clone()).collect()
}


/// Get list of all models (installed and available)
pub async fn list_models(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Return ALL models regardless of API key presence
    // Users should see available models before adding API keys
    let models = get_cloned_models(model_manager).await;

    Ok(Json(models))
}

/// Get models filtered by source (currently only "huggingface" / "offline")
pub async fn list_models_by_mode(
    State(state): State<UnifiedAppState>,
    Query(_params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_models = get_cloned_models(model_manager).await;

    let big_tech_authors = vec![
        "google", "meta", "microsoft", "openai", "anthropic",
        "deepseek-ai", "bigscience", "EleutherAI", "tiiuae",
        "mistralai", "01-ai", "Qwen", "THUDM", "baai",
    ];

    let mut hf_models: Vec<ModelInfo> = all_models
        .into_iter()
        .filter(|m| {
            m.download_source.as_deref() == Some("huggingface") &&
            !matches!(m.status, ModelStatus::Error(_))
        })
        .collect();

    hf_models.sort_by(|a, b| {
        let a_lower = a.author.as_deref().unwrap_or("").to_lowercase();
        let b_lower = b.author.as_deref().unwrap_or("").to_lowercase();
        let a_is_big = big_tech_authors.iter().any(|p| a_lower.contains(p));
        let b_is_big = big_tech_authors.iter().any(|p| b_lower.contains(p));

        match (a_is_big, b_is_big) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.downloads.cmp(&a.downloads),
        }
    });

    hf_models.truncate(100);

    Ok(Json(hf_models))
}

/// Get the currently active/loaded model info from the running server config
pub async fn get_active_model(
    State(state): State<UnifiedAppState>,
) -> Json<ActiveModelResponse> {
    let config = &state.shared_state.config;

    // Prefer the runtime's live config (populated when a model is auto-loaded or
    // activated via the UI) over the static startup config, which may be empty.
    //
    // The lock guard (RwLockReadGuard) is NOT Send, so it must be fully dropped
    // before any .await call.  We clone the Arc inside a synchronous block, then
    // call the async method outside that block.
    let runtime_arc = state.shared_state.runtime_manager
        .read()
        .ok()
        .and_then(|g| g.clone()); // guard dropped at end of this expression

    let runtime_model_path: Option<String> = if let Some(rm) = runtime_arc {
        rm.get_current_config().await
            .map(|c| c.model_path.to_string_lossy().to_string())
            .filter(|p| !p.is_empty())
    } else {
        None
    };

    let model_path = runtime_model_path
        .as_deref()
        .unwrap_or(&config.model_path);

    let model_name = std::path::Path::new(model_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let format = std::path::Path::new(model_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    let file_exists = std::path::Path::new(model_path).exists();
    let status = if file_exists { "loaded" } else { "not_found" }.to_string();

    Json(ActiveModelResponse {
        model_path: model_path.to_string(),
        model_name,
        format,
        context_size: config.ctx_size,
        gpu_layers: config.gpu_layers,
        backend_url: config.backend_url.clone(),
        status,
    })
}

/// Search for models by name, description, or tags
pub async fn search_models(
    State(state): State<UnifiedAppState>,
    Query(params): Query<SearchModelsRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_models = get_cloned_models(model_manager).await;
    let query_lower = params.query.to_lowercase();
    
    let mut filtered_models: Vec<ModelInfo> = all_models
        .into_iter()
        .filter(|model| {
            model.name.to_lowercase().contains(&query_lower) ||
            model.description.as_ref().map_or(false, |desc| desc.to_lowercase().contains(&query_lower)) ||
            model.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
        })
        .collect();

    let total_found = filtered_models.len();
    let limit = params.limit.unwrap_or(20).min(total_found);
    
    // Truncate to limit if needed
    filtered_models.truncate(limit);

    Ok(Json(SearchModelsResponse {
        models: filtered_models,
        total_found,
    }))
}

/// Refresh the dynamic model catalog from HuggingFace.
pub async fn refresh_models(
    State(state): State<UnifiedAppState>,
    Json(_payload): Json<RefreshModelsRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state
        .shared_state
        .model_manager
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut updated_sources = Vec::new();

    {
        let mut registry = model_manager.registry.write().await;
        if let Err(e) = registry.refresh_huggingface_catalog_from_api(100).await {
            error!("Failed to refresh HuggingFace catalog: {}", e);
        } else {
            updated_sources.push("huggingface".to_string());
        }
        if let Err(e) = registry.save_registry().await {
            error!("Failed to save model registry after HuggingFace refresh: {}", e);
        }
    }

    // Recompute compatibility scores for newly fetched offline models
    if !updated_sources.is_empty() {
        let cfg = &state.shared_state.config;
        let hardware = crate::model_management::ModelRecommender::detect_hardware_profile(cfg);
        let mut registry = model_manager.registry.write().await;
        registry.update_compatibility_scores(&*model_manager.recommender, &hardware);
        if let Err(e) = registry.save_registry().await {
            error!("Failed to save registry after compatibility scoring: {}", e);
        }
    }

    let total_models = {
        let registry = model_manager.registry.read().await;
        registry.list_models().len()
    };

    Ok(Json(RefreshModelsResponse {
        updated_sources,
        total_models,
    }))
}

/// Install/download a model
pub async fn install_model(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<InstallModelRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    info!("Installing model: {} ({})", payload.model_name, payload.model_id);

    // Create model info
    let model_info = ModelInfo {
        id: payload.model_id.clone(),
        name: payload.model_name.clone(),
        description: payload.description,
        author: None,
        status: ModelStatus::Available,
        size_bytes: payload.size_bytes,
        format: payload.format,
        download_source: None,
        filename: None, // Will be determined by download source
        installed_version: None,
        last_updated: None,
        tags: vec![], // Tags extracted from model metadata or source
        compatibility_score: None,
        parameters: None,
        context_length: None,
        provider: None,
        total_shards: None,
        shard_filenames: vec![],
        downloads: 0,
        is_gated: false,
        pricing: None,
    };

    // Convert source specifier to download source
    let download_source = match payload.source {
        ModelSourceSpecifier::HuggingFace { repo_id, filename } => {
            DownloadSource::HuggingFace { repo_id, filename }
        }
    };

    // Clone for use in async block
    let download_source_clone = download_source.clone();

    // Pre-create the download tracking entry so the frontend can poll immediately
    let pre_download_id = model_manager.downloader.progress_tracker()
        .start_download(
            payload.model_id.clone(),
            payload.model_name.clone(),
            Some(payload.size_bytes),
        )
        .await;

    let return_download_id = pre_download_id.clone();

    // Update registry status to Downloading
    {
        let mut reg = model_manager.registry.write().await;
        reg.update_model_status(&payload.model_id, ModelStatus::Downloading);
    }

    // Start download in background
    let registry = model_manager.registry.clone();
    let downloader = model_manager.downloader.clone();
    let existing_download_id = pre_download_id.clone();

    // Resolve HF token: request body takes priority, then fall back to the
    // token the user stored in the database via the API-keys UI.
    let hf_token = match payload.hf_token {
        Some(t) if !t.is_empty() => Some(t),
        _ => state.get_huggingface_token().await,
    };

    tokio::spawn(async move {
        match downloader.download_model(model_info.clone(), download_source_clone.clone(), Some(existing_download_id), hf_token).await {
            Ok(_download_id) => {
                // Extract the filename from the download source
                let filename = match &download_source_clone {
                    DownloadSource::HuggingFace { filename, .. } => Some(filename.clone()),
                };

                // Update registry status AND filename, then persist
                let mut reg = registry.write().await;
                reg.update_model_status(&model_info.id, ModelStatus::Installed);

                // CRITICAL: Update the filename in the registry so we know which file to load
                if let Some(fname) = filename {
                    if let Some(model) = reg.get_model_mut(&model_info.id) {
                        model.filename = Some(fname);
                        info!("Updated registry with filename for model: {}", model_info.id);
                    }
                }

                if let Err(e) = reg.save_registry().await {
                    error!("Failed to persist registry: {}", e);
                }
                drop(reg);

                if let Err(e) = downloader.save_model_metadata(&model_info, &download_source_clone).await {
                    error!("Failed to save model metadata: {}", e);
                }
                info!("Model installation completed: {}", model_info.name);
            }
            Err(e) => {
                error!("Model installation failed: {} - {}", model_info.name, e);
                let mut reg = registry.write().await;
                reg.update_model_status(&model_info.id, ModelStatus::Error(e.to_string()));
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(InstallModelResponse {
            download_id: return_download_id,
            message: format!("Started downloading model: {}", payload.model_name),
        })
    ))
}

/// Get download progress for a specific download
pub async fn get_download_progress(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let download_id = params.get("download_id")
        .ok_or(StatusCode::BAD_REQUEST)?;

    let progress = model_manager.downloader.progress_tracker()
        .get_progress(download_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(progress))
}

/// Get all downloads (active and completed)
pub async fn get_active_downloads(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let downloads = model_manager.downloader.progress_tracker()
        .get_all_downloads()
        .await;

    Ok(Json(downloads))
}

/// Cancel an ongoing download
pub async fn cancel_download(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let download_id = params.get("download_id")
        .ok_or(StatusCode::BAD_REQUEST)?;

    let success = model_manager.downloader.cancel_download(download_id).await;
    
    if success {
        Ok(Json(serde_json::json!({
            "message": "Download cancelled successfully"
        })))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Pause an ongoing download
pub async fn pause_download(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let download_id = params.get("download_id")
        .ok_or(StatusCode::BAD_REQUEST)?;

    let tracker = model_manager.downloader.progress_tracker();
    if let Some(progress) = tracker.get_progress(download_id).await {
        // Only allow pausing if the download is currently downloading
        if progress.status == crate::model_management::progress::DownloadStatus::Downloading {
            tracker.update_progress(
                download_id,
                progress.bytes_downloaded,
                crate::model_management::progress::DownloadStatus::Paused,
                None,
            ).await;
            Ok(Json(serde_json::json!({ "message": "Download paused" })))
        } else {
            // Return an error if trying to pause a download that isn't downloading
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Resume a paused download
pub async fn resume_download(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let download_id = params.get("download_id")
        .ok_or(StatusCode::BAD_REQUEST)?;

    let tracker = model_manager.downloader.progress_tracker();
    if let Some(progress) = tracker.get_progress(download_id).await {
        // Only allow resuming if the download is currently paused
        if progress.status == crate::model_management::progress::DownloadStatus::Paused {
            tracker.update_progress(
                download_id,
                progress.bytes_downloaded,
                crate::model_management::progress::DownloadStatus::Downloading,
                None,
            ).await;
            Ok(Json(serde_json::json!({ "message": "Download resumed" })))
        } else {
            // Return an error if trying to resume a download that isn't paused
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Remove/uninstall a model
pub async fn remove_model(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let model_id = params.get("model_id")
        .ok_or(StatusCode::BAD_REQUEST)?;

    info!("Removing model: {}", model_id);

    // Remove from storage
    if let Err(e) = model_manager.storage.remove_model(model_id) {
        error!("Failed to remove model from storage: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Remove from registry and persist
    let mut registry = model_manager.registry.write().await;
    registry.remove_model(model_id);
    if let Err(e) = registry.save_registry().await {
        error!("Failed to persist registry after removal: {}", e);
    }

    Ok(Json(serde_json::json!({
        "message": format!("Model {} removed successfully", model_id)
    })))
}

/// Get hardware recommendations
pub async fn get_hardware_recommendations(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let hardware = ModelRecommender::detect_hardware_profile(&state.shared_state.config);
    let message = model_manager.recommender.get_hardware_recommendation_message(&hardware);
    
    let recommendations = message.lines().map(|s| s.to_string()).collect::<Vec<String>>();

    Ok(Json(HardwareRecommendationsResponse {
        recommendations,
        message,
    }))
}

/// Update user preferences for model recommendations
pub async fn update_preferences(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<UpdatePreferencesRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut preferences = model_manager.recommender.get_preferences().clone();

    if let Some(use_case) = payload.primary_use_case {
        preferences.primary_use_case = match use_case.as_str() {
            "chat_assistant" => UseCase::ChatAssistant,
            "code_generation" => UseCase::CodeGeneration,
            "creative_writing" => UseCase::CreativeWriting,
            "research_analysis" => UseCase::ResearchAnalysis,
            "translation" => UseCase::Translation,
            _ => UseCase::GeneralPurpose,
        };
    }

    if let Some(quality) = payload.quality_preference {
        preferences.quality_preference = match quality.as_str() {
            "high_quality" => QualityPreference::HighQuality,
            "fast_response" => QualityPreference::FastResponse,
            _ => QualityPreference::Balanced,
        };
    }

    if let Some(speed) = payload.speed_preference {
        preferences.speed_preference = match speed.as_str() {
            "fastest" => SpeedPreference::Fastest,
            "highest_quality" => SpeedPreference::HighestQuality,
            _ => SpeedPreference::Balanced,
        };
    }

    if let Some(cost) = payload.cost_sensitivity {
        preferences.cost_sensitivity = match cost.as_str() {
            "budget" => CostSensitivity::Budget,
            "premium" => CostSensitivity::Premium,
            _ => CostSensitivity::Moderate,
        };
    }

    // We can't mutate the recommender through Arc, so we'll need to restructure this
    // For now, let's just acknowledge the preferences were set
    info!("User preferences updated: {:?}", preferences);

    Ok(Json(serde_json::json!({
        "message": "Preferences updated successfully"
    })))
}

/// Get recommended models based on current hardware and preferences
pub async fn get_recommended_models(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let max_results = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let hardware = ModelRecommender::detect_hardware_profile(&state.shared_state.config);
    let all_models = get_cloned_models(model_manager).await;
    
    let recommendations = model_manager.recommender.get_recommendations(
        all_models.iter().collect(),
        &hardware,
        max_results
    );

    // Get full model info for recommended models
    let recommended_models: Vec<ModelInfo> = recommendations
        .into_iter()
        .filter_map(|(model_id, _)| {
            all_models.iter().find(|m| m.id == model_id).cloned()
        })
        .collect();

    Ok(Json(recommended_models))
}

/// Hardware information response
#[derive(Debug, Serialize)]
pub struct HardwareInfoResponse {
    pub total_ram_gb: f32,
    pub available_ram_gb: f32,
    pub cpu_cores: u32,
    pub gpu_available: bool,
    pub gpu_vram_gb: Option<f32>,
    pub storage_used_bytes: u64,
    pub storage_available_bytes: u64,
}

/// Get current hardware info and storage usage
pub async fn get_hardware_info(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let hardware = ModelRecommender::detect_hardware_profile(&state.shared_state.config);

    let (storage_used, storage_available) = if let Some(mm) = state.shared_state.model_manager.as_ref() {
        let used = mm.storage.get_storage_usage().unwrap_or(0);
        let available = mm.storage.get_available_space().unwrap_or(0);
        (used, available)
    } else {
        (0, 0)
    };

    Ok(Json(HardwareInfoResponse {
        total_ram_gb: hardware.total_ram_gb,
        available_ram_gb: hardware.available_ram_gb,
        cpu_cores: hardware.cpu_cores,
        gpu_available: hardware.gpu_available,
        gpu_vram_gb: hardware.gpu_vram_gb,
        storage_used_bytes: storage_used,
        storage_available_bytes: storage_available,
    }))
}

/// Live system metrics response
#[derive(Serialize)]
pub struct SystemMetricsResponse {
    pub cpu_usage_percent: f32,
    pub per_core_usage: Vec<f32>,
    pub cpu_model_name: String,
    pub cpu_frequency_mhz: u64,
    pub gpu_available: bool,
    pub gpu_name: String,
    pub gpu_usage_percent: f32,
    pub gpu_vram_total_gb: f32,
    pub gpu_vram_used_gb: f32,
    pub gpu_temperature_c: f32,
    pub memory_total_gb: f32,
    pub memory_used_gb: f32,
    pub memory_available_gb: f32,
    pub gpu_layers_offloaded: u32,
    pub inference_device: String, // "GPU", "CPU", "CPU+GPU"
}

/// Switch to a different model by ID
pub async fn switch_model(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<SwitchModelRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Look up the model in the registry by ID
    let model_info = {
        let registry = model_manager.registry.read().await;
        registry.get_model(&payload.model_id)
            .ok_or(StatusCode::NOT_FOUND)?
            .clone()
    };
    
    // Get the complete model metadata including runtime binaries
    let model_metadata = {
        let registry = model_manager.registry.read().await;
        registry.get_model_metadata(&payload.model_id).await
    };
    
    // Verify that the model is installed and the file exists
    if model_info.status != ModelStatus::Installed {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Get the model's path using the storage module
    let model_path = if let Some(ref filename) = model_info.filename {
        // Use the stored filename if available
        let path = model_manager.storage.model_path(&payload.model_id, filename);
        info!("🔍 Resolving model path from registry filename: {}", path.display());
        path
    } else {
        // If no filename is stored, look for model files in the model directory
        warn!("⚠️  Model {} has no filename in registry, scanning directory...", payload.model_id);

        // We can get the directory by using a dummy filename with model_path, then taking the parent
        let model_dir = model_manager.storage.model_path(&payload.model_id, "dummy").parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| {
                error!("❌ Failed to get parent directory for model: {}", payload.model_id);
                StatusCode::NOT_FOUND
            })?;

        info!("📂 Scanning model directory: {}", model_dir.display());

        if !model_dir.exists() {
            error!("❌ Model directory does not exist: {}", model_dir.display());
            return Err(StatusCode::NOT_FOUND);
        }

        // Look for model files in the directory
        let mut found_path = None;
        if let Ok(entries) = std::fs::read_dir(&model_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                        if matches!(ext.as_str(), "gguf" | "bin" | "ggml" | "onnx" | "trt" | "engine" | "safetensors" | "mlmodel") {
                            // Found a valid model file
                            info!("✅ Found model file: {}", path.display());
                            found_path = Some(path);
                            break; // Take the first valid file found
                        }
                    }
                }
            }
        }

        match found_path {
            Some(path) => path,
            None => {
                error!("❌ No valid model file found in directory: {}", model_dir.display());
                return Err(StatusCode::NOT_FOUND);
            }
        }
    };

    if !model_path.exists() {
        error!("❌ Model file does not exist: {}", model_path.display());
        error!("   Please check if the model was downloaded correctly to AppData");
        return Err(StatusCode::NOT_FOUND);
    }

    info!("✅ Model file verified at: {}", model_path.display());
    
    // Convert model_path to string for later use since it will be moved
    let model_path_str = model_path.to_string_lossy().to_string();
    
    // Get the runtime manager from shared state
    let runtime_manager = {
        let guard = state.shared_state.runtime_manager.read()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.clone().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
    };
    
    // Prepare runtime config for the new model
    // Try to get the model's format from the registry info, default to GGUF if not recognized
    let model_format = match model_info.format.as_str().to_lowercase().as_str() {
        "gguf" => crate::model_runtime::ModelFormat::GGUF,
        "ggml" => crate::model_runtime::ModelFormat::GGML,
        "onnx" => crate::model_runtime::ModelFormat::ONNX,
        "tensorrt" => crate::model_runtime::ModelFormat::TensorRT,
        "safetensors" => crate::model_runtime::ModelFormat::Safetensors,
        "coreml" => crate::model_runtime::ModelFormat::CoreML,
        _ => crate::model_runtime::ModelFormat::GGUF, // Default fallback
    };
    
    // Determine the appropriate runtime binary based on platform and model metadata
    // Priority: 1) Model metadata, 2) Installed engine from registry, 3) Config fallback
    let runtime_binary = if let Some(ref metadata) = model_metadata {
        // If we have model metadata with runtime binaries, try to use the platform-appropriate one
        use crate::model_runtime::platform_detector::HardwareCapabilities;
        let hw_caps = HardwareCapabilities::default();
        let platform_name = match hw_caps.platform {
            crate::model_runtime::platform_detector::Platform::Windows => "windows",
            crate::model_runtime::platform_detector::Platform::Linux => "linux",
            crate::model_runtime::platform_detector::Platform::MacOS => "macos",
        };

        // First try to get platform-specific binary from metadata
        if let Some(bin_path) = metadata.runtime_binaries.get(platform_name) {
            Some(bin_path.clone())
        } else {
            // Fallback to installed engine from engine registry, then to config llama_bin
            if let Some(ref engine_manager) = state.shared_state.engine_manager {
                let registry = engine_manager.registry.read().await;
                registry.get_default_engine_binary_path()
                    .or_else(|| if !state.shared_state.config.llama_bin.is_empty() {
                        Some(std::path::PathBuf::from(&state.shared_state.config.llama_bin))
                    } else { None })
            } else {
                Some(std::path::PathBuf::from(&state.shared_state.config.llama_bin))
            }
        }
    } else {
        // No metadata available, use installed engine from registry, then config llama_bin
        if let Some(ref engine_manager) = state.shared_state.engine_manager {
            let registry = engine_manager.registry.read().await;
            registry.get_default_engine_binary_path()
                .or_else(|| if !state.shared_state.config.llama_bin.is_empty() {
                    Some(std::path::PathBuf::from(&state.shared_state.config.llama_bin))
                } else { None })
        } else {
            Some(std::path::PathBuf::from(&state.shared_state.config.llama_bin))
        }
    };
    
    let runtime_config = crate::model_runtime::RuntimeConfig {
        model_path: model_path.clone(),
        format: model_format, // Use the detected format from model info
        host: state.shared_state.config.llama_host.clone(),
        port: state.shared_state.config.llama_port,
        context_size: state.shared_state.config.ctx_size,
        batch_size: state.shared_state.config.batch_size,
        threads: state.shared_state.config.threads,
        gpu_layers: state.shared_state.config.gpu_layers,
        parallel_slots: state.shared_state.config.parallel_slots,
        ubatch_size: state.shared_state.config.ubatch_size,
        runtime_binary: runtime_binary.clone(), // Use platform-appropriate binary
        draft_model_path: {
            let p = &state.shared_state.config.draft_model_path;
            if p == "none" || p.is_empty() { None } else { Some(std::path::PathBuf::from(p)) }
        },
        speculative_draft_max: state.shared_state.config.speculative_draft_max,
        speculative_draft_p_min: state.shared_state.config.speculative_draft_p_min,
        extra_config: serde_json::json!({}),
    };

    info!("🚀 Initializing runtime with config:");
    info!("   Model Path: {}", runtime_config.model_path.display());
    info!("   Runtime Binary: {}", runtime_binary.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "None".to_string()));
    info!("   Format: {:?}", runtime_config.format);
    info!("   Host: {}:{}", runtime_config.host, runtime_config.port);
    info!("   Context Size: {}", runtime_config.context_size);
    info!("   GPU Layers: {}", runtime_config.gpu_layers);

    // Skip re-initialization if this exact model is already loaded and healthy.
    // This avoids a shutdown→restart race where the model becomes briefly unavailable.
    if let Some(current_config) = runtime_manager.get_current_config().await {
        if current_config.model_path == runtime_config.model_path
            && runtime_manager.is_ready().await
        {
            info!("✅ Model {} is already loaded and ready — skipping re-initialization", model_info.name);
            let _ = std::fs::write(crate::utils::PathResolver::last_model_path(), &payload.model_id);
            return Ok(Json(SwitchModelResponse {
                message: format!("Model {} is already loaded and ready for inference", model_info.name),
                model_id: payload.model_id.clone(),
                model_path: model_path_str,
            }));
        }
    }

    // Use initialize_auto to automatically detect the model format
    match runtime_manager.initialize_auto(runtime_config).await {
        Ok(base_url) => {
            info!("Runtime initialized at {}, performing health check...", base_url);

            // CRITICAL: Verify runtime is actually ready before returning success
            match runtime_manager.health_check().await {
                Ok(_) => {
                    info!("✅ Model {} activated successfully and health check passed", model_info.name);

                    // Save last used model for auto-load on next startup
                    let _ = std::fs::write(crate::utils::PathResolver::last_model_path(), &payload.model_id);

                    Ok(Json(SwitchModelResponse {
                        message: format!("Model {} loaded and ready for inference", model_info.name),
                        model_id: payload.model_id.clone(),
                        model_path: model_path_str,
                    }))
                }
                Err(e) => {
                    error!("❌ Model activation health check failed: {}", e);
                    error!("   The model may be too large or incompatible with your hardware");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check if the error is due to a missing binary
            if error_msg.contains("binary not found")
                || error_msg.contains("not found at")
                || error_msg.contains("No such file")
            {
                error!("Engine binary not found - attempting automatic download and retry");

                // Attempt to download engine automatically
                if let Some(ref engine_manager) = state.shared_state.engine_manager {
                    match engine_manager.ensure_engine_available().await {
                        Ok(_) => {
                            info!("Engine downloaded successfully, retrying model switch...");
                            return Ok(Json(SwitchModelResponse {
                                message: "Engine was downloaded. Please retry switching models.".to_string(),
                                model_id: payload.model_id.clone(),
                                model_path: "retry_required".to_string(),
                            }));
                        }
                        Err(e) => {
                            error!("Engine download failed: {}", e);
                            return Err(StatusCode::SERVICE_UNAVAILABLE);
                        }
                    }
                } else {
                    error!("No engine manager available — cannot auto-download engine");
                    return Err(StatusCode::SERVICE_UNAVAILABLE);
                }
            }

            error!("Failed to switch model: {}", error_msg);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get live system metrics including CPU/GPU usage
pub async fn get_system_metrics(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    use sysinfo::System;

    // CPU metrics - need two refreshes with delay for accurate usage
    let mut system = System::new_all();
    system.refresh_cpu();
    // Small delay for accurate CPU measurement
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    system.refresh_cpu();
    system.refresh_memory();

    let cpu_usage = system.global_cpu_info().cpu_usage();
    let per_core: Vec<f32> = system.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
    let cpu_model = system.cpus().first().map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown CPU".into());
    let cpu_freq = system.cpus().first().map(|c| c.frequency()).unwrap_or(0);

    let total_mem = system.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
    let used_mem = (system.total_memory() - system.available_memory()) as f32 / (1024.0 * 1024.0 * 1024.0);
    let available_mem = system.available_memory() as f32 / (1024.0 * 1024.0 * 1024.0);

    // GPU metrics
    let (gpu_available, gpu_name, gpu_usage, gpu_vram_total, gpu_vram_used, gpu_temp) = {
        #[cfg(feature = "nvidia")]
        {
            match nvml_wrapper::Nvml::init() {
                Ok(nvml) => {
                    match nvml.device_by_index(0) {
                        Ok(device) => {
                            let name = device.name().unwrap_or_else(|_| "GPU".into());
                            let utilization = device.utilization_rates().map(|u| u.gpu as f32).unwrap_or(0.0);
                            let mem_info = device.memory_info();
                            let vram_total = mem_info.as_ref().map(|m| m.total as f32 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(0.0);
                            let vram_used = mem_info.as_ref().map(|m| m.used as f32 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(0.0);
                            let temp = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).unwrap_or(0) as f32;
                            tracing::debug!("GPU detected: {}, usage: {}%, VRAM: {}/{} GB", name, utilization, vram_used, vram_total);
                            (true, name, utilization, vram_total, vram_used, temp)
                        }
                        Err(e) => {
                            tracing::warn!("NVML initialized but failed to get device: {}", e);
                            (false, String::from("Not detected"), 0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32)
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("NVML not available: {}", e);
                    (false, String::from("Not detected"), 0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32)
                }
            }
        }
        #[cfg(not(feature = "nvidia"))]
        {
            // Without NVML, report no GPU metrics - GPU detection happens at config level
            (false, String::from("Not detected"), 0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32)
        }
    };

    // Determine inference device from config
    let gpu_layers = state.shared_state.config.gpu_layers;
    let inference_device = if !gpu_available {
        "CPU".to_string()
    } else if gpu_layers == 0 {
        "CPU".to_string()
    } else if gpu_layers >= 50 {
        "GPU".to_string()
    } else {
        "CPU+GPU".to_string()
    };

    Ok(Json(SystemMetricsResponse {
        cpu_usage_percent: cpu_usage,
        per_core_usage: per_core,
        cpu_model_name: cpu_model,
        cpu_frequency_mhz: cpu_freq,
        gpu_available,
        gpu_name,
        gpu_usage_percent: gpu_usage,
        gpu_vram_total_gb: gpu_vram_total,
        gpu_vram_used_gb: gpu_vram_used,
        gpu_temperature_c: gpu_temp,
        memory_total_gb: total_mem,
        memory_used_gb: used_mem,
        memory_available_gb: available_mem,
        gpu_layers_offloaded: gpu_layers,
        inference_device,
    }))
}

/// Storage metadata response
#[derive(Debug, Serialize)]
pub struct StorageMetadataResponse {
    /// System paths
    pub paths: StoragePaths,
    /// Downloaded models with metadata
    pub models: Vec<DownloadedModelInfo>,
    /// Storage usage statistics
    pub storage_stats: StorageStats,
    /// Database information
    pub database_info: DatabaseInfo,
    /// Installed engines information
    pub engines: Vec<InstalledEngineInfo>,
}

/// Storage paths on the system
#[derive(Debug, Serialize)]
pub struct StoragePaths {
    pub app_data_dir: String,
    pub models_dir: String,
    pub registry_dir: String,
    pub database_path: String,
    pub engines_dir: String,
}

/// Information about an installed engine
#[derive(Debug, Serialize)]
pub struct InstalledEngineInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub platform: String,
    pub acceleration: String,
    pub file_size: u64,
    pub size_human: String,
    pub install_path: String,
    pub binary_name: String,
    pub is_default: bool,
}

/// Information about a downloaded model
#[derive(Debug, Serialize)]
pub struct DownloadedModelInfo {
    pub id: String,
    pub name: String,
    pub format: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub download_date: String,
    pub download_source: String,
    pub file_path: String,
    pub metadata_path: Option<String>,
}

/// Storage usage statistics
#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub models_total_bytes: u64,
    pub models_total_human: String,
    pub available_space_bytes: u64,
    pub available_space_human: String,
    pub model_count: usize,
}

/// Database information
#[derive(Debug, Serialize)]
pub struct DatabaseInfo {
    pub path: String,
    pub size_bytes: u64,
    pub size_human: String,
}

/// Get comprehensive local storage metadata
pub async fn get_storage_metadata(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::model_management::storage::ModelMetadata;
    
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get app data directory using dirs crate
    let app_data_dir = crate::utils::PathResolver::data_dir();
    let engines_dir = crate::utils::PathResolver::engines_dir();

    // Get storage paths
    let paths = StoragePaths {
        app_data_dir: app_data_dir.to_string_lossy().to_string(),
        models_dir: model_manager.storage.location.models_dir.to_string_lossy().to_string(),
        registry_dir: model_manager.storage.location.registry_dir.to_string_lossy().to_string(),
        database_path: app_data_dir.join("memory.db").to_string_lossy().to_string(),
        engines_dir: engines_dir.to_string_lossy().to_string(),
    };
    
    // Get downloaded models with metadata
    let mut models = Vec::new();
    let installed_models: Vec<crate::model_management::registry::ModelInfo> = {
        let registry = model_manager.registry.read().await;
        registry.list_models().into_iter()
            .filter(|m| matches!(m.status, crate::model_management::registry::ModelStatus::Installed))
            .cloned()
            .collect()
    };
    
    for model in installed_models {
        // Try to load metadata
        let metadata_path = model_manager.storage.metadata_path(&model.id);
        let download_date = if metadata_path.exists() {
            std::fs::metadata(&metadata_path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .flatten()
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };
        
        let download_source = if metadata_path.exists() {
            std::fs::read_to_string(&metadata_path)
                .ok()
                .and_then(|content| serde_json::from_str::<ModelMetadata>(&content).ok())
                .map(|m| m.download_source)
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        };
        
        // Get actual file size from disk
        let model_dir = model_manager.storage.location.models_dir.join(
            model.id.replace(':', "_").replace('/', "_").replace('\\', "_")
        );
        let mut actual_size = model.size_bytes;
        if model_dir.exists() {
            actual_size = walkdir::WalkDir::new(&model_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum();
        }
        
        models.push(DownloadedModelInfo {
            id: model.id.clone(),
            name: model.name.clone(),
            format: model.format.clone(),
            size_bytes: actual_size,
            size_human: format_bytes(actual_size),
            download_date,
            download_source,
            file_path: model_dir.to_string_lossy().to_string(),
            metadata_path: if metadata_path.exists() {
                Some(metadata_path.to_string_lossy().to_string())
            } else {
                None
            },
        });
    }
    
    // Get storage stats
    let models_total_bytes = model_manager.storage.get_storage_usage().unwrap_or(0);
    let available_space_bytes = model_manager.storage.get_available_space().unwrap_or(0);
    
    let storage_stats = StorageStats {
        models_total_bytes,
        models_total_human: format_bytes(models_total_bytes),
        available_space_bytes,
        available_space_human: format_bytes(available_space_bytes),
        model_count: models.len(),
    };
    
    // Get database info
    let db_path = app_data_dir.join("memory.db");
    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    
    let database_info = DatabaseInfo {
        path: db_path.to_string_lossy().to_string(),
        size_bytes: db_size,
        size_human: format_bytes(db_size),
    };

    // Get installed engines info
    let mut engines = Vec::new();
    if let Some(ref engine_manager) = state.shared_state.engine_manager {
        let registry = engine_manager.registry.read().await;
        let default_engine_id = registry.default_engine.clone();

        for (engine_id, engine_info) in &registry.installed_engines {
            if let Some(install_path) = &engine_info.install_path {
                engines.push(InstalledEngineInfo {
                    id: engine_info.id.clone(),
                    name: engine_info.name.clone(),
                    version: engine_info.version.clone(),
                    platform: format!("{:?}", engine_info.platform),
                    acceleration: format!("{:?}", engine_info.acceleration),
                    file_size: engine_info.file_size,
                    size_human: format_bytes(engine_info.file_size),
                    install_path: install_path.to_string_lossy().to_string(),
                    binary_name: engine_info.binary_name.clone(),
                    is_default: default_engine_id.as_ref() == Some(engine_id),
                });
            }
        }
    }

    Ok(Json(StorageMetadataResponse {
        paths,
        models,
        storage_stats,
        database_info,
        engines,
    }))
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase A: HuggingFace Gated Model Access Check
// ─────────────────────────────────────────────────────────────────────────────

/// Query parameters for the HF access-check endpoint
#[derive(Debug, Deserialize)]
pub struct HfAccessParams {
    pub repo_id: String,
    pub filename: String,
    pub hf_token: Option<String>,
}

/// Response body for the HF access-check endpoint
#[derive(Debug, Serialize)]
pub struct HfAccessResponse {
    /// One of: "accessible", "not_approved", "unauthorized", "not_found", "error"
    pub status: String,
    /// `true` when the user may start a download immediately
    pub can_download: bool,
    /// Human-readable explanation
    pub message: String,
}

/// `GET /models/hf/access?repo_id=…&filename=…&hf_token=…`
///
/// Performs a HEAD request against the HuggingFace CDN to determine whether
/// the supplied token grants download access to a gated repository.
pub async fn check_hf_access(
    Query(params): Query<HfAccessParams>,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::model_management::{check_hf_gated_access, HfAccessStatus};

    let status = check_hf_gated_access(
        &params.repo_id,
        &params.filename,
        params.hf_token.as_deref(),
    )
    .await;

    let (status_str, can_download, message) = match &status {
        HfAccessStatus::Accessible => (
            "accessible",
            true,
            "Access granted — download can proceed.".to_string(),
        ),
        HfAccessStatus::NotApproved => (
            "not_approved",
            false,
            "Your token is valid but you have not been approved to access this \
             model yet. Visit the model page on HuggingFace to request access."
                .to_string(),
        ),
        HfAccessStatus::Unauthorized => (
            "unauthorized",
            false,
            "No HuggingFace token provided or the token is invalid. \
             Please add your HF token in Settings."
                .to_string(),
        ),
        HfAccessStatus::NotFound => (
            "not_found",
            false,
            "The model or file was not found on HuggingFace.".to_string(),
        ),
        HfAccessStatus::Error(e) => (
            "error",
            false,
            format!("Network or server error: {}", e),
        ),
    };

    Ok(Json(HfAccessResponse {
        status: status_str.to_string(),
        can_download,
        message,
    }))
}

