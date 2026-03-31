
use super::runtime_trait::*;
use super::format_detector::FormatDetector;
use super::platform_detector::HardwareCapabilities;
use crate::model_runtime::{GGUFRuntime, GGMLRuntime, ONNXRuntime, TensorRTRuntime, SafetensorsRuntime, CoreMLRuntime};
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::{info, error};
use super::*;

struct RuntimeHolder {
    runtime: Option<Box<dyn ModelRuntime>>,
    config: Option<RuntimeConfig>,
}

pub struct RuntimeManager {
    
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

    pub async fn initialize_auto(&self, mut config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Auto-detecting model format from: {}", config.model_path.display());
        
        if config.model_path.as_os_str().is_empty() {
            info!("No model selected, skipping runtime initialization");
            return Ok(config.host.clone() + ":" + &config.port.to_string());
        }
        
        let detected_format = FormatDetector::detect_from_path(&config.model_path)
            .ok_or_else(|| anyhow::anyhow!(
                "Could not detect model format from file: {}. Supported formats: {:?}",
                config.model_path.display(),
                FormatDetector::supported_extensions()
            ))?;

        info!("Detected format: {}", detected_format.name());

        config.format = detected_format;
        
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

    pub async fn initialize(&self, config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Initializing runtime for format: {}", config.format.name());

        if config.model_path.as_os_str().is_empty() {
            info!("No model selected, skipping runtime initialization");
            
            let base_url = config.host.clone() + ":" + &config.port.to_string();
            let new_holder = Arc::new(RuntimeHolder {
                runtime: None,
                config: Some(config),
            });
            self.holder.store(new_holder);
            return Ok(base_url);
        }
        
        self.shutdown().await?;

        let mut runtime: Box<dyn ModelRuntime> = match config.format {
            ModelFormat::GGUF => Box::new(GGUFRuntime::new()),
            ModelFormat::GGML => Box::new(GGMLRuntime::new()),
            ModelFormat::ONNX => Box::new(ONNXRuntime::new()),
            ModelFormat::TensorRT => Box::new(TensorRTRuntime::new()),
            ModelFormat::Safetensors => Box::new(SafetensorsRuntime::new()),
            ModelFormat::CoreML => Box::new(CoreMLRuntime::new()),
        };

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

        let new_holder = Arc::new(RuntimeHolder {
            runtime: Some(runtime),
            config: Some(config),
        });
        self.holder.store(new_holder);

        Ok(base_url)
    }

    pub async fn get_base_url(&self) -> Option<String> {
        let holder = self.holder.load();
        holder.runtime.as_ref().map(|r| r.base_url())
    }

    pub async fn is_ready(&self) -> bool {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.is_ready().await,
            None => false,
        }
    }

    pub async fn health_check(&self) -> anyhow::Result<String> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.health_check().await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }

    pub async fn get_metadata(&self) -> Option<RuntimeMetadata> {
        let holder = self.holder.load();
        holder.runtime.as_ref().map(|r| r.metadata())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        
        let old_holder = self.holder.swap(Arc::new(RuntimeHolder {
            runtime: None,
            config: None,
        }));

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
        
        tracing::warn!(
            "RuntimeManager::shutdown: could not acquire exclusive Arc ownership after 10 retries. \
             llama-server will be killed by the OS on process exit."
        );

        Ok(())
    }

    pub async fn hot_swap(&self, new_config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Performing hot-swap to new model: {}", new_config.model_path.display());
        
        self.shutdown().await?;
        self.initialize(new_config).await
    }

    pub async fn get_current_config(&self) -> Option<RuntimeConfig> {
        let holder = self.holder.load();
        holder.config.clone()
    }

    pub async fn generate(&self, request: InferenceRequest) -> anyhow::Result<InferenceResponse> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.generate(request).await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }

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
            format: ModelFormat::GGUF, 
            ..Default::default()
        };

        let result = manager.initialize_auto(config).await;
        assert!(result.is_err()); 
    }
    
    #[test]
    fn test_platform_detection() {
        let hw_caps = HardwareCapabilities::default();
        
        assert!(matches!(hw_caps.platform, Platform::Windows | Platform::Linux | Platform::MacOS));
        
        assert!(matches!(
            hw_caps.architecture,
            HardwareArchitecture::X86_64 | HardwareArchitecture::Aarch64 | HardwareArchitecture::Other(_)
        ));
    }
    
    #[tokio::test]
    async fn test_auto_binary_selection() {
        
        let config = RuntimeConfig {
            model_path: PathBuf::from("test.gguf"),
            format: ModelFormat::GGUF,
            runtime_binary: None, 
            ..Default::default()
        };
        
        let manager = RuntimeManager::new();
        
        let result = manager.initialize_auto(config).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_hot_swap_functionality() {
        let manager = RuntimeManager::new();
        
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
        
        let result1 = manager.initialize_auto(config1).await;
        
        let result2 = manager.hot_swap(config2).await;
        
        assert!(result2.is_ok() || result2.is_err());
        
        manager.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_multiple_format_support() {
        let manager = RuntimeManager::new();
        
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
            
            assert!(result.is_ok() || result.is_err());
        }
        
        manager.shutdown().await.unwrap();
    }
}
