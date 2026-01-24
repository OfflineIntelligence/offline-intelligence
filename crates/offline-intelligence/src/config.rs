// _Aud.io/crates/offline-intelligence/src/config.rs

use anyhow::{Context, Result};
use std::env;
use std::net::SocketAddr;
use tracing::{info, warn};
use nvml_wrapper::Nvml;
use sysinfo::System;
// use crate::resources::ResourceManager;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Config {
    pub model_path: String,
    pub llama_bin: String,
    pub llama_host: String,
    pub llama_port: u16,
    pub ctx_size: u32,
    pub batch_size: u32,
    pub threads: u32,
    pub gpu_layers: u32,
    pub health_timeout_seconds: u64,
    pub hot_swap_grace_seconds: u64,
    pub max_concurrent_streams: u32,
    pub prometheus_port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub requests_per_second: u32,
    pub generate_timeout_seconds: u64,
    pub stream_timeout_seconds: u64,
    pub health_check_timeout_seconds: u64,
    pub queue_size: usize,
    pub queue_timeout_seconds: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        if let Err(e) = dotenvy::dotenv() {
            warn!("Failed to load .env file: {}. Using system environment variables.", e);
        } else {
            info!("Loaded environment variables from .env file");
        }

        // Use LLAMA_BIN directly from environment variable
        let llama_bin = env::var("LLAMA_BIN")
            .context("LLAMA_BIN environment variable not set. Please set it in your .env file")?;
        
        // Verify the binary exists
        if !std::path::Path::new(&llama_bin).exists() {
            return Err(anyhow::anyhow!(
                "Llama binary not found at: {}. Please check LLAMA_BIN in .env file.",
                llama_bin
            ));
        }
        
        info!("Using llama binary from .env: {}", llama_bin);

        // Use MODEL_PATH from env, or try to find embedded model
        let model_path = Self::get_model_path_with_fallback()?;

        // Auto‑detect threads if set to "auto"
        let threads = if env::var("THREADS").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_threads()
        } else {
            env::var("THREADS").unwrap_or_else(|_| "6".into()).parse().unwrap_or(6)
        };

        // Auto‑detect GPU layers if set to "auto"
        let gpu_layers = if env::var("GPU_LAYERS").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_gpu_layers()
        } else {
            env::var("GPU_LAYERS").unwrap_or_else(|_| "20".into()).parse().unwrap_or(20)
        };

        // Auto‑detect context size
        let ctx_size = if env::var("CTX_SIZE").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_ctx_size(&model_path)
        } else {
            env::var("CTX_SIZE").unwrap_or_else(|_| "8192".into()).parse().unwrap_or(8192)
        };

        // Auto‑detect batch size
        let batch_size = if env::var("BATCH_SIZE").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_batch_size(gpu_layers, ctx_size)
        } else {
            env::var("BATCH_SIZE").unwrap_or_else(|_| "256".into()).parse().unwrap_or(256)
        };

        // Get LLM backend configuration
        let llama_host = env::var("LLAMA_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let llama_port = env::var("LLAMA_PORT").unwrap_or_else(|_| "8081".into()).parse()?;

        info!(
            "Resource Configuration: {} GPU layers, {} threads, batch size: {}, context: {}",
            gpu_layers, threads, batch_size, ctx_size
        );

        Ok(Self {
            model_path,
            llama_bin,
            llama_host: llama_host.clone(),
            llama_port,
            ctx_size,
            batch_size,
            threads,
            gpu_layers,
            health_timeout_seconds: env::var("HEALTH_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "60".into())
                .parse()?,
            hot_swap_grace_seconds: env::var("HOT_SWAP_GRACE_SECONDS")
                .unwrap_or_else(|_| "25".into())
                .parse()?,
            max_concurrent_streams: env::var("MAX_CONCURRENT_STREAMS")
                .unwrap_or_else(|_| "4".into())
                .parse()?,
            prometheus_port: env::var("PROMETHEUS_PORT")
                .unwrap_or_else(|_| "9000".into())
                .parse()?,
            api_host: env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            api_port: env::var("API_PORT").unwrap_or_else(|_| "8000".into()).parse()?,
            requests_per_second: env::var("REQUESTS_PER_SECOND")
                .unwrap_or_else(|_| "24".into())
                .parse()?,
            generate_timeout_seconds: env::var("GENERATE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "300".into())
                .parse()?,
            stream_timeout_seconds: env::var("STREAM_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "600".into())
                .parse()?,
            health_check_timeout_seconds: env::var("HEALTH_CHECK_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "90".into())
                .parse()?,
            queue_size: env::var("QUEUE_SIZE")
                .unwrap_or_else(|_| "100".into())
                .parse()?,
            queue_timeout_seconds: env::var("QUEUE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".into())
                .parse()?,
        })
    }

    fn get_model_path_with_fallback() -> Result<String> {
        // First try environment variable
        if let Ok(model_path) = env::var("MODEL_PATH") {
            // Check if the path exists
            if std::path::Path::new(&model_path).exists() {
                info!("Using model from MODEL_PATH: {}", model_path);
                return Ok(model_path);
            } else {
                warn!("MODEL_PATH set but file doesn't exist: {}", model_path);
            }
        }

        // Try to find embedded model
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Check multiple possible embedded model locations
        let possible_model_locations = vec![
            exe_dir.join("resources/models/default.gguf"),
            exe_dir.join("resources/models/model.gguf"),
            exe_dir.join("models/default.gguf"),
            exe_dir.join("models/model.gguf"),
            exe_dir.join("default.gguf"),
        ];

        for model_path in possible_model_locations {
            if model_path.exists() {
                info!("Using embedded model: {}", model_path.display());
                return Ok(model_path.to_string_lossy().to_string());
            }
        }

        // Check for any .gguf file in models directory
        if let Ok(entries) = std::fs::read_dir(exe_dir.join("resources/models")) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "gguf" {
                        info!("Using found model: {}", entry.path().display());
                        return Ok(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "No model file found. Please set MODEL_PATH environment variable or place a .gguf file in resources/models/"
        ))
    }

    fn auto_detect_threads() -> u32 {
        let num_cpus = num_cpus::get() as u32;
        info!("Auto‑detected CPU cores: {}", num_cpus);

        match num_cpus {
            1..=2 => 1,
            3..=4 => (num_cpus * 2) / 3,
            5..=8 => (num_cpus * 3) / 5,
            9..=16 => num_cpus / 2,
            17..=32 => (num_cpus * 2) / 5,
            _ => 16,
        }
    }

    fn auto_detect_gpu_layers() -> u32 {
        if let Ok(nvml) = Nvml::init() {
            if let Ok(device_count) = nvml.device_count() {
                if device_count > 0 {
                    if let Ok(first_gpu) = nvml.device_by_index(0) {
                        if let Ok(memory) = first_gpu.memory_info() {
                            let vram_gb = memory.total / 1024 / 1024 / 1024;
                            let layers = match vram_gb {
                                0..=4 => 12,
                                5..=8 => 20,
                                9..=12 => 32,
                                13..=16 => 40,
                                _ => 50,
                            };
                            info!("Auto‑detected GPU layers: {} ({} GB VRAM)", layers, vram_gb);
                            return layers;
                        }
                    }
                }
            }
        }
        warn!("Failed to detect GPU, using default 20 layers");
        20
    }

    fn auto_detect_ctx_size(model_path: &str) -> u32 {
        let inferred = Self::read_ctx_size_from_model_path(model_path)
            .unwrap_or_else(|| {
                info!("Falling back to default context size (8192)");
                8192
            });
        let adjusted = Self::adjust_ctx_size_for_system(inferred);
        info!("Final context size: {} (inferred: {})", adjusted, inferred);
        adjusted
    }

    fn read_ctx_size_from_model_path(model_path: &str) -> Option<u32> {
        // Simple heuristic based on model filename patterns
        let path_lower = model_path.to_lowercase();

        if path_lower.contains("32k") {
            Some(32768)
        } else if path_lower.contains("16k") {
            Some(16384)
        } else if path_lower.contains("8k") {
            Some(8192)
        } else if path_lower.contains("4k") {
            Some(4096)
        } else if path_lower.contains("2k") {
            Some(2048)
        } else if path_lower.contains("7b") || path_lower.contains("8b") {
            Some(4096)
        } else if path_lower.contains("13b") {
            Some(4096)
        } else if path_lower.contains("34b") || path_lower.contains("70b") {
            Some(8192)
        } else {
            // Default fallback
            Some(8192)
        }
    }

    fn adjust_ctx_size_for_system(inferred_ctx: u32) -> u32 {
        let mut system = System::new_all();
        system.refresh_memory();

        let available_ram_gb = system.available_memory() / 1024 / 1024 / 1024;
        let _total_ram_gb = system.total_memory() / 1024 / 1024 / 1024;

        let required_ram_gb = (inferred_ctx as f32 / 4096.0) * 1.5;
        if available_ram_gb < required_ram_gb as u64 {
            let adjusted = (available_ram_gb as f32 * 4096.0 / 1.5) as u32;
            let safe_ctx = adjusted.min(inferred_ctx).max(2048);
            warn!(
                "Reducing context size from {} → {} due to limited RAM ({}GB available)",
                inferred_ctx, safe_ctx, available_ram_gb
            );
            safe_ctx
        } else {
            inferred_ctx
        }
    }

    fn auto_detect_batch_size(gpu_layers: u32, ctx_size: u32) -> u32 {
        let mut system = System::new_all();
        system.refresh_memory();

        let available_mb = system.available_memory() / 1024;
        let has_gpu = gpu_layers > 0;
        let memory_per_batch = Self::estimate_memory_per_batch(ctx_size, has_gpu);
        let safe_available_mb = (available_mb as f32 * 0.6) as u32;
        let max_batch = (safe_available_mb as f32 / memory_per_batch).max(1.0) as u32;

        let optimal = Self::apply_batch_limits(max_batch, ctx_size, has_gpu);
        info!(
            "Auto batch size: {} (ctx: {}, GPU: {}, est mem: {:.1}MB/batch)",
            optimal, ctx_size, has_gpu, memory_per_batch
        );
        optimal
    }

    fn estimate_memory_per_batch(ctx_size: u32, has_gpu: bool) -> f32 {
        if has_gpu {
            (ctx_size as f32 / 1024.0) * 0.5
        } else {
            (ctx_size as f32 / 1024.0) * 1.2
        }
    }

    fn apply_batch_limits(batch_size: u32, ctx_size: u32, _has_gpu: bool) -> u32 {
        let limited = batch_size.max(16).min(1024);
        match ctx_size {
            0..=2048 => limited.min(512),
            2049..=4096 => limited.min(384),
            4097..=8192 => limited.min(256),
            8193..=16384 => limited.min(128),
            16385..=32768 => limited.min(64),
            _ => limited.min(32),
        }
    }

    pub fn print_config(&self) {
        info!("Current Configuration:");
        info!("- Model Path: {}", self.model_path);
        info!("- Llama Binary: {}", self.llama_bin);
        info!("- Context Size: {}", self.ctx_size);
        info!("- Batch Size: {}", self.batch_size);
        info!("- Threads: {}", self.threads);
        info!("- GPU Layers: {}", self.gpu_layers);
        info!("- Max Streams: {}", self.max_concurrent_streams);
        info!("- API: {}:{}", self.api_host, self.api_port);
        info!("- LLM Backend: {}:{}", self.llama_host, self.llama_port);
        info!("- Queue Size: {}", self.queue_size);
        info!("- Queue Timeout: {}s", self.queue_timeout_seconds);
    }

    pub fn api_addr(&self) -> SocketAddr {
        format!("{}:{}", self.api_host, self.api_port).parse().unwrap()
    }
}