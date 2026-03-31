
pub mod runtime_trait;
pub mod gguf_runtime;
pub mod onnx_runtime;
pub mod tensorrt_runtime;
pub mod safetensors_runtime;
pub mod ggml_runtime;
pub mod coreml_runtime;
pub mod format_detector;
pub mod platform_detector;
pub mod runtime_manager;

pub use runtime_trait::{ModelRuntime, ModelFormat, RuntimeConfig, InferenceRequest, InferenceResponse};
pub use gguf_runtime::GGUFRuntime;
pub use onnx_runtime::ONNXRuntime;
pub use tensorrt_runtime::TensorRTRuntime;
pub use safetensors_runtime::SafetensorsRuntime;
pub use ggml_runtime::GGMLRuntime;
pub use coreml_runtime::CoreMLRuntime;
pub use format_detector::FormatDetector;
pub use platform_detector::HardwareCapabilities;
pub use runtime_manager::RuntimeManager;
