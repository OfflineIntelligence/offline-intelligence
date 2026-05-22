//! Engine Management System
//!
//! Provides comprehensive llama.cpp engine lifecycle management including:
//! - Hardware capability detection and analysis
//! - Engine registry and metadata storage
//! - Download from official llama.cpp releases
//! - Local engine storage management
//! - Automatic engine selection based on hardware
//! - Cross-platform compatibility (Windows, macOS, Linux)

pub mod registry;
pub mod downloader;
pub mod analyzer;
pub mod download_progress;
pub mod runtime_deps;

pub use registry::{EngineRegistry, EngineInfo, EngineStatus, AccelerationType};
pub use downloader::{EngineDownloader, EngineSource};
pub use analyzer::{HardwareAnalyzer, HardwareProfile};
pub use download_progress::{EngineDownloadProgressTracker, EngineDownloadProgress, EngineDownloadStatus};
pub use runtime_deps::{RuntimeDepsManager, DepSummary, DepStatus, RuntimeDepKind};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::model_runtime::platform_detector::HardwareCapabilities;

/// Main engine management service
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

    /// Initialize the engine manager and ensure a suitable engine is available
    pub async fn initialize(&self, _cfg: &Config) -> Result<bool> {
        // Scan for existing engines and populate registry
        // This also refreshes available engines from official sources
        {
            let mut registry = self.registry.write().await;
            registry.scan_installed_engines(&self.hardware_capabilities).await?;
        }

        // Check if we have any installed engines at all (first run detection)
        let installed_engines_count = self.registry.read().await.installed_engines.len();

        if installed_engines_count == 0 {
            tracing::info!("First run detected - no engines installed, automatically downloading most compatible engine");

            // Block startup for up to 600 seconds (10 minutes) waiting for engine download
            // This ensures first-run experience completes engine setup before allowing app usage
            match tokio::time::timeout(
                std::time::Duration::from_secs(600),
                self.download_suitable_engine()
            ).await {
                Ok(Ok(engine)) => {
                    tracing::info!("✅ Engine downloaded successfully on first run: {}", engine.name);
                    // Set newly downloaded engine as default
                    let mut reg = self.registry.write().await;
                    if let Some(first_engine_id) = reg.installed_engines.keys().next().cloned() {
                        reg.set_default_engine(&first_engine_id)?;
                    }
                    return Ok(true);
                }
                Ok(Err(e)) => {
                    tracing::error!("❌ Engine download failed: {}. Local inference is unavailable until an engine is installed.", e);
                    return Ok(false);
                }
                Err(_) => {
                    tracing::error!("❌ Engine download timed out after 600 seconds. Local inference is unavailable until an engine is installed.");
                    return Ok(false);
                }
            }
        } else {
            // Check if we have a suitable installed engine for current hardware
            let has_suitable_engine = self.check_suitable_engine().await?;

            if !has_suitable_engine {
                tracing::info!("No suitable engine found for current hardware configuration");
                tracing::info!("Attempting to download most compatible engine");

                match self.download_suitable_engine().await {
                    Ok(engine) => {
                        tracing::info!("Automatically installed engine: {}", engine.name);
                        // Set the newly installed engine as default
                        self.registry.write().await.set_default_engine(&engine.id)?;
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to automatically install suitable engine: {}. Local inference is unavailable until an engine is installed.", e);
                    }
                }
            } else {
                // Set the best available engine as default
                self.select_best_engine().await?;
            }
        }

        Ok(true)
    }

    /// Check if we have an engine suitable for current hardware
    pub async fn check_suitable_engine(&self) -> Result<bool> {
        let registry = self.registry.read().await;
        let suitable_engines = registry.get_compatible_engines(&self.hardware_capabilities);
        Ok(!suitable_engines.is_empty())
    }

    /// Select and set the best engine for current hardware as default
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

    /// Download and install a suitable engine for current hardware
    pub async fn download_suitable_engine(&self) -> Result<EngineInfo> {
        // Check if another download is already in progress
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

        // Update registry to show this engine is being downloaded
        {
            let mut registry = self.registry.write().await;
            let mut engine_to_download = recommended_engine.clone();
            engine_to_download.status = EngineStatus::Downloading;
            registry.add_installed_engine(engine_to_download).await?;
        }

        // Perform the download
        let download_result = self.downloader.download_engine(&recommended_engine).await;

        // Mark download as finished regardless of success/failure
        {
            let registry = self.registry.read().await;
            registry.mark_download_finished();
        }

        // Now handle the download result
        let engine = download_result?;
        self.registry.write().await.add_installed_engine(engine.clone()).await?;

        Ok(engine)
    }

    /// Ensures an engine is available, downloading if necessary.
    /// Returns `Ok(true)` if a working engine is already installed.
    /// Returns `Ok(true)` after a successful download.
    /// Returns `Err` if download fails — caller must handle this as a hard failure.
    pub async fn ensure_engine_available(&self) -> Result<bool> {
        let registry = self.registry.read().await;

        if registry.has_installed_engine() {
            return Ok(true);
        }

        drop(registry);

        tracing::info!("No engine found — downloading suitable engine...");
        match self.download_suitable_engine().await {
            Ok(_) => {
                tracing::info!("✅ Engine downloaded successfully");
                Ok(true)
            }
            Err(e) => {
                tracing::error!("❌ Engine download failed: {}. Local inference cannot proceed.", e);
                Err(e)
            }
        }
    }

    /// Get information about current hardware capabilities
    pub fn get_hardware_info(&self) -> &HardwareCapabilities {
        &self.hardware_capabilities
    }

    /// Refresh available engines from remote sources
    pub async fn refresh_available_engines(&self) -> Result<()> {
        let mut registry = self.registry.write().await;
        registry.refresh_available_engines(&self.hardware_capabilities).await?;
        Ok(())
    }

    /// Get detailed status information about the engine manager
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
    
    /// Install a specific engine by ID
    pub async fn install_engine_by_id(&self, engine_id: &str) -> Result<EngineInfo> {
        let registry = self.registry.read().await;
        let engine_to_install = registry.available_engines.iter()
            .find(|engine| engine.id == engine_id)
            .cloned();

        match engine_to_install {
            Some(engine_info) => {
                drop(registry); // Release read lock before downloading
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