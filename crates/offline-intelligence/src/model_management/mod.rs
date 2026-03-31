
pub mod registry;
pub mod downloader;
pub mod storage;
pub mod recommendation;
pub mod progress;
pub mod hf_access;

pub use registry::{ModelRegistry, ModelInfo, ModelPricing, ModelStatus};
pub use downloader::{ModelDownloader, DownloadSource};
pub use storage::{ModelStorage, StorageLocation};
pub use recommendation::ModelRecommender;
pub use progress::{DownloadProgress, ProgressTracker};
pub use hf_access::{check_hf_gated_access, HfAccessStatus};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;

pub struct ModelManager {
    pub registry: Arc<RwLock<ModelRegistry>>,
    pub downloader: Arc<ModelDownloader>,
    pub storage: Arc<ModelStorage>,
    pub recommender: Arc<ModelRecommender>,
}

impl ModelManager {
    pub fn new() -> Result<Self> {
        let storage = Arc::new(ModelStorage::new()?);
        let registry = Arc::new(RwLock::new(ModelRegistry::new(storage.clone())?));
        let downloader = Arc::new(ModelDownloader::new(storage.clone()));
        let recommender = Arc::new(ModelRecommender::new());

        Ok(Self {
            registry,
            downloader,
            storage,
            recommender,
        })
    }

    pub async fn initialize(&self, cfg: &Config) -> Result<()> {
        
        self.registry.write().await.scan_storage().await?;

        self.refresh_catalogs(cfg).await?;

        let hardware = ModelRecommender::detect_hardware_profile(cfg);
        {
            let mut registry = self.registry.write().await;
            let recommender = &*self.recommender;
            registry.update_compatibility_scores(recommender, &hardware);
            
            let _ = registry.save_registry().await;
        }

        Ok(())
    }

    pub async fn refresh_catalogs(&self, cfg: &Config) -> Result<()> {
        let env_key = std::env::var("OPENROUTER_API_KEY").ok();
        let api_key = if !cfg.openrouter_api_key.is_empty() && !cfg.openrouter_api_key.trim().is_empty() {
            Some(cfg.openrouter_api_key.as_str())
        } else if let Some(ref env_value) = env_key {
            Some(env_value.as_str())
        } else {
            None
        };

        let mut registry = self.registry.write().await;
        if let Some(key) = api_key {
            if let Err(e) = registry.refresh_openrouter_catalog_from_api(key).await {
                tracing::warn!("Failed to refresh OpenRouter catalog: {}", e);
                
            }
        } else {
            
            registry.populate_default_openrouter_models().await;
        }
        
        if let Err(e) = registry.save_registry().await {
            tracing::error!("Failed to save model registry after OpenRouter refresh: {}", e);
        }

        if let Err(e) = registry.refresh_huggingface_catalog_from_api(500).await {
            tracing::warn!("Failed to refresh Hugging Face catalog: {}", e);
        }
        if let Err(e) = registry.save_registry().await {
            tracing::error!("Failed to save model registry after HuggingFace refresh: {}", e);
        }

        Ok(())
    }
}
