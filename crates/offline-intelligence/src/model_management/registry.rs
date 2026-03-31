
use super::storage::{ModelStorage, ModelMetadata};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::recommendation::{HardwareProfile, ModelRecommender};
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    
    Installed,
    
    Downloading,
    
    Available,
    
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    
    pub prompt: String,
    
    pub completion: String,
}

impl ModelPricing {
    pub fn is_free(&self) -> bool {
        (self.prompt == "0" || self.prompt.is_empty())
            && (self.completion == "0" || self.completion.is_empty())
    }
}

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
    
    #[serde(default)]
    pub filename: Option<String>,
    pub installed_version: Option<String>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
    pub compatibility_score: Option<f32>, 
    
    #[serde(default)]
    pub parameters: Option<String>,
    
    #[serde(default)]
    pub context_length: Option<u64>,
    
    #[serde(default)]
    pub provider: Option<String>,
    
    #[serde(default)]
    pub total_shards: Option<u32>,
    
    #[serde(default)]
    pub shard_filenames: Vec<String>,
    
    #[serde(default)]
    pub downloads: u64,
    
    #[serde(default)]
    pub is_gated: bool,
    
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
}

pub struct ModelRegistry {
    storage: Arc<ModelStorage>,
    models: HashMap<String, ModelInfo>,
    known_sources: Vec<ModelSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSource {
    pub name: String,
    pub url: String,
    pub api_type: SourceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    HuggingFace,
    OpenRouter,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenRouterPricing {
    
    #[serde(default)]
    prompt: String,
    
    #[serde(default)]
    completion: String,
}

impl OpenRouterPricing {
    
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
    
    #[serde(default)]
    pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenRouterArchitecture {
    #[serde(default)]
    modality: Option<String>,
    #[serde(default)]
    tokenizer: Option<String>,
    
    #[serde(default)]
    instruct_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceModel {
    id: String,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    author: Option<String>,
    downloads: Option<u64>,
    
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

        registry.load_registry()?;

        registry.populate_default_catalog();

        Ok(registry)
    }

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

        let mut openrouter_ids: HashSet<String> = HashSet::new();

        for m in body.data.into_iter() {
            let plain_id = m.id.clone();

            let registry_id = format!("openrouter:{}", plain_id);
            openrouter_ids.insert(registry_id.clone());

            let is_free = m.pricing.as_ref().map_or(true, |p| p.is_free())
                || plain_id.ends_with(":free");

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

            let parameters = {
                let name_str = m.name.as_deref().unwrap_or(&plain_id);
                
                let re = regex::Regex::new(r"(\d+(?:\.\d+)?(?:x\d+)?[BMK])").ok();
                re.and_then(|r| r.find(name_str).map(|m| m.as_str().to_string()))
            };

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

    pub async fn refresh_huggingface_catalog_from_api(
        &mut self,
        limit: usize,
    ) -> Result<()> {
        let client = Client::new();

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

            let is_gated = match &m.gated {
                Some(serde_json::Value::Bool(false)) | None => false,
                _ => true, 
            };

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

            let mut sharded_model_info = None;
            for file in &gguf_files {
                if let Some(total_shards) = self.detect_shard_pattern_internal(&file.rfilename) {
                    
                    let all_shards = self.collect_shards_internal(&gguf_files, total_shards);
                    
                    let total_size = all_shards.iter()
                        .map(|s| s.size.unwrap_or(0))
                        .sum();
                    
                    let registry_id = repo_id.clone();
                    hf_ids.insert(registry_id.clone());

                    let format = if file.rfilename.ends_with(".gguf") {
                        "gguf"
                    } else {
                        "ggml"
                    }
                    .to_string();

                    let mut tags: Vec<String> = m
                        .tags
                        .iter()
                        .filter(|t| !t.is_empty() && *t != "gguf" && *t != "ggml")
                        .take(5)
                        .cloned()
                        .collect();
                    tags.push("offline".to_string());
                    tags.push(format.clone());
                    tags.push("sharded".to_string()); 

                    let name = repo_id
                        .split('/')
                        .last()
                        .unwrap_or(&repo_id)
                        .replace("-GGUF", "")
                        .replace("-gguf", "");

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
                        filename: Some(file.rfilename.clone()), 
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
                    break; 
                }
            }

            let model_info = if let Some(sharded_info) = sharded_model_info {
                sharded_info
            } else {
                
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

                let format = if file.rfilename.ends_with(".gguf") {
                    "gguf"
                } else {
                    "ggml"
                }
                .to_string();

                let mut tags: Vec<String> = m
                    .tags
                    .iter()
                    .filter(|t| !t.is_empty() && *t != "gguf" && *t != "ggml")
                    .take(5)
                    .cloned()
                    .collect();
                tags.push("offline".to_string());
                tags.push(format.clone());

                let name = repo_id
                    .split('/')
                    .last()
                    .unwrap_or(&repo_id)
                    .replace("-GGUF", "")
                    .replace("-gguf", "");

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

            self.models
                .entry(model_info.id.clone())
                .and_modify(|existing| {
                    
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

    pub fn update_compatibility_scores(
        &mut self,
        recommender: &ModelRecommender,
        hardware: &HardwareProfile,
    ) {
        for model in self.models.values_mut() {
            
            let is_offline_format = model.format.eq_ignore_ascii_case("gguf")
                || model.format.eq_ignore_ascii_case("ggml");
            let is_api_model = model.download_source.as_deref() == Some("openrouter");

            if is_offline_format && !is_api_model {
                let score = recommender.score_model_compatibility(model, hardware);
                model.compatibility_score = Some(score);
            }
        }
    }

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
                    filename: None, 
                    installed_version: None, 
                    last_updated: Some(metadata.download_date),
                    tags: metadata.tags,
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
                
                self.models.insert(model_id, model_info);
            }
        }
        
        info!("Scanned storage and found {} models", self.models.len());
        Ok(())
    }

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

    pub async fn update_model_status_from_storage(&mut self, model_id: &str) -> Result<()> {
        if let Some(model_info) = self.models.get_mut(model_id) {
            let model_exists = self.storage.model_exists(model_id);
            
            if model_exists {
                model_info.status = ModelStatus::Installed;
            } else {
                
                if matches!(model_info.status, ModelStatus::Installed) {
                    model_info.status = ModelStatus::Available;
                }
            }
        }
        
        Ok(())
    }

    pub async fn update_all_model_statuses_from_storage(&mut self) -> Result<()> {
        let model_ids: Vec<String> = self.models.keys().cloned().collect();
        
        for model_id in model_ids {
            self.update_model_status_from_storage(&model_id).await?;
        }
        
        Ok(())
    }

    pub fn get_installed_model_path(&self, model_id: &str) -> Option<std::path::PathBuf> {
        let model_info = self.models.get(model_id)?;
        if model_info.status != ModelStatus::Installed {
            return None;
        }
        
        if let Some(filename) = &model_info.filename {
            return Some(self.storage.model_path(model_id, filename));
        }
        
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
    
    pub async fn get_model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        match self.load_model_metadata(model_id).await {
            Ok(Some(metadata)) => Some(metadata),
            _ => None,
        }
    }

    pub fn add_model(&mut self, model_info: ModelInfo) {
        self.models.insert(model_info.id.clone(), model_info);
    }

    pub fn get_model(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.get(model_id)
    }

    pub fn get_model_mut(&mut self, model_id: &str) -> Option<&mut ModelInfo> {
        self.models.get_mut(model_id)
    }

    pub fn list_models(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    pub fn list_models_by_status(&self, status: ModelStatus) -> Vec<&ModelInfo> {
        self.models.values()
            .filter(|model| model.status == status)
            .collect()
    }

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

    pub fn update_model_status(&mut self, model_id: &str, status: ModelStatus) {
        if let Some(model) = self.models.get_mut(model_id) {
            model.status = status;
        }
    }

    pub fn remove_model(&mut self, model_id: &str) -> bool {
        self.models.remove(model_id).is_some()
    }

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

    pub fn get_models_by_category(&self, category: &str) -> Vec<&ModelInfo> {
        self.models.values()
            .filter(|model| {
                model.tags.iter().any(|tag| 
                    tag.to_lowercase().contains(&category.to_lowercase())
                )
            })
            .collect()
    }

    pub fn get_trending_models(&self, limit: usize) -> Vec<&ModelInfo> {
        let mut models: Vec<&ModelInfo> = self.models.values()
            .filter(|model| {
                
                model.tags.iter().any(|tag| 
                    tag == "popular" || tag == "trending" || tag == "featured"
                )
            })
            .collect();
        
        models.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        models.truncate(limit);
        models
    }

    pub fn get_models_by_task(&self, task: &str) -> Vec<&ModelInfo> {
        self.models.values()
            .filter(|model| {
                
                model.name.to_lowercase().contains(&task.to_lowercase()) ||
                model.description.as_ref().map_or(false, |desc| 
                    desc.to_lowercase().contains(&task.to_lowercase())) ||
                model.tags.iter().any(|tag| 
                    tag.to_lowercase().contains(&task.to_lowercase()))
            })
            .collect()
    }

    pub async fn save_registry(&self) -> Result<()> {
        let registry_path = self.storage.location.registry_dir.join("registry.json");
        let content = serde_json::to_string_pretty(&self.models)
            .context("Failed to serialize registry")?;
        tokio::fs::write(&registry_path, content).await
            .context("Failed to write registry file")?;
        debug!("Saved {} models to registry", self.models.len());
        Ok(())
    }

    pub fn populate_default_catalog(&mut self) {
        let catalog = Self::get_default_catalog();
        let catalog_ids: std::collections::HashSet<String> = catalog.iter().map(|m| m.id.clone()).collect();

        let stale_ids: Vec<String> = self.models.iter()
            .filter(|(id, m)| {
                
                m.download_source.as_deref() == Some("ollama")
                
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

    fn get_default_catalog() -> Vec<ModelInfo> {
        vec![]
    }

    pub async fn populate_default_openrouter_models(&mut self) {
        if let Err(e) = self.fetch_public_openrouter_models().await {
            warn!("Failed to fetch public OpenRouter models: {}", e);
            
        }
    }

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
            
            if self.is_invalid_openrouter_model(&plain_id) {
                debug!("Skipping invalid model: {}", plain_id);
                continue;
            }
            
            let registry_id = format!("openrouter:{}", plain_id);

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

            let parameters = {
                let name_str = m.name.as_deref().unwrap_or(&plain_id);
                
                let re = regex::Regex::new(r"(\d+(?:\.\d+)?(?:x\d+)?[BMK])").ok();
                re.and_then(|r| r.find(name_str).map(|m| m.as_str().to_string()))
            };

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

            self.models.insert(registry_id, model_info);
            added += 1;
        }

        info!("Fetched {} public OpenRouter models from API", added);
        Ok(())
    }

    fn detect_shard_pattern_internal(&self, filename: &str) -> Option<u32> {
        
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

    fn collect_shards_internal<'a>(&self, gguf_files: &[&'a HuggingFaceSibling], total_shards: u32) -> Vec<&'a HuggingFaceSibling> {
        let mut shards = Vec::new();
        
        if let Some(first_file) = gguf_files.iter().find(|f| self.detect_shard_pattern_internal(&f.rfilename).is_some()) {
            
            if let Some(caps) = regex::Regex::new(r"(.*-)(\d{5})(-of-\d{5}\.[^.]+)$")
                .ok()
                .and_then(|re| re.captures(&first_file.rfilename)) {
                
                let prefix = caps[1].to_string();  
                let suffix = caps[3].to_string();  
                
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

    fn is_invalid_openrouter_model(&self, model_id: &str) -> bool {
        
        model_id == "google/gemini-pro" || 
        model_id == "google/palm-2-chat-bison" ||
        model_id.starts_with("google/palm") ||
        model_id.starts_with("google/gemini-pro") ||
        
        false
    }
}

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
