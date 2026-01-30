//!
//! Orchestrates model runtime selection, initialization, and lifecycle management.
//! Automatically selects the appropriate runtime based on model format.
//! Lock-free implementation using ArcSwap for atomic pointer swapping.
use super::runtime_trait::*;
use super::format_detector::FormatDetector;
use super::*;
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::{info, error};
/
struct RuntimeHolder {
    runtime: Option<Box<dyn ModelRuntime>>,
    config: Option<RuntimeConfig>,
}
/
pub struct RuntimeManager {
    /
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
    /
    pub async fn initialize_auto(&self, config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Auto-detecting model format from: {}", config.model_path.display());


        let detected_format = FormatDetector::detect_from_path(&config.model_path)
            .ok_or_else(|| anyhow::anyhow!(
                "Could not detect model format from file: {}. Supported formats: {:?}",
                config.model_path.display(),
                FormatDetector::supported_extensions()
            ))?;
        info!("Detected format: {}", detected_format.name());

        let mut final_config = config;
        final_config.format = detected_format;
        self.initialize(final_config).await
    }
    /
    pub async fn initialize(&self, config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Initializing runtime for format: {}", config.format.name());

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
        info!("âœ… Runtime initialized successfully:");
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
    /
    pub async fn get_base_url(&self) -> Option<String> {
        let holder = self.holder.load();
        holder.runtime.as_ref().map(|r| r.base_url())
    }
    /
    pub async fn is_ready(&self) -> bool {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.is_ready().await,
            None => false,
        }
    }
    /
    pub async fn health_check(&self) -> anyhow::Result<String> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.health_check().await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }
    /
    pub async fn get_metadata(&self) -> Option<RuntimeMetadata> {
        let holder = self.holder.load();
        holder.runtime.as_ref().map(|r| r.metadata())
    }
    /
    pub async fn shutdown(&self) -> anyhow::Result<()> {

        let old_holder = self.holder.swap(Arc::new(RuntimeHolder {
            runtime: None,
            config: None,
        }));


        if let Ok(mut holder) = Arc::try_unwrap(old_holder) {
            if let Some(mut runtime) = holder.runtime.take() {
                info!("Shutting down runtime");
                runtime.shutdown().await?;
            }
        }
        Ok(())
    }
    /
    pub async fn hot_swap(&self, new_config: RuntimeConfig) -> anyhow::Result<String> {
        info!("Performing hot-swap to new model: {}", new_config.model_path.display());

        self.shutdown().await?;
        self.initialize(new_config).await
    }
    /
    pub async fn get_current_config(&self) -> Option<RuntimeConfig> {
        let holder = self.holder.load();
        holder.config.clone()
    }
    /
    pub async fn generate(&self, request: InferenceRequest) -> anyhow::Result<InferenceResponse> {
        let holder = self.holder.load();
        match holder.runtime.as_ref() {
            Some(r) => r.generate(request).await,
            None => Err(anyhow::anyhow!("No runtime initialized")),
        }
    }
    /
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
}


