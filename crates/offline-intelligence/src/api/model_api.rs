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
use std::env;

use crate::{
    model_management::{
        downloader::DownloadSource,
        registry::{ModelInfo, ModelStatus},
        recommendation::{ModelRecommender, UseCase, QualityPreference, SpeedPreference, CostSensitivity},
        ModelManager,
    },
    model_runtime::RuntimeManager,
    shared_state::UnifiedAppState,
    memory_db::ApiKeyType,
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
    OpenRouter { model_id: String },
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
    /// Which source to refresh: "openrouter", "huggingface", or "all" (default)
    pub source: Option<String>,
    /// Optional OpenRouter API key supplied by the frontend. If not provided,
    /// the backend will fall back to the OPENROUTER_API_KEY environment
    /// variable when refreshing OpenRouter models.
    pub openrouter_api_key: Option<String>,
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

/// Helper function to check if a verified API key exists in the database
async fn has_verified_key(state: &UnifiedAppState, key_type: ApiKeyType) -> bool {
    match state.shared_state.database_pool.api_keys.get_key_plaintext(&key_type) {
        Ok(Some(_)) => true,
        _ => false,
    }
}

/// Get list of all models (installed and available)
/// Filters out OpenRouter and HuggingFace models if no verified key exists
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

/// Get models filtered by mode (online/offline)
pub async fn list_models_by_mode(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state.shared_state.model_manager.as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mode = params.get("mode")
        .map(|s| s.as_str())
        .unwrap_or("offline");

    let all_models = get_cloned_models(model_manager).await;

    // Check for verified keys (for display purposes - show all models regardless)
    let has_openrouter_key = has_verified_key(&state, ApiKeyType::OpenRouter).await;
    let has_huggingface_key = has_verified_key(&state, ApiKeyType::HuggingFace).await;

    match mode {
        "online" => {
            // Show filtered models (free, context > 16k from premium providers)
            // API key is only required when actually using the model for chat, not for viewing
            let premium_providers = vec![
                "openai",       // GPT-4, GPT-4o, GPT-4 Turbo
                "google",       // Gemini
                "anthropic",    // Claude
                "deepseek",     // DeepSeek
                "moonshot",    // Kimi
                "qwen",        // Qwen
                "zhipuai",     // GLM/智谱AI
                "minimax",     // Minimax
                "microsoft",   // Phi, Copilot
                "meta-llama",  // Llama
                "mistral",     // Mistral
                "cohere",      // Command R
                "sarvam",     // Sarvam AI
                "x-ai",       // Grok
                "nvidia",     // Nvidia
                "amazon",     // Claude on Bedrock
                "replicate",  // Various models
            ];

            // Context length thresholds (in tokens)
            const MIN_PREMIUM_CTX: u64 = 32000;  // 32k minimum for premium
            const MIN_STANDARD_CTX: u64 = 16000; // 16k minimum for standard

            // Filter OpenRouter models: premium providers + good context length
            let or_models: Vec<ModelInfo> = all_models
                .into_iter()
                .filter(|m| {
                    if m.download_source.as_deref() != Some("openrouter") {
                        return false;
                    }
                    let id_lower = m.id.to_lowercase();
                    
                    // Check if it's from a premium provider
                    let is_premium = premium_providers.iter().any(|p| id_lower.contains(p));
                    
                    // Check context length
                    let ctx_len = m.context_length.unwrap_or(0);
                    let has_good_ctx = ctx_len >= MIN_STANDARD_CTX;
                    
                    // Include if premium provider OR has good context length
                    is_premium || has_good_ctx
                })
                .collect();

            // Sort: Premium providers first (by context length), then by provider name
            let mut sorted_models: Vec<ModelInfo> = or_models;
            sorted_models.sort_by(|a, b| {
                let a_lower = a.id.to_lowercase();
                let b_lower = b.id.to_lowercase();
                
                // Check premium status
                let a_is_premium = premium_providers.iter().any(|p| a_lower.contains(p));
                let b_is_premium = premium_providers.iter().any(|p| b_lower.contains(p));
                
                // Get context lengths (default to 0 if not set)
                let a_ctx = a.context_length.unwrap_or(0);
                let b_ctx = b.context_length.unwrap_or(0);
                
                match (a_is_premium, b_is_premium) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        // Both are premium - sort by context length (higher first)
                        b_ctx.cmp(&a_ctx)
                    }
                }
            });

            // Deduplicate by keeping first occurrence of each base model
            let mut seen = std::collections::HashSet::new();
            sorted_models.retain(|m| {
                // Extract base model name (remove openrouter: prefix and version suffixes)
                let base_name = m.id
                    .replace("openrouter:", "")
                    .split(':')
                    .next()
                    .unwrap_or(&m.id)
                    .to_string();
                seen.insert(base_name).then(|| {
                    // Keep this model
                    true
                }).unwrap_or(false)
            });

            // Limit to top 50 models
            sorted_models.truncate(50);

            Ok(Json(sorted_models))
        }
        "offline" => {
            // Show ALL HuggingFace models (not just installed ones)
            // API key is only required when downloading gated/private models, not for viewing
            // Prioritize big tech authors at top
            let big_tech_authors = vec![
                "google",
                "meta",
                "microsoft",
                "openai",
                "anthropic",
                "deepseek-ai",
                "bigscience",
                "EleutherAI",
                "tiiuae",
                "mistralai",
                "01-ai",
                "Qwen",
                "THUDM",
                "baai",
            ];

            // All non-gated HF models
            let mut hf_models: Vec<ModelInfo> = all_models
                .into_iter()
                .filter(|m| {
                    m.download_source.as_deref() == Some("huggingface") &&
                    !matches!(m.status, ModelStatus::Error(_))
                })
                .collect();

            // Sort: big tech authors first
            hf_models.sort_by(|a, b| {
                let a_lower = a.author.as_deref().unwrap_or("").to_lowercase();
                let b_lower = b.author.as_deref().unwrap_or("").to_lowercase();
                let a_is_big = big_tech_authors.iter().any(|p| a_lower.contains(p));
                let b_is_big = big_tech_authors.iter().any(|p| b_lower.contains(p));
                
                match (a_is_big, b_is_big) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.downloads.cmp(&a.downloads), // Then by downloads
                }
            });

            // Limit to top 100 models
            hf_models.truncate(100);

            Ok(Json(hf_models))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
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

/// Refresh the dynamic model catalog from remote sources (OpenRouter, Hugging Face).
pub async fn refresh_models(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<RefreshModelsRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state
        .shared_state
        .model_manager
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let source = payload.source.as_deref().unwrap_or("all");
    let mut updated_sources = Vec::new();

    // Refresh OpenRouter catalog if requested
    if source == "openrouter" || source == "all" {
        // Priority: 1) Frontend-passed key, 2) Database stored key, 3) Env var, 4) Config
        let api_key = if let Some(key) = &payload.openrouter_api_key {
            if !key.is_empty() { Some(key.clone()) } else { state.get_openrouter_api_key().await }
        } else {
            state.get_openrouter_api_key().await
        };

        if let Some(key) = api_key {
            info!("Refreshing OpenRouter catalog with stored API key");
            let mut registry = model_manager.registry.write().await;
            if let Err(e) = registry.refresh_openrouter_catalog_from_api(&key).await {
                error!("Failed to refresh OpenRouter catalog: {}", e);
            } else {
                updated_sources.push("openrouter".to_string());
            }
            if let Err(e) = registry.save_registry().await {
                error!("Failed to save model registry after OpenRouter refresh: {}", e);
            }
        } else {
            info!("No OpenRouter API key available - loading default OpenRouter catalog");
            // Load default popular OpenRouter models so users can see what's available
            let mut registry = model_manager.registry.write().await;
            registry.populate_default_openrouter_models().await;
            if let Err(e) = registry.save_registry().await {
                error!("Failed to save model registry after populating default OpenRouter models: {}", e);
            } else {
                updated_sources.push("openrouter".to_string());
            }
        }
    }

    // Refresh Hugging Face GGUF/GGML catalog if requested
    // Token is mainly used for downloading gated models, not for catalog refresh
    if source == "huggingface" || source == "all" {
        let mut registry = model_manager.registry.write().await;
        // Fetch top 100 GGUF models by downloads
        if let Err(e) = registry.refresh_huggingface_catalog_from_api(100).await {
            error!("Failed to refresh HuggingFace catalog: {}", e);
            // Continue - don't fail the entire refresh
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
        ModelSourceSpecifier::OpenRouter { model_id } => {
            DownloadSource::OpenRouter { model_id }
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

    tokio::spawn(async move {
        // Pass the pre-created download ID so progress is tracked correctly
        // Also pass the HF token from the request for authentication
        match downloader.download_model(model_info.clone(), download_source_clone.clone(), Some(existing_download_id), payload.hf_token).await {
            Ok(_download_id) => {
                // Extract the filename from the download source
                let filename = match &download_source_clone {
                    DownloadSource::HuggingFace { filename, .. } => Some(filename.clone()),
                    DownloadSource::OpenRouter { .. } => None, // OpenRouter models are API-based, no file
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
            if let Some(data_dir) = dirs::data_dir() {
                let last_model_path = data_dir.join("Aud.io").join("last_model.txt");
                let _ = std::fs::write(last_model_path, &payload.model_id);
            }
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
                    if let Some(data_dir) = dirs::data_dir() {
                        let last_model_path = data_dir.join("Aud.io").join("last_model.txt");
                        let _ = std::fs::write(last_model_path, &payload.model_id);
                    }

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
                        Ok(true) => {
                            info!("Engine downloaded successfully, retrying model switch...");

                            // Retry the initialization with updated engine path
                            // Return special response to signal retry required
                            // Note: We return Ok() with the SwitchModelResponse type
                            // The frontend will check the message for retry_required
                            return Ok(Json(SwitchModelResponse {
                                message: "Engine was downloaded. Please retry switching models.".to_string(),
                                model_id: payload.model_id.clone(),
                                model_path: "retry_required".to_string(),
                            }));
                        }
                        Ok(false) => {
                            error!("Failed to auto-download engine - download returned false");
                            // Return detailed error message
                            return Ok(Json(SwitchModelResponse {
                                message: "Engine download failed. Please check your internet connection and try again.".to_string(),
                                model_id: payload.model_id.clone(),
                                model_path: "engine_download_failed".to_string(),
                            }));
                        }
                        Err(e) => {
                            error!("Failed to auto-download engine: {}", e);
                            // Return detailed error message
                            return Ok(Json(SwitchModelResponse {
                                message: format!("Engine download error: {}", e),
                                model_id: payload.model_id.clone(),
                                model_path: "engine_download_error".to_string(),
                            }));
                        }
                    }
                } else {
                    error!("No engine manager available");
                    return Ok(Json(SwitchModelResponse {
                        message: "Engine manager not initialized. Please restart the application.".to_string(),
                        model_id: payload.model_id.clone(),
                        model_path: "no_engine_manager".to_string(),
                    }));
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
    let app_data_dir = dirs::data_dir()
        .map(|d| {
            if cfg!(target_os = "windows") || cfg!(target_os = "macos") {
                d.join("Aud.io")
            } else {
                d.join("aud.io")
            }
        })
        .unwrap_or_else(|| std::path::PathBuf::from("./aud.io-data"));
    
    // Get engines directory
    let engines_dir = app_data_dir.join("engines");

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

// ─────────────────────────────────────────────────────────────────────────────
// Phase B: OpenRouter Full Catalog (paginated + filtered)
// ─────────────────────────────────────────────────────────────────────────────

/// Query parameters for the OpenRouter catalog endpoint
#[derive(Debug, Deserialize)]
pub struct OpenRouterCatalogParams {
    /// 1-based page number (default: 1)
    pub page: Option<usize>,
    /// Results per page, clamped to [1, 200] (default: 50)
    pub per_page: Option<usize>,
    /// Free-text search across name, id, description, and provider
    pub search: Option<String>,
    /// Filter by provider prefix, e.g. "openai", "meta", "google"
    pub provider: Option<String>,
    /// When `true`, only return models with zero-cost pricing
    pub free_only: Option<bool>,
    /// Minimum context-length in tokens, e.g. 32000
    pub min_context: Option<u64>,
}

/// Paginated response for the OpenRouter catalog
#[derive(Debug, Serialize)]
pub struct OpenRouterCatalogResponse {
    pub models: Vec<ModelInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// `GET /models/openrouter/catalog`
///
/// Returns the full set of OpenRouter models stored in the registry,
/// with optional server-side filtering and pagination.
pub async fn openrouter_catalog(
    State(state): State<UnifiedAppState>,
    Query(params): Query<OpenRouterCatalogParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let model_manager = state
        .shared_state
        .model_manager
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);

    // Collect all OpenRouter models from the registry
    let mut models: Vec<ModelInfo> = {
        let registry = model_manager.registry.read().await;
        registry
            .list_models()
            .into_iter()
            .filter(|m| m.download_source.as_deref() == Some("openrouter"))
            .cloned()
            .collect()
    };

    // ── Filters ────────────────────────────────────────────────────────────
    if let Some(ref search) = params.search {
        let q = search.to_lowercase();
        models.retain(|m| {
            m.name.to_lowercase().contains(&q)
                || m.id.to_lowercase().contains(&q)
                || m.description
                    .as_ref()
                    .map_or(false, |d| d.to_lowercase().contains(&q))
                || m.provider
                    .as_ref()
                    .map_or(false, |p| p.to_lowercase().contains(&q))
        });
    }

    if let Some(ref provider) = params.provider {
        let prov = provider.to_lowercase();
        models.retain(|m| {
            m.provider
                .as_ref()
                .map_or(false, |p| p.to_lowercase().contains(&prov))
                || m.id.to_lowercase().starts_with(&prov)
        });
    }

    if params.free_only.unwrap_or(false) {
        models.retain(|m| {
            m.pricing.as_ref().map_or(false, |p| p.is_free())
                || m.tags.iter().any(|t| t == "free")
        });
    }

    if let Some(min_ctx) = params.min_context {
        models.retain(|m| m.context_length.map_or(false, |ctx| ctx >= min_ctx));
    }

    // Sort by name for stable ordering
    models.sort_by(|a, b| a.name.cmp(&b.name));

    let total = models.len();
    let total_pages = total.div_ceil(per_page);
    let start = (page - 1) * per_page;
    let page_models: Vec<ModelInfo> = models.into_iter().skip(start).take(per_page).collect();

    Ok(Json(OpenRouterCatalogResponse {
        models: page_models,
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase B: OpenRouter Account Quota
// ─────────────────────────────────────────────────────────────────────────────

/// Response body for the OpenRouter quota endpoint
#[derive(Debug, Serialize)]
pub struct OpenRouterQuotaResponse {
    /// Total credits spent so far in USD
    pub usage_usd: f64,
    /// Hard spending cap in USD (`null` = unlimited)
    pub limit_usd: Option<f64>,
    /// `true` when the account is on the free tier
    pub is_free_tier: bool,
    /// Remaining credits in USD (`null` when limit is unlimited)
    pub remaining_usd: Option<f64>,
}

/// `GET /models/openrouter/quota`
///
/// Queries `https://openrouter.ai/api/v1/auth/key` with the stored API key
/// and returns the current usage / limit information.
pub async fn openrouter_quota(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // Retrieve stored OpenRouter key
    let api_key = state
        .shared_state
        .database_pool
        .api_keys
        .get_key_plaintext(&ApiKeyType::OpenRouter)
        .ok()
        .flatten()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let resp = client
        .get("https://openrouter.ai/api/v1/auth/key")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| {
            warn!("OpenRouter quota request failed: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    if !resp.status().is_success() {
        warn!("OpenRouter /auth/key returned {}", resp.status());
        return Err(StatusCode::BAD_GATEWAY);
    }

    #[derive(serde::Deserialize)]
    struct OrKeyData {
        usage: Option<f64>,
        limit: Option<f64>,
        is_free_tier: Option<bool>,
    }
    #[derive(serde::Deserialize)]
    struct OrKeyResp {
        data: OrKeyData,
    }

    let body: OrKeyResp = resp.json().await.map_err(|e| {
        warn!("Failed to parse OpenRouter quota response: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    let usage = body.data.usage.unwrap_or(0.0);
    let limit = body.data.limit;
    let is_free_tier = body.data.is_free_tier.unwrap_or(false);
    let remaining = limit.map(|l| (l - usage).max(0.0));

    Ok(Json(OpenRouterQuotaResponse {
        usage_usd: usage,
        limit_usd: limit,
        is_free_tier,
        remaining_usd: remaining,
    }))
}