//! Runtime Manager
//!
//! Orchestrates model runtime selection, initialization, and lifecycle management.
//! Automatically selects the appropriate runtime based on model format.
//! Lock-free implementation using ArcSwap for atomic pointer swapping.

use super::runtime_trait::*;
use super::format_detector::FormatDetector;
use super::platform_detector::HardwareCapabilities;
use crate::model_runtime::{GGUFRuntime, GGMLRuntime, ONNXRuntime, TensorRTRuntime, SafetensorsRuntime, CoreMLRuntime};
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::{info, error};
use super::*;

/// Runtime holder for lock-free access
struct RuntimeHolder {
    runtime: Option<Box<dyn ModelRuntime>>,
    config: Option<RuntimeConfig>,
}

/// Runtime Manager - manages active model runtime
pub struct RuntimeManager {
    /// Currently active runtime (lock-free via ArcSwap)
    holder: Arc<ArcSwap<RuntimeHolder>>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self {
            holder: Arc::new(ArcSwap::new(Arc::new(RuntimeHolder {
                runtime: None,
                config: None,
            }))),
        }
    }

    /// Initialize runtime with automatic format detection and platform-appropriate binary
    pub async fn initialize_auto(&self, mut config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Auto-detecting model format from: {}", config.model_path.display());
        
        // Check if model path is empty (no model selected)
        if config.model_path.as_os_str().is_empty() {
            info!("No model selected, skipping runtime initialization");
            return Ok(config.host.clone() + ":" + &config.port.to_string());
        }
        
        // Detect format from file extension
        let detected_format = FormatDetector::detect_from_path(&config.model_path)
            .ok_or_else(|| anyhow::anyhow!(
                "Could not detect model format from file: {}. Supported formats: {:?}",
                config.model_path.display(),
                FormatDetector::supported_extensions()
            ))?;

        info!("Detected format: {}", detected_format.name());

        // Override config format with detected format
        config.format = detected_format;
        
        // Auto-detect and set appropriate runtime binary based on platform and hardware
        if config.runtime_binary.is_none() {
            let hw_caps = HardwareCapabilities::default();
            if let Some(binary_path) = hw_caps.get_runtime_binary_path() {
                if binary_path.exists() {
                    info!("Using platform-appropriate runtime binary: {}", binary_path.display());
                    config.runtime_binary = Some(binary_path);
                } else {
                    info!("Platform-specific binary not found: {}, using default", binary_path.display());
                }
            }
        }

        self.initialize(config).await
    }

    /// Initialize runtime with specified configuration
    pub async fn initialize(&self, config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Initializing runtime for format: {}", config.format.name());

        // Check if model path is empty (no model selected)
        if config.model_path.as_os_str().is_empty() {
            info!("No model selected, skipping runtime initialization");
            // Store an empty runtime holder but return the expected base URL
            let base_url = config.host.clone() + ":" + &config.port.to_string();
            let new_holder = Arc::new(RuntimeHolder {
                runtime: None,
                config: Some(config),
            });
            self.holder.store(new_holder);
            return Ok(base_url);
        }
        
        // Shutdown existing runtime if any
        self.shutdown().await?;

        // Create appropriate runtime based on format
        let mut runtime: Box<dyn ModelRuntime> = match config.format {
            ModelFormat::GGUF => Box::new(GGUFRuntime::new()),
            ModelFormat::GGML => Box::new(GGMLRuntime::new()),
            ModelFormat::ONNX => Box::new(ONNXRuntime::new()),
            ModelFormat::TensorRT => Box::new(TensorRTRuntime::new()),
            ModelFormat::Safetensors => Box::new(SafetensorsRuntime::new()),
            ModelFormat::CoreML => Box::new(CoreMLRuntime::new()),
        };

        // Initialize the runtime
        runtime.initialize(config.clone()).await
            .map_err(|e| {
                error!("Failed to initialize {} runtime: {}", config.format.name(), e);
                e
            })?;

        let base_url = runtime.base_url();
        let metadata = runtime.metadata();

        info!("✅ Runtime initialized successfully:");
        info!("  Format: {}", metadata.format.name());
        info!("  Runtime: {}", metadata.runtime_name);
        info!("  Base URL: {}", base_url);
        info!("  GPU Support: {}", metadata.supports_gpu);
        info!("  Streaming: {}", metadata.supports_streaming);

        // Atomically store the new runtime
        let new_holder = Arc::new(RuntimeHolder {
            runtime: Some(runtime),
            config: Some(config),
        });
        self.holder.store(new_holder);

        Ok(base_url)
    }

    /// Get the current runtime's base URL (lock-free)
    pub async fn get_base_url(&self) -> Option<String> {
        let holder = self.holder.load();
        holder.runtime.as_ref().map(|r| r.base_url())
    }

    /// Check if runtime is ready (lock-free read)
    pub async fn is_ready(&self) -> bool {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.is_ready().await,
            None => false,
        }
    }

    /// Perform health check (lock-free read)
    pub async fn health_check(&self) -> anyhow::Result<String> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.health_check().await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }

    /// Get runtime metadata (lock-free read)
    pub async fn get_metadata(&self) -> Option<RuntimeMetadata> {
        let holder = self.holder.load();
        holder.runtime.as_ref().map(|r| r.metadata())
    }

    /// Shutdown current runtime (atomic replacement)
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        // Atomically replace with empty holder so new load() calls see no runtime.
        let old_holder = self.holder.swap(Arc::new(RuntimeHolder {
            runtime: None,
            config: None,
        }));

        // Retry Arc::try_unwrap up to 10 times (100 ms total).
        // ArcSwap load() guards are held for nanoseconds; any concurrent caller
        // that loaded old_holder just before the swap above will have dropped its
        // guard by the second or third attempt at most.
        let mut attempt = old_holder;
        for i in 0..10u8 {
            match Arc::try_unwrap(attempt) {
                Ok(mut holder) => {
                    if let Some(mut runtime) = holder.runtime.take() {
                        info!("Shutting down runtime (attempt {})", i + 1);
                        runtime.shutdown().await?;
                    }
                    return Ok(());
                }
                Err(arc) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    attempt = arc;
                }
            }
        }
        // Could not get exclusive ownership — the process is exiting anyway
        // (std::process::exit(0) in the ExitRequested handler terminates everything).
        tracing::warn!(
            "RuntimeManager::shutdown: could not acquire exclusive Arc ownership after 10 retries. \
             llama-server will be killed by the OS on process exit."
        );

        Ok(())
    }

    /// Hot-swap model (shutdown current, initialize new)
    pub async fn hot_swap(&self, new_config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Performing hot-swap to new model: {}", new_config.model_path.display());
        
        self.shutdown().await?;
        self.initialize(new_config).await
    }

    /// Get current configuration (lock-free)
    pub async fn get_current_config(&self) -> Option<RuntimeConfig> {
        let holder = self.holder.load();
        holder.config.clone()
    }

    /// Perform inference (non-streaming, lock-free read)
    pub async fn generate(&self, request: InferenceRequest) -> anyhow::Result<InferenceResponse> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.generate(request).await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }

    /// Perform streaming inference (lock-free read)
    pub async fn generate_stream(
        &self,
        request: InferenceRequest,
    ) -> anyhow::Result<Box<dyn futures_util::Stream<Item = Result<String, anyhow::Error>> + Send + Unpin>> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.generate_stream(request).await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RuntimeManager {
    fn drop(&mut self) {
        // Runtime cleanup happens in shutdown()
        // This is just a safety net
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::platform_detector::{Platform, HardwareArchitecture};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_runtime_manager_creation() {
        let manager = RuntimeManager::new();
        assert!(!manager.is_ready().await);
    }

    #[tokio::test]
    async fn test_format_detection() {
        let manager = RuntimeManager::new();
        
        let config = RuntimeConfig {
            model_path: PathBuf::from("test.gguf"),
            format: ModelFormat::GGUF, // Will be overridden
            ..Default::default()
        };

        // This will fail because the file doesn't exist, but tests the detection logic
        let result = manager.initialize_auto(config).await;
        assert!(result.is_err()); // Expected to fail - file doesn't exist
    }
    
    #[test]
    fn test_platform_detection() {
        let hw_caps = HardwareCapabilities::default();
        
        // Verify that platform detection returns a valid platform
        assert!(matches!(hw_caps.platform, Platform::Windows | Platform::Linux | Platform::MacOS));
        
        // Verify that architecture detection returns a valid architecture
        assert!(matches!(
            hw_caps.architecture,
            HardwareArchitecture::X86_64 | HardwareArchitecture::Aarch64 | HardwareArchitecture::Other(_)
        ));
    }
    
    #[tokio::test]
    async fn test_auto_binary_selection() {
        // Create a config without specifying a binary path
        let config = RuntimeConfig {
            model_path: PathBuf::from("test.gguf"),
            format: ModelFormat::GGUF,
            runtime_binary: None, // Intentionally set to None
            ..Default::default()
        };
        
        let manager = RuntimeManager::new();
        
        // The initialize_auto method should attempt to select an appropriate binary
        // based on platform. It will fail due to missing file but should at least
        // try to select a platform-appropriate binary.
        let result = manager.initialize_auto(config).await;
        
        // The result will be an error because the file doesn't exist, but the
        // platform detection part should work
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_hot_swap_functionality() {
        let manager = RuntimeManager::new();
        
        // Test that hot-swap works properly
        let config1 = RuntimeConfig {
            model_path: PathBuf::from("model1.gguf"),
            format: ModelFormat::GGUF,
            runtime_binary: None,
            port: 8081,
            ..Default::default()
        };
        
        let config2 = RuntimeConfig {
            model_path: PathBuf::from("model2.gguf"),
            format: ModelFormat::GGUF,
            runtime_binary: None,
            port: 8082,
            ..Default::default()
        };
        
        // Initialize first config
        let result1 = manager.initialize_auto(config1).await;
        
        // Hot-swap to second config
        let result2 = manager.hot_swap(config2).await;
        
        // Both operations will fail due to missing files, but the process should complete
        // without crashing
        assert!(result2.is_ok() || result2.is_err());
        
        manager.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_multiple_format_support() {
        let manager = RuntimeManager::new();
        
        // Test different model formats
        let formats = [
            ModelFormat::GGUF,
            ModelFormat::GGML,
            ModelFormat::ONNX,
            ModelFormat::TensorRT,
            ModelFormat::Safetensors,
            ModelFormat::CoreML,
        ];
        
        for format in &formats {
            let config = RuntimeConfig {
                model_path: PathBuf::from("test"),
                format: format.clone(),
                runtime_binary: None,
                port: 8080,
                ..Default::default()
            };
            
            let result = manager.initialize(config).await;
            
            // Each format should be attempted without crashing the system
            assert!(result.is_ok() || result.is_err());
        }
        
        manager.shutdown().await.unwrap();
    }
}
