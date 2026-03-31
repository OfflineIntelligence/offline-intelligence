
pub mod registry;
pub mod downloader;
pub mod analyzer;
pub mod download_progress;

pub use registry::{EngineRegistry, EngineInfo, EngineStatus, AccelerationType};
pub use downloader::{EngineDownloader, EngineSource};
pub use analyzer::{HardwareAnalyzer, HardwareProfile};
pub use download_progress::{EngineDownloadProgressTracker, EngineDownloadProgress, EngineDownloadStatus};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::model_runtime::platform_detector::HardwareCapabilities;

pub struct EngineManager {
    pub registry: Arc<RwLock<EngineRegistry>>,
    pub downloader: Arc<EngineDownloader>,
    pub analyzer: Arc<HardwareAnalyzer>,
    pub hardware_capabilities: HardwareCapabilities,
}

impl EngineManager {
    pub fn new() -> Result<Self> {
        let hardware_capabilities = HardwareCapabilities::detect();
        let analyzer = Arc::new(HardwareAnalyzer::new(hardware_capabilities.clone()));
        let registry = Arc::new(RwLock::new(EngineRegistry::new()?));
        let downloader = Arc::new(EngineDownloader::new());

        Ok(Self {
            registry,
            downloader,
            analyzer,
            hardware_capabilities,
        })
    }

    pub async fn initialize(&self, _cfg: &Config) -> Result<bool> {
        
        {
            let mut registry = self.registry.write().await;
            registry.scan_installed_engines(&self.hardware_capabilities).await?;
        }

        let installed_engines_count = self.registry.read().await.installed_engines.len();

        if installed_engines_count == 0 {
            tracing::info!("First run detected - no engines installed, automatically downloading most compatible engine");

            match tokio::time::timeout(
                std::time::Duration::from_secs(600),
                self.download_suitable_engine()
            ).await {
                Ok(Ok(engine)) => {
                    tracing::info!("✅ Engine downloaded successfully on first run: {}", engine.name);
                    
                    let mut reg = self.registry.write().await;
                    if let Some(first_engine_id) = reg.installed_engines.keys().next().cloned() {
                        reg.set_default_engine(&first_engine_id)?;
                    }
                    return Ok(true);
                }
                Ok(Err(e)) => {
                    tracing::warn!("⚠️ Engine download failed: {}. App will continue but models won't work until engine is downloaded.", e);
                    return Ok(false); 
                }
                Err(_) => {
                    tracing::warn!("⚠️ Engine download timed out after 600 seconds (10 minutes). App will continue with background retry.");
                    return Ok(false);
                }
            }
        } else {
            
            let has_suitable_engine = self.check_suitable_engine().await?;

            if !has_suitable_engine {
                tracing::info!("No suitable engine found for current hardware configuration");
                tracing::info!("Attempting to download most compatible engine");

                match self.download_suitable_engine().await {
                    Ok(engine) => {
                        tracing::info!("Automatically installed engine: {}", engine.name);
                        
                        self.registry.write().await.set_default_engine(&engine.id)?;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to automatically install suitable engine: {}", e);
                        
                    }
                }
            } else {
                
                self.select_best_engine().await?;
            }
        }

        Ok(true)
    }

    pub async fn check_suitable_engine(&self) -> Result<bool> {
        let registry = self.registry.read().await;
        let suitable_engines = registry.get_compatible_engines(&self.hardware_capabilities);
        Ok(!suitable_engines.is_empty())
    }

    pub async fn select_best_engine(&self) -> Result<Option<EngineInfo>> {
        let mut registry = self.registry.write().await;
        let best_engine = registry.select_best_compatible_engine(&self.hardware_capabilities);

        if let Some(engine) = &best_engine {
            registry.set_default_engine(&engine.id)?;
            tracing::info!("Selected engine: {} for hardware: {:?}",
                engine.name, self.hardware_capabilities);
        }

        Ok(best_engine)
    }

    pub async fn download_suitable_engine(&self) -> Result<EngineInfo> {
        
        let can_start = {
            let registry = self.registry.read().await;
            registry.mark_download_started()
        };

        if !can_start {
            return Err(anyhow::anyhow!("Another engine download is already in progress"));
        }

        let recommended_engine = {
            let registry = self.registry.read().await;
            registry.get_recommended_engine(&self.hardware_capabilities)
                .ok_or_else(|| anyhow::anyhow!("No recommended engine found for current hardware"))?
        };

        tracing::info!("Downloading recommended engine: {}", recommended_engine.name);

        {
            let mut registry = self.registry.write().await;
            let mut engine_to_download = recommended_engine.clone();
            engine_to_download.status = EngineStatus::Downloading;
            registry.add_installed_engine(engine_to_download).await?;
        }

        let download_result = self.downloader.download_engine(&recommended_engine).await;

        {
            let registry = self.registry.read().await;
            registry.mark_download_finished();
        }

        let engine = download_result?;
        self.registry.write().await.add_installed_engine(engine.clone()).await?;

        Ok(engine)
    }

    pub async fn ensure_engine_available(&self) -> Result<bool> {
        let registry = self.registry.read().await;

        if registry.has_installed_engine() {
            return Ok(true);
        }

        drop(registry); 

        tracing::info!("No engine available, downloading suitable engine...");
        match self.download_suitable_engine().await {
            Ok(_) => {
                tracing::info!("Engine downloaded successfully");
                Ok(true)
            }
            Err(e) => {
                tracing::error!("Failed to download engine: {}", e);
                Ok(false)
            }
        }
    }

    pub fn get_hardware_info(&self) -> &HardwareCapabilities {
        &self.hardware_capabilities
    }

    pub async fn refresh_available_engines(&self) -> Result<()> {
        let mut registry = self.registry.write().await;
        registry.refresh_available_engines(&self.hardware_capabilities).await?;
        Ok(())
    }

    pub async fn get_status_info(&self) -> String {
        let registry = self.registry.read().await;
        let installed_count = registry.installed_engines.len();
        let available_count = registry.available_engines.len();
        let default_engine = registry.default_engine.as_deref().unwrap_or("None");

        format!(
            "Engine Manager Status:\n  Installed Engines: {}\n  Available Engines: {}\n  Default Engine: {}\n  Hardware: {:?} {:?} (CUDA: {})\n  Recommended Engine: {:?}",
            installed_count,
            available_count,
            default_engine,
            self.hardware_capabilities.platform,
            self.hardware_capabilities.architecture,
            self.hardware_capabilities.has_cuda,
            registry.get_recommended_engine(&self.hardware_capabilities).map(|e| e.name)
        )
    }
    
    pub async fn install_engine_by_id(&self, engine_id: &str) -> Result<EngineInfo> {
        let registry = self.registry.read().await;
        let engine_to_install = registry.available_engines.iter()
            .find(|engine| engine.id == engine_id)
            .cloned();

        match engine_to_install {
            Some(engine_info) => {
                drop(registry); 
                let installed_engine = self.downloader.download_engine(&engine_info).await?;
                self.registry.write().await.add_installed_engine(installed_engine.clone()).await?;
                Ok(installed_engine)
            }
            None => {
                Err(anyhow::anyhow!("Engine not found: {}", engine_id))
            }
        }
    }
}
