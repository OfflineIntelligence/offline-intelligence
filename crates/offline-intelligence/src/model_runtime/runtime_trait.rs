use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelFormat {
    /
    GGUF,
    /
    GGML,
    /
    ONNX,
    /
    TensorRT,
    /
    Safetensors,
    /
    CoreML,
}
impl ModelFormat {
    /
    pub fn extensions(&self) -> &[&str] {
        match self {
            ModelFormat::GGUF => &["gguf"],
            ModelFormat::GGML => &["ggml", "bin"],
            ModelFormat::ONNX => &["onnx"],
            ModelFormat::TensorRT => &["trt", "engine", "plan"],
            ModelFormat::Safetensors => &["safetensors"],
            ModelFormat::CoreML => &["mlmodel", "mlpackage"],
        }
    }
    /
    pub fn name(&self) -> &str {
        match self {
            ModelFormat::GGUF => "GGUF (llama.cpp)",
            ModelFormat::GGML => "GGML (llama.cpp legacy)",
            ModelFormat::ONNX => "ONNX Runtime",
            ModelFormat::TensorRT => "TensorRT",
            ModelFormat::Safetensors => "Safetensors",
            ModelFormat::CoreML => "CoreML",
        }
    }
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /
    pub model_path: PathBuf,
    /
    pub format: ModelFormat,
    /
    pub host: String,
    /
    pub port: u16,
    /
    pub context_size: u32,
    /
    pub batch_size: u32,
    /
    pub threads: u32,
    /
    pub gpu_layers: u32,
    /
    pub runtime_binary: Option<PathBuf>,
    /
    pub extra_config: serde_json::Value,
}
impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            format: ModelFormat::GGUF,
            host: "127.0.0.1".to_string(),
            port: 8001,
            context_size: 8192,
            batch_size: 128,
            threads: 6,
            gpu_layers: 0,
            runtime_binary: None,
            extra_config: serde_json::json!({}),
        }
    }
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_stream")]
    pub stream: bool,
}
fn default_max_tokens() -> u32 { 2000 }
fn default_temperature() -> f32 { 0.7 }
fn default_stream() -> bool { false }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub content: String,
    pub finish_reason: Option<String>,
}
/
#[async_trait]
pub trait ModelRuntime: Send + Sync {
    /
    fn supported_format(&self) -> ModelFormat;
    /
    async fn initialize(&mut self, config: RuntimeConfig) -> anyhow::Result<()>;
    /
    async fn is_ready(&self) -> bool;
    /
    async fn health_check(&self) -> anyhow::Result<String>;
    /
    fn base_url(&self) -> String;
    /
    fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url())
    }
    /
    async fn generate(
        &self,
        request: InferenceRequest,
    ) -> anyhow::Result<InferenceResponse>;
    /
    async fn generate_stream(
        &self,
        request: InferenceRequest,
    ) -> anyhow::Result<Box<dyn futures_util::Stream<Item = Result<String, anyhow::Error>> + Send + Unpin>>;
    /
    async fn shutdown(&mut self) -> anyhow::Result<()>;
    /
    fn metadata(&self) -> RuntimeMetadata;
}
/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetadata {
    pub format: ModelFormat,
    pub runtime_name: String,
    pub version: String,
    pub supports_gpu: bool,
    pub supports_streaming: bool,
}


