//! Model Registry
//!
//! Manages model metadata, tracks installed models, and provides
//! querying capabilities for available models.

use super::storage::{ModelStorage, ModelMetadata};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::recommendation::{HardwareProfile, ModelRecommender};
use reqwest::Client;

/// Status of a model in the registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    /// Model is available locally
    Installed,
    /// Model is being downloaded
    Downloading,
    /// Model is available for download
    Available,
    /// Model had an error during download/installation
    Error(String),
}

/// Public pricing information for an API model (e.g., OpenRouter).
/// Both fields are decimal strings; "0" means the model is free.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per prompt token — "0" means free
    pub prompt: String,
    /// Cost per completion token — "0" means free
    pub completion: String,
}

impl ModelPricing {
    pub fn is_free(&self) -> bool {
        (self.prompt == "0" || self.prompt.is_empty())
            && (self.completion == "0" || self.completion.is_empty())
    }
}

/// Information about a model in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub status: ModelStatus,
    pub size_bytes: u64,
    pub format: String,
    pub download_source: Option<String>,
    /// Specific filename to download (for HuggingFace models with non-standard naming)
    #[serde(default)]
    pub filename: Option<String>,
    pub installed_version: Option<String>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
    pub compatibility_score: Option<f32>, // 0.0 to 1.0 based on hardware match
    /// Parameter count string (e.g., "7B", "70B", "671B")
    #[serde(default)]
    pub parameters: Option<String>,
    /// Context length in tokens
    #[serde(default)]
    pub context_length: Option<u64>,
    /// Provider name (for OpenRouter models)
    #[serde(default)]
    pub provider: Option<String>,
    /// Total number of shards for sharded models (None for single-file models)
    #[serde(default)]
    pub total_shards: Option<u32>,
    /// List of all shard filenames for sharded models
    #[serde(default)]
    pub shard_filenames: Vec<String>,
    /// Download count (for HuggingFace models)
    #[serde(default)]
    pub downloads: u64,
    /// Whether this HuggingFace model requires access approval from the repo owner
    #[serde(default)]
    pub is_gated: bool,
    /// Pricing info for OpenRouter API models (None for offline/HF models)
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
}

/// Model registry manager
pub struct ModelRegistry {
    storage: Arc<ModelStorage>,
    models: HashMap<String, ModelInfo>,
    known_sources: Vec<ModelSource>,
}

/// Source where models can be found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSource {
    pub name: String,
    pub url: String,
    pub api_type: SourceType,
}

/// Type of model source API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    HuggingFace,
    OpenRouter,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

/// Pricing block returned by the OpenRouter /models API.
/// Free models have both fields as the string "0".
#[derive(Debug, Deserialize, Default)]
struct OpenRouterPricing {
    /// Cost per prompt token as a string, e.g. "0" for free.
    #[serde(default)]
    prompt: String,
    /// Cost per completion token as a string, e.g. "0" for free.
    #[serde(default)]
    completion: String,
}

impl OpenRouterPricing {
    /// Returns true when both prompt and completion are zero-cost.
    fn is_free(&self) -> bool {
        (self.prompt == "0" || self.prompt.is_empty())
            && (self.completion == "0" || self.completion.is_empty())
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<u64>,
    #[serde(default)]
    architecture: Option<OpenRouterArchitecture>,
    /// Pricing info — present in the live API but optional for backwards compat.
    #[serde(default)]
    pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenRouterArchitecture {
    #[serde(default)]
    modality: Option<String>,
    #[serde(default)]
    tokenizer: Option<String>,
    /// Instruction type like "none" or "vicuna" etc.
    #[serde(default)]
    instruct_type: Option<String>,
}

/// Hugging Face API model response
#[derive(Debug, Deserialize)]
struct HuggingFaceModel {
    id: String,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    author: Option<String>,
    downloads: Option<u64>,
    /// Gated status: false, "auto", or "manual". Gated models require user approval.
    #[serde(default)]
    gated: Option<serde_json::Value>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    siblings: Vec<HuggingFaceSibling>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceSibling {
    rfilename: String,
    #[serde(default)]
    size: Option<u64>,
}

impl ModelRegistry {
    pub fn new(storage: Arc<ModelStorage>) -> Result<Self> {
        let mut registry = Self {
            storage,
            models: HashMap::new(),
            known_sources: vec![
                ModelSource {
                    name: "Hugging Face".to_string(),
                    url: "https://huggingface.co".to_string(),
                    api_type: SourceType::HuggingFace,
                },
                ModelSource {
                    name: "OpenRouter".to_string(),
                    url: "https://openrouter.ai".to_string(),
                    api_type: SourceType::OpenRouter,
                },
            ],
        };

        // Load existing registry data
        registry.load_registry()?;

        // Populate default catalog (only adds models not already present)
        registry.populate_default_catalog();

        Ok(registry)
    }

    /// Refresh the OpenRouter model catalog from the live OpenRouter API.
    /// This replaces any existing OpenRouter entries with the up-to-date list
    /// of models visible to the provided API key.
    pub async fn refresh_openrouter_catalog_from_api(
        &mut self,
        api_key: &str,
    ) -> Result<()> {
        let client = Client::new();
        let resp = client
            .get("https://openrouter.ai/api/v1/models")
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .context("Failed to call OpenRouter /models API")?;

        let resp = resp.error_for_status().context("OpenRouter /models returned error status")?;
        let body: OpenRouterModelsResponse = resp
            .json()
            .await
            .context("Failed to parse OpenRouter models response")?;

        // Track which OpenRouter model IDs are present in the latest response
        let mut openrouter_ids: HashSet<String> = HashSet::new();

        for m in body.data.into_iter() {
            let plain_id = m.id.clone();

            let registry_id = format!("openrouter:{}", plain_id);
            openrouter_ids.insert(registry_id.clone());

            // Determine free vs paid for tagging purposes
            let is_free = m.pricing.as_ref().map_or(true, |p| p.is_free())
                || plain_id.ends_with(":free");

            // Derive provider tag from the model id prefix (e.g. "openai/gpt-4o")
            let provider = plain_id
                .split('/')
                .next()
                .unwrap_or("")
                .to_lowercase();

            let mut tags = vec![
                "api".to_string(),
                "online".to_string(),
                "cloud".to_string(),
            ];
            if is_free {
                tags.push("free".to_string());
            } else {
                tags.push("paid".to_string());
            }
            if !provider.is_empty() {
                tags.push(format!("provider:{}", provider));
            }

            if let Some(ctx) = m.context_length {
                if ctx >= 128_000 {
                    tags.push("context:xl".to_string());
                } else if ctx >= 32_000 {
                    tags.push("context:large".to_string());
                } else if ctx >= 8_000 {
                    tags.push("context:medium".to_string());
                } else {
                    tags.push("context:small".to_string());
                }
            }

            // Extract parameter count from model name (e.g., "70B", "7B", "671B")
            let parameters = {
                let name_str = m.name.as_deref().unwrap_or(&plain_id);
                // Match patterns like "70B", "7B", "671B", "1.5B", "8x7B"
                let re = regex::Regex::new(r"(\d+(?:\.\d+)?(?:x\d+)?[BMK])").ok();
                re.and_then(|r| r.find(name_str).map(|m| m.as_str().to_string()))
            };

            // Capitalize provider name for display
            let provider_display = if !provider.is_empty() {
                let mut chars = provider.chars();
                match chars.next() {
                    Some(c) => Some(c.to_uppercase().collect::<String>() + chars.as_str()),
                    None => None,
                }
            } else {
                None
            };

            let pricing = m.pricing.as_ref().map(|p| ModelPricing {
                prompt: p.prompt.clone(),
                completion: p.completion.clone(),
            });

            let model_info = ModelInfo {
                id: registry_id.clone(),
                name: m.name.clone().unwrap_or_else(|| plain_id.clone()),
                description: m.description.clone(),
                author: provider_display.clone(),
                status: ModelStatus::Available,
                size_bytes: 0,
                format: "api".to_string(),
                download_source: Some("openrouter".to_string()),
                filename: None,
                installed_version: None,
                last_updated: None,
                tags,
                compatibility_score: None,
                parameters,
                context_length: m.context_length,
                provider: provider_display,
                total_shards: None,
                shard_filenames: vec![],
                downloads: 0,
                is_gated: false,
                pricing,
            };

            // Insert or update existing OpenRouter entry
            self.models
                .entry(registry_id.clone())
                .and_modify(|existing| {
                    existing.name = model_info.name.clone();
                    existing.description = model_info.description.clone();
                    existing.status = model_info.status.clone();
                    existing.format = model_info.format.clone();
                    existing.download_source = model_info.download_source.clone();
                    existing.tags = model_info.tags.clone();
                    existing.pricing = model_info.pricing.clone();
                })
                .or_insert(model_info);
        }

        // Remove any stale OpenRouter models that are no longer returned
        self.models.retain(|id, model| {
            if model.download_source.as_deref() == Some("openrouter") {
                openrouter_ids.contains(id)
            } else {
                true
            }
        });

        info!("Refreshed OpenRouter catalog, now tracking {} models", openrouter_ids.len());

        Ok(())
    }

    /// Refresh the Hugging Face GGUF/GGML model catalog from the HF Hub API.
    /// Fetches top models by downloads and extracts available quantized files.
    pub async fn refresh_huggingface_catalog_from_api(
        &mut self,
        limit: usize,
    ) -> Result<()> {
        let client = Client::new();

        // Fetch GGUF models sorted by downloads
        let url = format!(
            "https://huggingface.co/api/models?filter=gguf&sort=downloads&direction=-1&limit={}&full=true",
            limit
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Aud.io-Desktop/1.0")
            .send()
            .await
            .context("Failed to call Hugging Face models API")?;

        let resp = resp
            .error_for_status()
            .context("Hugging Face API returned error status")?;

        let models: Vec<HuggingFaceModel> = resp
            .json()
            .await
            .context("Failed to parse Hugging Face models response")?;

        let mut hf_ids: HashSet<String> = HashSet::new();

        for m in models.into_iter() {
            let repo_id = m.model_id.as_ref().unwrap_or(&m.id).clone();

            // Flag gated models — they require HuggingFace access approval.
            // We include them in the catalog so the UI can show a
            // "Request Access" button instead of a direct download button.
            let is_gated = match &m.gated {
                Some(serde_json::Value::Bool(false)) | None => false,
                _ => true, // "auto", "manual", true all count as gated
            };

            // Find GGUF files in siblings
            let gguf_files: Vec<&HuggingFaceSibling> = m
                .siblings
                .iter()
                .filter(|s| {
                    s.rfilename.ends_with(".gguf") || s.rfilename.ends_with(".ggml")
                })
                .collect();

            if gguf_files.is_empty() {
                continue;
            }

            // Check if this is a sharded model by looking for shard patterns
            let mut sharded_model_info = None;
            for file in &gguf_files {
                if let Some(total_shards) = self.detect_shard_pattern_internal(&file.rfilename) {
                    // This is a sharded model, collect all shards
                    let all_shards = self.collect_shards_internal(&gguf_files, total_shards);
                    
                    // Calculate total size
                    let total_size = all_shards.iter()
                        .map(|s| s.size.unwrap_or(0))
                        .sum();
                    
                    let registry_id = repo_id.clone();
                    hf_ids.insert(registry_id.clone());

                    // Determine format from first shard
                    let format = if file.rfilename.ends_with(".gguf") {
                        "gguf"
                    } else {
                        "ggml"
                    }
                    .to_string();

                    // Build tags from HF tags + our own
                    let mut tags: Vec<String> = m
                        .tags
                        .iter()
                        .filter(|t| !t.is_empty() && *t != "gguf" && *t != "ggml")
                        .take(5)
                        .cloned()
                        .collect();
                    tags.push("offline".to_string());
                    tags.push(format.clone());
                    tags.push("sharded".to_string()); // Add sharded tag

                    // Derive a friendly name from repo_id
                    let name = repo_id
                        .split('/')
                        .last()
                        .unwrap_or(&repo_id)
                        .replace("-GGUF", "")
                        .replace("-gguf", "");

                    // Extract parameter count from name
                    let parameters = {
                        let name_str = &name;
                        let re = regex::Regex::new(r"(\d+(?:\.\d+)?(?:x\d+)?[BMK])").ok();
                        re.and_then(|r| r.find(name_str).map(|m| m.as_str().to_string()))
                    };

                    sharded_model_info = Some(ModelInfo {
                        id: registry_id.clone(),
                        name,
                        description: Some(format!("Sharded GGUF model from {} ({} parts)", repo_id, total_shards)),
                        author: m.author.clone(),
                        status: ModelStatus::Available,
                        size_bytes: total_size,
                        format,
                        download_source: Some("huggingface".to_string()),
                        filename: Some(file.rfilename.clone()), // Store the first shard as the primary filename
                        installed_version: None,
                        last_updated: None,
                        tags,
                        compatibility_score: None,
                        parameters,
                        context_length: None,
                        provider: None,
                        total_shards: Some(total_shards),
                        shard_filenames: all_shards.iter().map(|s| s.rfilename.clone()).collect(),
                        downloads: m.downloads.unwrap_or(0),
                        is_gated,
                        pricing: None,
                    });
                    break; // Found sharded model, no need to check other files
                }
            }

            // If we found a sharded model, use that info; otherwise use the preferred single file
            let model_info = if let Some(sharded_info) = sharded_model_info {
                sharded_info
            } else {
                // Prefer Q4_K_M, Q5_K_M, Q6_K, Q8_0, IQ_X_X quantizations (good balance of size/quality)
                let preferred_file = gguf_files
                    .iter()
                    .find(|f| f.rfilename.contains("Q4_K_M"))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q5_K_M")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q6_K")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q8_0")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("IQ3_XXS")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("IQ3_S")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("IQ4_NL")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("IQ4_XS")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q3_K_S")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q3_K_M")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q3_K_L")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q5_K_S")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q5_K_L")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q2_K")))
                    .or_else(|| gguf_files.iter().find(|f| f.rfilename.contains("Q2_K_S")))
                    .or_else(|| gguf_files.first())
                    .copied();

                let Some(file) = preferred_file else {
                    continue;
                };

                let registry_id = repo_id.clone();
                hf_ids.insert(registry_id.clone());

                // Determine format from filename
                let format = if file.rfilename.ends_with(".gguf") {
                    "gguf"
                } else {
                    "ggml"
                }
                .to_string();

                // Build tags from HF tags + our own
                let mut tags: Vec<String> = m
                    .tags
                    .iter()
                    .filter(|t| !t.is_empty() && *t != "gguf" && *t != "ggml")
                    .take(5)
                    .cloned()
                    .collect();
                tags.push("offline".to_string());
                tags.push(format.clone());

                // Derive a friendly name from repo_id
                let name = repo_id
                    .split('/')
                    .last()
                    .unwrap_or(&repo_id)
                    .replace("-GGUF", "")
                    .replace("-gguf", "");

                // Extract parameter count from name
                let parameters = {
                    let name_str = &name;
                    let re = regex::Regex::new(r"(\d+(?:\.\d+)?(?:x\d+)?[BMK])").ok();
                    re.and_then(|r| r.find(name_str).map(|m| m.as_str().to_string()))
                };

                ModelInfo {
                    id: registry_id.clone(),
                    name,
                    description: Some(format!("GGUF model from {}", repo_id)),
                    author: m.author.clone(),
                    status: ModelStatus::Available,
                    size_bytes: file.size.unwrap_or(0),
                    format,
                    download_source: Some("huggingface".to_string()),
                    filename: Some(file.rfilename.clone()),
                    installed_version: None,
                    last_updated: None,
                    tags,
                    compatibility_score: None,
                    parameters,
                    context_length: None,
                    provider: None,
                    total_shards: None,
                    shard_filenames: vec![],
                    downloads: m.downloads.unwrap_or(0),
                    is_gated,
                    pricing: None,
                }
            };

            // Insert or update
            self.models
                .entry(model_info.id.clone())
                .and_modify(|existing| {
                    // Don't overwrite installed models
                    if existing.status != ModelStatus::Installed {
                        existing.name = model_info.name.clone();
                        existing.description = model_info.description.clone();
                        existing.author = model_info.author.clone();
                        existing.size_bytes = model_info.size_bytes;
                        existing.format = model_info.format.clone();
                        existing.download_source = model_info.download_source.clone();
                        existing.filename = model_info.filename.clone();
                        existing.tags = model_info.tags.clone();
                        existing.total_shards = model_info.total_shards;
                        existing.shard_filenames = model_info.shard_filenames.clone();
                        existing.is_gated = model_info.is_gated;
                    }
                })
                .or_insert(model_info);
        }

        info!(
            "Refreshed Hugging Face catalog, now tracking {} GGUF/GGML models",
            hf_ids.len()
        );

        Ok(())
    }

    /// Recompute compatibility scores for all known local models based on
    /// the current hardware profile and user preferences. This is used by
    /// the model manager to support "Best Match" sorting in the UI.
    pub fn update_compatibility_scores(
        &mut self,
        recommender: &ModelRecommender,
        hardware: &HardwareProfile,
    ) {
        for model in self.models.values_mut() {
            // Only score offline models (GGUF/GGML) – API models and other
            // formats are not constrained by local hardware.
            let is_offline_format = model.format.eq_ignore_ascii_case("gguf")
                || model.format.eq_ignore_ascii_case("ggml");
            let is_api_model = model.download_source.as_deref() == Some("openrouter");

            if is_offline_format && !is_api_model {
                let score = recommender.score_model_compatibility(model, hardware);
                model.compatibility_score = Some(score);
            }
        }
    }

    /// Load registry data from persistent storage
    fn load_registry(&mut self) -> Result<()> {
        let registry_path = self.storage.location.registry_dir.join("registry.json");
        if registry_path.exists() {
            match std::fs::read_to_string(&registry_path) {
                Ok(content) if !content.trim().is_empty() => {
                    match serde_json::from_str::<HashMap<String, ModelInfo>>(&content) {
                        Ok(saved_models) => {
                            self.models = saved_models;
                            info!("Loaded {} models from registry", self.models.len());
                        }
                        Err(e) => {
                            warn!("Registry file corrupted, starting fresh: {}", e);
                        }
                    }
                }
                Ok(_) => {
                    debug!("Registry file is empty, starting fresh");
                }
                Err(e) => {
                    warn!("Failed to read registry file: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Scan local storage for existing models and populate registry
    pub async fn scan_storage(&mut self) -> Result<()> {
        let model_ids = self.storage.list_models()?;
        
        for model_id in model_ids {
            if let Some(metadata) = self.load_model_metadata(&model_id).await? {
                let model_info = ModelInfo {
                    id: model_id.clone(),
                    name: metadata.name,
                    description: metadata.description,
                    author: metadata.author,
                    status: ModelStatus::Installed,
                    size_bytes: metadata.size_bytes,
                    format: metadata.format,
                    download_source: Some(metadata.download_source),
                    filename: None, // Already downloaded, filename not needed
                    installed_version: None, // Version extracted from model metadata
                    last_updated: Some(metadata.download_date),
                    tags: metadata.tags,
                    compatibility_score: None, // Will be calculated on demand
                    parameters: None,
                    context_length: None,
                    provider: None,
                    total_shards: None,
                    shard_filenames: vec![],
                    downloads: 0,
                    is_gated: false,
                    pricing: None,
                };
                
                self.models.insert(model_id, model_info);
            }
        }
        
        info!("Scanned storage and found {} models", self.models.len());
        Ok(())
    }

    /// Load metadata for a specific model
    async fn load_model_metadata(&self, model_id: &str) -> Result<Option<ModelMetadata>> {
        let metadata_path = self.storage.metadata_path(model_id);
        
        if metadata_path.exists() {
            let content = tokio::fs::read_to_string(&metadata_path).await?;
            let metadata: ModelMetadata = serde_json::from_str(&content)?;
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    /// Update model status based on file existence
    pub async fn update_model_status_from_storage(&mut self, model_id: &str) -> Result<()> {
        if let Some(model_info) = self.models.get_mut(model_id) {
            let model_exists = self.storage.model_exists(model_id);
            
            if model_exists {
                model_info.status = ModelStatus::Installed;
            } else {
                // If it was installed but no longer exists, mark as available (downloadable)
                if matches!(model_info.status, ModelStatus::Installed) {
                    model_info.status = ModelStatus::Available;
                }
            }
        }
        
        Ok(())
    }

    /// Update all model statuses based on file existence in storage
    pub async fn update_all_model_statuses_from_storage(&mut self) -> Result<()> {
        let model_ids: Vec<String> = self.models.keys().cloned().collect();
        
        for model_id in model_ids {
            self.update_model_status_from_storage(&model_id).await?;
        }
        
        Ok(())
    }

    /// Get the path of an installed model by ID
    pub fn get_installed_model_path(&self, model_id: &str) -> Option<std::path::PathBuf> {
        let model_info = self.models.get(model_id)?;
        if model_info.status != ModelStatus::Installed {
            return None;
        }
        
        // Try to get the filename from model_info, otherwise look for any model file in the directory
        if let Some(filename) = &model_info.filename {
            return Some(self.storage.model_path(model_id, filename));
        }
        
        // Look for any model file in the directory
        let temp_path = self.storage.model_path(model_id, "dummy");
        let model_dir = match temp_path.parent() {
            Some(dir) => dir.to_path_buf(),
            None => return None,
        };
        if !model_dir.exists() {
            return None;
        }
        
        if let Ok(entries) = std::fs::read_dir(&model_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                        if matches!(ext.as_str(), "gguf" | "bin" | "ggml" | "onnx" | "trt" | "engine" | "safetensors" | "mlmodel") {
                            return Some(path);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Get the complete model metadata including runtime binaries information
    pub async fn get_model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        match self.load_model_metadata(model_id).await {
            Ok(Some(metadata)) => Some(metadata),
            _ => None,
        }
    }

    /// Add a model to the registry
    pub fn add_model(&mut self, model_info: ModelInfo) {
        self.models.insert(model_info.id.clone(), model_info);
    }

    /// Get model information by ID
    pub fn get_model(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.get(model_id)
    }

    /// Get mutable reference to model information
    pub fn get_model_mut(&mut self, model_id: &str) -> Option<&mut ModelInfo> {
        self.models.get_mut(model_id)
    }

    /// List all models in registry
    pub fn list_models(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    /// List models by status
    pub fn list_models_by_status(&self, status: ModelStatus) -> Vec<&ModelInfo> {
        self.models.values()
            .filter(|model| model.status == status)
            .collect()
    }

    /// Search models by name or tags
    pub fn search_models(&self, query: &str) -> Vec<&ModelInfo> {
        let query_lower = query.to_lowercase();
        self.models.values()
            .filter(|model| {
                model.name.to_lowercase().contains(&query_lower) ||
                model.description.as_ref().map_or(false, |desc| desc.to_lowercase().contains(&query_lower)) ||
                model.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get models sorted by compatibility score for current hardware
    pub fn get_recommended_models(&self, max_results: usize) -> Vec<&ModelInfo> {
        let mut models: Vec<_> = self.models.values().collect();
        models.sort_by(|a, b| {
            b.compatibility_score.unwrap_or(0.0)
                .partial_cmp(&a.compatibility_score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        models.truncate(max_results);
        models
    }

    /// Update model status
    pub fn update_model_status(&mut self, model_id: &str, status: ModelStatus) {
        if let Some(model) = self.models.get_mut(model_id) {
            model.status = status;
        }
    }

    /// Remove a model from registry
    pub fn remove_model(&mut self, model_id: &str) -> bool {
        self.models.remove(model_id).is_some()
    }

    /// Get registry statistics
    pub fn get_statistics(&self) -> RegistryStats {
        let mut stats = RegistryStats::default();
        
        for model in self.models.values() {
            match model.status {
                ModelStatus::Installed => stats.installed_count += 1,
                ModelStatus::Downloading => stats.downloading_count += 1,
                ModelStatus::Available => stats.available_count += 1,
                ModelStatus::Error(_) => stats.error_count += 1,
            }
            stats.total_size_bytes += model.size_bytes;
        }
        
        stats
    }

    /// Get models by category/tags
    pub fn get_models_by_category(&self, category: &str) -> Vec<&ModelInfo> {
        self.models.values()
            .filter(|model| {
                model.tags.iter().any(|tag| 
                    tag.to_lowercase().contains(&category.to_lowercase())
                )
            })
            .collect()
    }

    /// Get trending models (recently added or popular tags)
    pub fn get_trending_models(&self, limit: usize) -> Vec<&ModelInfo> {
        let mut models: Vec<&ModelInfo> = self.models.values()
            .filter(|model| {
                // Filter for popular models (based on certain tags)
                model.tags.iter().any(|tag| 
                    tag == "popular" || tag == "trending" || tag == "featured"
                )
            })
            .collect();
        
        // Sort by some criteria (e.g., size as proxy for popularity, or by name)
        models.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        models.truncate(limit);
        models
    }

    /// Get models by task type (e.g., "chat", "coding", "text-generation")
    pub fn get_models_by_task(&self, task: &str) -> Vec<&ModelInfo> {
        self.models.values()
            .filter(|model| {
                // Look in name, description and tags for the task
                model.name.to_lowercase().contains(&task.to_lowercase()) ||
                model.description.as_ref().map_or(false, |desc| 
                    desc.to_lowercase().contains(&task.to_lowercase())) ||
                model.tags.iter().any(|tag| 
                    tag.to_lowercase().contains(&task.to_lowercase()))
            })
            .collect()
    }

    /// Save registry to persistent storage
    pub async fn save_registry(&self) -> Result<()> {
        let registry_path = self.storage.location.registry_dir.join("registry.json");
        let content = serde_json::to_string_pretty(&self.models)
            .context("Failed to serialize registry")?;
        tokio::fs::write(&registry_path, content).await
            .context("Failed to write registry file")?;
        debug!("Saved {} models to registry", self.models.len());
        Ok(())
    }

    /// Populate the registry with well-known models from all sources.
    /// Only adds models that are not already in the registry.
    /// Also removes stale models that are no longer available.
    pub fn populate_default_catalog(&mut self) {
        let catalog = Self::get_default_catalog();
        let catalog_ids: std::collections::HashSet<String> = catalog.iter().map(|m| m.id.clone()).collect();

        // Remove stale models: Ollama models (functionality removed) and any other obsolete models
        let stale_ids: Vec<String> = self.models.iter()
            .filter(|(id, m)| {
                // Remove all Ollama models (functionality removed)
                m.download_source.as_deref() == Some("ollama")
                // Note: We no longer remove OpenRouter models based on static catalog since they come from live API
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &stale_ids {
            self.models.remove(id);
        }
        if !stale_ids.is_empty() {
            info!("Removed {} stale/obsolete models from registry", stale_ids.len());
        }

        let mut added = 0;
        for model in catalog {
            if !self.models.contains_key(&model.id) {
                self.models.insert(model.id.clone(), model);
                added += 1;
            }
        }
        if added > 0 {
            info!("Populated catalog with {} new available models", added);
        }
    }

    /// Returns the built-in catalog of well-known models
    /// Currently returns an empty vector as models are loaded dynamically from APIs
    fn get_default_catalog() -> Vec<ModelInfo> {
        vec![]
    }

    /// Populate registry with OpenRouter models fetched from the public API.
    /// This is called when no API key is available to show users what models are available.
    pub async fn populate_default_openrouter_models(&mut self) {
        if let Err(e) = self.fetch_public_openrouter_models().await {
            warn!("Failed to fetch public OpenRouter models: {}", e);
            // Optionally, could fall back to a minimal static list if API fails
        }
    }

    /// Fetch public models from OpenRouter API without authentication.
    /// This method fetches the publicly available models that anyone can see.
    async fn fetch_public_openrouter_models(&mut self) -> Result<()> {
        let client = Client::new();
        let resp = client
            .get("https://openrouter.ai/api/v1/models")
            .send()
            .await
            .context("Failed to call OpenRouter public /models API")?;

        let resp = resp.error_for_status().context("OpenRouter public /models returned error status")?;
        let body: OpenRouterModelsResponse = resp
            .json()
            .await
            .context("Failed to parse OpenRouter public models response")?;

        let mut added = 0;

        for m in body.data.into_iter() {
            let plain_id = m.id.clone();
            
            // Validate and skip known deprecated/invalid models
            if self.is_invalid_openrouter_model(&plain_id) {
                debug!("Skipping invalid model: {}", plain_id);
                continue;
            }
            
            let registry_id = format!("openrouter:{}", plain_id);

            // Derive provider tag from the model id prefix (e.g. "openai/gpt-4o")
            let provider = plain_id
                .split('/')
                .next()
                .unwrap_or("")
                .to_lowercase();

            let mut tags = vec![
                "api".to_string(),
                "online".to_string(),
                "cloud".to_string(),
            ];
            if !provider.is_empty() {
                tags.push(format!("provider:{}", provider));
            }

            if let Some(ctx) = m.context_length {
                if ctx >= 128_000 {
                    tags.push("context:xl".to_string());
                } else if ctx >= 32_000 {
                    tags.push("context:large".to_string());
                } else if ctx >= 8_000 {
                    tags.push("context:medium".to_string());
                } else {
                    tags.push("context:small".to_string());
                }
            }

            // Extract parameter count from model name (e.g., "70B", "7B", "671B")
            let parameters = {
                let name_str = m.name.as_deref().unwrap_or(&plain_id);
                // Match patterns like "70B", "7B", "671B", "1.5B", "8x7B"
                let re = regex::Regex::new(r"(\d+(?:\.\d+)?(?:x\d+)?[BMK])").ok();
                re.and_then(|r| r.find(name_str).map(|m| m.as_str().to_string()))
            };

            // Capitalize provider name for display
            let provider_display = if !provider.is_empty() {
                let mut chars = provider.chars();
                match chars.next() {
                    Some(c) => Some(c.to_uppercase().collect::<String>() + chars.as_str()),
                    None => None,
                }
            } else {
                None
            };

            let is_free = m.pricing.as_ref().map_or(true, |p| p.is_free())
                || plain_id.ends_with(":free");
            if is_free {
                tags.push("free".to_string());
            } else {
                tags.push("paid".to_string());
            }

            let pricing = m.pricing.as_ref().map(|p| ModelPricing {
                prompt: p.prompt.clone(),
                completion: p.completion.clone(),
            });

            let model_info = ModelInfo {
                id: registry_id.clone(),
                name: m.name.clone().unwrap_or_else(|| plain_id.clone()),
                description: m.description.clone(),
                author: provider_display.clone(),
                status: ModelStatus::Available,
                size_bytes: 0,
                format: "api".to_string(),
                download_source: Some("openrouter".to_string()),
                filename: None,
                installed_version: None,
                last_updated: Some(chrono::Utc::now()),
                tags,
                compatibility_score: None,
                parameters,
                context_length: m.context_length,
                provider: provider_display,
                total_shards: None,
                shard_filenames: vec![],
                downloads: 0,
                is_gated: false,
                pricing,
            };

            // Insert or update - this will replace any existing entry
            self.models.insert(registry_id, model_info);
            added += 1;
        }

        info!("Fetched {} public OpenRouter models from API", added);
        Ok(())
    }

    /// Detect if the filename follows a shard pattern (e.g., model-00001-of-00003.gguf)
    fn detect_shard_pattern_internal(&self, filename: &str) -> Option<u32> {
        // Pattern: some-name-00001-of-00003.ext
        let re = regex::Regex::new(r".*-(\d{5})-of-(\d{5})\.[^.]+$").ok()?;
        if let Some(caps) = re.captures(filename) {
            if let Some(total_str) = caps.get(2) {
                if let Ok(total) = total_str.as_str().parse::<u32>() {
                    return Some(total);
                }
            }
        }
        None
    }

    /// Collect all shards for a given total_shards number
    fn collect_shards_internal<'a>(&self, gguf_files: &[&'a HuggingFaceSibling], total_shards: u32) -> Vec<&'a HuggingFaceSibling> {
        let mut shards = Vec::new();
        
        // Find the pattern from one of the shard files
        if let Some(first_file) = gguf_files.iter().find(|f| self.detect_shard_pattern_internal(&f.rfilename).is_some()) {
            // Extract the pattern from the first file to find other shards
            if let Some(caps) = regex::Regex::new(r"(.*-)(\d{5})(-of-\d{5}\.[^.]+)$")
                .ok()
                .and_then(|re| re.captures(&first_file.rfilename)) {
                
                let prefix = caps[1].to_string();  // Owned string to avoid lifetime issues
                let suffix = caps[3].to_string();  // Owned string to avoid lifetime issues
                
                // Collect all expected shard files
                for i in 1..=total_shards {
                    let expected_filename = format!("{}{:05}{}", prefix, i, suffix);
                    if let Some(file) = gguf_files.iter().find(|f| f.rfilename == expected_filename) {
                        shards.push(*file);
                    }
                }
            }
        }
        
        shards
    }

    /// Check if an OpenRouter model ID is invalid/deprecated
    fn is_invalid_openrouter_model(&self, model_id: &str) -> bool {
        // Known deprecated/invalid models
        model_id == "google/gemini-pro" || 
        model_id == "google/palm-2-chat-bison" ||
        model_id.starts_with("google/palm") ||
        model_id.starts_with("google/gemini-pro") ||
        // Additional checks can be added here as needed
        false
    }
}

/// Registry statistics
#[derive(Debug, Default)]
pub struct RegistryStats {
    pub installed_count: usize,
    pub downloading_count: usize,
    pub available_count: usize,
    pub error_count: usize,
    pub total_size_bytes: u64,
}

impl RegistryStats {
    pub fn total_models(&self) -> usize {
        self.installed_count + self.downloading_count + self.available_count + self.error_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_registry_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Arc::new(ModelStorage {
            location: super::super::storage::StorageLocation {
                app_data_dir: temp_dir.path().to_path_buf(),
                models_dir: temp_dir.path().join("models"),
                registry_dir: temp_dir.path().join("registry"),
            },
        });
        
        let registry = ModelRegistry::new(storage)?;
        assert_eq!(registry.models.len(), 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_model_addition_and_lookup() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Arc::new(ModelStorage {
            location: super::super::storage::StorageLocation {
                app_data_dir: temp_dir.path().to_path_buf(),
                models_dir: temp_dir.path().join("models"),
                registry_dir: temp_dir.path().join("registry"),
            },
        });
        
        let mut registry = ModelRegistry::new(storage)?;
        
        let model_info = ModelInfo {
            id: "test-model".to_string(),
            name: "Test Model".to_string(),
            description: Some("A test model".to_string()),
            author: Some("Test Author".to_string()),
            status: ModelStatus::Available,
            size_bytes: 1024,
            format: "gguf".to_string(),
            download_source: Some("huggingface".to_string()),
            filename: None,
            installed_version: None,
            last_updated: None,
            tags: vec!["test".to_string()],
            compatibility_score: Some(0.8),
            parameters: None,
            context_length: None,
            provider: None,
            total_shards: None,
            shard_filenames: vec![],
            downloads: 0,
            is_gated: false,
            pricing: None,
        };
        
        registry.add_model(model_info);
        assert_eq!(registry.models.len(), 1);
        
        let retrieved = registry.get_model("test-model");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Model");
        
        Ok(())
    }
}