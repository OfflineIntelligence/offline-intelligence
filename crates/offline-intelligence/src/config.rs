
use anyhow::Result;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use sysinfo::System;
use tracing::{debug, info, warn};

#[cfg(all(feature = "nvidia", any(target_os = "windows", target_os = "linux")))]
use nvml_wrapper::Nvml;

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
    pub parallel_slots: u32,
    pub ubatch_size: u32,
    pub prometheus_port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub requests_per_second: u32,
    pub generate_timeout_seconds: u64,
    pub stream_timeout_seconds: u64,
    pub health_check_timeout_seconds: u64,
    pub queue_size: usize,
    pub queue_timeout_seconds: u64,
    pub backend_url: String,
    pub openrouter_api_key: String,
    
    pub draft_model_path: String,
    
    pub speculative_draft_max: u32,
    
    pub speculative_draft_p_min: f32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        
        let mut env_loaded = false;

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let env_path = exe_dir.join(".env");
                if env_path.exists() {
                    match dotenvy::from_path(&env_path) {
                        Ok(_) => {
                            info!("Loaded .env from executable directory: {:?}", env_path);
                            env_loaded = true;
                        }
                        Err(e) => {
                            warn!("Failed to load .env from {:?}: {}", env_path, e);
                        }
                    }
                }

                #[cfg(target_os = "macos")]
                if !env_loaded {
                    
                    if let Some(contents_dir) = exe_dir.parent() {
                        let bundle_env = contents_dir.join("Resources").join(".env");
                        if bundle_env.exists() {
                            match dotenvy::from_path(&bundle_env) {
                                Ok(_) => {
                                    info!("Loaded .env from macOS bundle Resources: {:?}", bundle_env);
                                    env_loaded = true;
                                }
                                Err(e) => {
                                    warn!("Failed to load .env from bundle Resources {:?}: {}", bundle_env, e);
                                }
                            }
                        }
                    }
                }

                #[cfg(target_os = "macos")]
                if !env_loaded {
                    if let Some(app_support) = dirs::data_dir() {
                        let user_env = app_support.join("Aud.io").join(".env");
                        if user_env.exists() {
                            match dotenvy::from_path(&user_env) {
                                Ok(_) => {
                                    info!("Loaded .env from user data directory: {:?}", user_env);
                                    env_loaded = true;
                                }
                                Err(e) => {
                                    warn!("Failed to load .env from user data dir {:?}: {}", user_env, e);
                                }
                            }
                        }
                    }
                }

                if !env_loaded {
                    let project_root = if exe_dir.ends_with("target/release")
                        || exe_dir.ends_with("target\\release")
                    {
                        exe_dir.parent().and_then(|p| p.parent())
                    } else {
                        None
                    };

                    if let Some(root) = project_root {
                        let root_env = root.join(".env");
                        if root_env.exists() {
                            match dotenvy::from_path(&root_env) {
                                Ok(_) => {
                                    info!("Loaded .env from project root: {:?}", root_env);
                                    env_loaded = true;
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to load .env from project root {:?}: {}",
                                        root_env, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if !env_loaded {
            if let Err(e) = dotenvy::dotenv() {
                warn!("Failed to load .env from current directory: {}. Using system environment variables.", e);
            } else {
                info!("Loaded environment variables from .env file in current directory");
            }
        }

        let llama_bin = Self::get_llama_binary_path()?;
        info!("Using llama binary: {}", llama_bin);

        let model_path = Self::get_model_path_with_fallback()?;

        let threads = if env::var("THREADS").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_threads()
        } else {
            env::var("THREADS")
                .unwrap_or_else(|_| "6".into())
                .parse()
                .unwrap_or(6)
        };

        let ctx_size = if env::var("CTX_SIZE").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_ctx_size(&model_path)
        } else {
            env::var("CTX_SIZE")
                .unwrap_or_else(|_| "8192".into())
                .parse()
                .unwrap_or(8192)
        };

        let parallel_slots: u32 = env::var("PARALLEL_SLOTS")
            .unwrap_or_else(|_| "8".into())
            .parse()
            .unwrap_or(8);

        let gpu_layers = if env::var("GPU_LAYERS").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_gpu_layers(&model_path, ctx_size, parallel_slots)
        } else {
            env::var("GPU_LAYERS")
                .unwrap_or_else(|_| "20".into())
                .parse()
                .unwrap_or(20)
        };

        let batch_size = if env::var("BATCH_SIZE").unwrap_or_else(|_| "auto".into()) == "auto" {
            Self::auto_detect_batch_size(gpu_layers, ctx_size)
        } else {
            env::var("BATCH_SIZE")
                .unwrap_or_else(|_| "256".into())
                .parse()
                .unwrap_or(256)
        };

        let llama_host = env::var("LLAMA_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let llama_port = env::var("LLAMA_PORT")
            .unwrap_or_else(|_| "8081".into())
            .parse()?;
        let backend_url = format!("http://{}:{}", llama_host, llama_port);

        let openrouter_api_key = env::var("OPENROUTER_API_KEY").unwrap_or_default();

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
            parallel_slots,
            ubatch_size: env::var("UBATCH_SIZE")
                .unwrap_or_else(|_| "512".into())
                .parse()
                .unwrap_or(512),
            prometheus_port: env::var("PROMETHEUS_PORT")
                .unwrap_or_else(|_| "9000".into())
                .parse()?,
            api_host: env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            api_port: env::var("API_PORT")
                .unwrap_or_else(|_| "9999".into())
                .parse()?,
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
            backend_url,
            openrouter_api_key,
            draft_model_path: env::var("DRAFT_MODEL_PATH")
                .unwrap_or_else(|_| "none".into()),
            speculative_draft_max: env::var("SPECULATIVE_DRAFT_MAX")
                .unwrap_or_else(|_| "8".into())
                .parse()
                .unwrap_or(8),
            speculative_draft_p_min: env::var("SPECULATIVE_DRAFT_P_MIN")
                .unwrap_or_else(|_| "0.4".into())
                .parse()
                .unwrap_or(0.4),
        })
    }

    fn get_model_path_with_fallback() -> Result<String> {
        
        if let Ok(model_path) = env::var("MODEL_PATH") {
            
            if std::path::Path::new(&model_path).exists() {
                info!("Using model from MODEL_PATH: {}", model_path);
                return Ok(model_path);
            } else {
                warn!("MODEL_PATH set but file doesn't exist: {}", model_path);
            }
        }

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let possible_model_locations = vec![
            
            exe_dir.join("resources/models/default.gguf"),
            exe_dir.join("resources/models/model.gguf"),
            exe_dir.join("models/default.gguf"),
            exe_dir.join("models/model.gguf"),
            exe_dir.join("default.gguf"),
            
            exe_dir.join("resources/models/default.onnx"),
            exe_dir.join("resources/models/model.onnx"),
            
            exe_dir.join("resources/models/default.trt"),
            exe_dir.join("resources/models/model.engine"),
            
            exe_dir.join("resources/models/default.safetensors"),
            exe_dir.join("resources/models/model.safetensors"),
            
            exe_dir.join("resources/models/default.ggml"),
            exe_dir.join("resources/models/model.bin"),
        ];

        for model_path in possible_model_locations {
            if model_path.exists() {
                info!("Using embedded model: {}", model_path.display());
                return Ok(model_path.to_string_lossy().to_string());
            }
        }

        if let Ok(entries) = std::fs::read_dir(exe_dir.join("resources/models")) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    let ext_str = ext.to_str().unwrap_or("").to_lowercase();
                    
                    if matches!(
                        ext_str.as_str(),
                        "gguf"
                            | "ggml"
                            | "onnx"
                            | "trt"
                            | "engine"
                            | "plan"
                            | "safetensors"
                            | "mlmodel"
                    ) {
                        info!("Using found model: {}", entry.path().display());
                        return Ok(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok("".to_string())
    }

    fn get_llama_binary_path() -> Result<String> {
        
        if let Ok(llama_bin) = env::var("LLAMA_BIN") {
            if std::path::Path::new(&llama_bin).exists() {
                info!("Using llama binary from LLAMA_BIN env: {}", llama_bin);
                return Ok(llama_bin);
            } else {
                warn!(
                    "LLAMA_BIN set but file doesn't exist: {}, falling back to auto-detection",
                    llama_bin
                );
            }
        }

        let (os_folder, binary_name) = Self::get_platform_binary_info();
        info!(
            "Auto-detecting llama binary for OS: {} (binary: {})",
            os_folder, binary_name
        );

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()));

        let cwd = std::env::current_dir().ok();

        let mut search_dirs: Vec<PathBuf> = Vec::new();

        if let Some(ref exe) = exe_dir {
            search_dirs.push(exe.clone());
            
            if let Some(parent) = exe.parent() {
                search_dirs.push(parent.to_path_buf());
                if let Some(grandparent) = parent.parent() {
                    search_dirs.push(grandparent.to_path_buf());
                }
            }
        }

        if let Some(ref cwd_path) = cwd {
            search_dirs.push(cwd_path.clone());
        }

        #[cfg(debug_assertions)]
        {
            let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            search_dirs.push(crate_dir);
        }

        let resource_folder_names = ["Resources", "resources"];
        for base_dir in &search_dirs {
            for resource_folder in &resource_folder_names {
            let bin_dir = base_dir.join(resource_folder).join("bin").join(os_folder);

            if bin_dir.exists() {
                
                if let Ok(entries) = std::fs::read_dir(&bin_dir) {
                    
                    let mut dir_entries: Vec<_> = entries.flatten().collect();
                    dir_entries.sort_by_key(|e| e.file_name());

                    for entry in dir_entries {
                        let entry_path = entry.path();
                        if !entry_path.is_dir() {
                            continue;
                        }

                        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                        {
                            let dir_name = entry_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            
                            if dir_name.contains("x64") || dir_name.contains("x86_64") {
                                debug!("Skipping Intel subdir on Apple Silicon: {}", dir_name);
                                continue;
                            }
                        }
                        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
                        {
                            let dir_name = entry_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            
                            if dir_name.contains("arm64") || dir_name.contains("aarch64") {
                                debug!("Skipping ARM subdir on Intel Mac: {}", dir_name);
                                continue;
                            }
                        }

                        let potential_binary = entry_path.join(binary_name);
                        if potential_binary.exists() {
                            info!("Found llama binary at: {}", potential_binary.display());
                            return Ok(potential_binary.to_string_lossy().to_string());
                        }
                    }
                }

                let direct_binary = bin_dir.join(binary_name);
                if direct_binary.exists() {
                    info!("Found llama binary at: {}", direct_binary.display());
                    return Ok(direct_binary.to_string_lossy().to_string());
                }
            }
            } 
        }

        let arch = Self::get_arch_hint();
        warn!(
            "Llama binary not found. Searched in Resources/bin/{os_folder}/ for '{binary_name}'.\n\
             Please either:\n\
             1. Set LLAMA_BIN environment variable to the full path\n\
             2. Place the binary in Resources/bin/{os_folder}/<subfolder>/\n\
             \n\
             Expected binary name: {binary_name}\n\
             OS detected: {os_folder}\n\
             Architecture: {arch}\n\
             Searched directories: {:?}",
            search_dirs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
        );

        Ok(String::new())
    }

    fn get_platform_binary_info() -> (&'static str, &'static str) {
        #[cfg(target_os = "windows")]
        {
            ("Windows", "llama-server.exe")
        }

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            ("MacOS", "llama-server")
            
        }

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            ("MacOS", "llama-server")
            
        }

        #[cfg(target_os = "linux")]
        {
            ("Linux", "llama-server")
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            compile_error!(
                "Unsupported operating system. Only Windows, macOS, and Linux are supported."
            );
        }
    }

    fn get_arch_hint() -> &'static str {
        #[cfg(target_arch = "x86_64")]
        {
            "x64"
        }
        #[cfg(target_arch = "aarch64")]
        {
            "arm64"
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            "unknown"
        }
    }

    fn auto_detect_threads() -> u32 {
        let threads = num_cpus::get() as u32;
        info!("Auto-detected {} CPU cores for inference", threads);
        threads
    }

    fn layers_for_vram(vram_mb: u64, model_path: &str, ctx_size: u32, parallel_slots: u32) -> u32 {
        let path_lower = model_path.to_lowercase();

        let params_b: f64 =
            if path_lower.contains("0.5b") { 0.5 }
            else if path_lower.contains("1.5b") { 1.5 }
            else if path_lower.contains("1b") && !path_lower.contains("13b") { 1.0 }
            else if path_lower.contains("3b") && !path_lower.contains("13b") && !path_lower.contains("33b") { 3.0 }
            else if path_lower.contains("7b") { 7.0 }
            else if path_lower.contains("8b") { 8.0 }
            else if path_lower.contains("13b") { 13.0 }
            else if path_lower.contains("14b") { 14.0 }
            else if path_lower.contains("33b") || path_lower.contains("34b") { 34.0 }
            else if path_lower.contains("70b") { 70.0 }
            else { 7.0 }; 

        let bits: f64 =
            if path_lower.contains("q4_k_m") || path_lower.contains("q4_k_s") { 4.5 }
            else if path_lower.contains("q4_k") { 4.5 }
            else if path_lower.contains("q4_0") || path_lower.contains("q4_1") { 4.0 }
            else if path_lower.contains("q5_k_m") || path_lower.contains("q5_k_s") { 5.5 }
            else if path_lower.contains("q5") { 5.0 }
            else if path_lower.contains("q6_k") { 6.5 }
            else if path_lower.contains("q8_0") { 8.5 }
            else if path_lower.contains("f16") || path_lower.contains("fp16") { 16.0 }
            else if path_lower.contains("f32") || path_lower.contains("fp32") { 32.0 }
            else { 4.5 }; 

        let total_layers: u32 =
            if params_b <= 0.6  { 24 }
            else if params_b <= 1.6  { 28 }
            else if params_b <= 3.5  { 28 }
            else if params_b <= 8.5  { 32 }
            else if params_b <= 14.5 { 40 }
            else if params_b <= 35.0 { 48 }
            else                     { 80 };

        let model_vram_mb = (params_b * 1e9 * bits / 8.0 / 1024.0 / 1024.0) as u64;

        let base_kv_mb = (model_vram_mb as f64 * 0.14).max(64.0);
        let kv_mb = (base_kv_mb
            * (ctx_size as f64 / 8192.0)
            * ((parallel_slots as f64 / 8.0).sqrt())).max(64.0) as u64;

        let overhead_mb: u64 = 384;

        let available_mb = vram_mb.saturating_sub(overhead_mb + kv_mb);

        if available_mb >= model_vram_mb {
            
            info!(
                "GPU auto-detect: full offload — model {:.0} MB fits in {:.0} MB available → {} layers",
                model_vram_mb, available_mb, total_layers
            );
            total_layers
        } else {
            
            let per_layer_mb = (model_vram_mb as f64 / total_layers as f64).ceil() as u64;
            let fit_layers = if per_layer_mb > 0 {
                (available_mb / per_layer_mb).min(total_layers as u64) as u32
            } else {
                0
            };
            info!(
                "GPU auto-detect: partial offload {}/{} layers ({} MB model, {} MB available, {} MB/layer)",
                fit_layers, total_layers, model_vram_mb, available_mb, per_layer_mb
            );
            fit_layers
        }
    }

    fn auto_detect_gpu_layers(model_path: &str, ctx_size: u32, parallel_slots: u32) -> u32 {
        
        #[cfg(all(feature = "nvidia", any(target_os = "windows", target_os = "linux")))]
        {
            if let Ok(nvml) = Nvml::init() {
                if let Ok(device_count) = nvml.device_count() {
                    if device_count > 0 {
                        if let Ok(first_gpu) = nvml.device_by_index(0) {
                            if let Ok(memory) = first_gpu.memory_info() {
                                let vram_mb = memory.total / 1024 / 1024;
                                let layers = Self::layers_for_vram(vram_mb, model_path, ctx_size, parallel_slots);
                                info!(
                                    "Auto‑detected NVIDIA GPU layers: {} ({} MB VRAM)",
                                    layers, vram_mb
                                );
                                return layers;
                            }
                        }
                    }
                }
            }
            info!("No NVIDIA GPU detected, using CPU-optimized defaults (0 GPU layers)");
            0
        }

        #[cfg(not(all(feature = "nvidia", any(target_os = "windows", target_os = "linux"))))]
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            use std::process::{Command, Stdio};

            let child = Command::new("nvidia-smi")
                .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();

            match child {
                Ok(mut process) => {
                    let start = std::time::Instant::now();
                    loop {
                        match process.try_wait() {
                            Ok(Some(status)) => {
                                if status.success() {
                                    if let Ok(output) = process.wait_with_output() {
                                        let stdout = String::from_utf8_lossy(&output.stdout);
                                        if let Some(vram_mb_str) = stdout.lines().next() {
                                            if let Ok(vram_mb) = vram_mb_str.trim().parse::<u64>() {
                                                let layers = Self::layers_for_vram(vram_mb, model_path, ctx_size, parallel_slots);
                                                info!(
                                                    "Auto‑detected NVIDIA GPU layers via nvidia-smi: {} ({} MB VRAM)",
                                                    layers, vram_mb
                                                );
                                                return layers;
                                            }
                                        }
                                    }
                                }
                                info!("nvidia-smi returned but could not parse VRAM, using CPU defaults (0 GPU layers)");
                                return 0;
                            }
                            Ok(None) => {
                                if start.elapsed() > std::time::Duration::from_secs(5) {
                                    let _ = process.kill();
                                    let _ = process.wait();
                                    info!("nvidia-smi timed out, using CPU defaults (0 GPU layers)");
                                    return 0;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
                            Err(_) => {
                                return 0;
                            }
                        }
                    }
                }
                Err(_) => {
                    info!("No NVIDIA GPU detected (nvidia-smi not available), using CPU defaults (0 GPU layers)");
                    0
                }
            }
        }

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            
            let total_mem_gb = {
                let mut sys = System::new_all();
                sys.refresh_memory();
                sys.total_memory() / 1024 / 1024 / 1024
            };

            let layers = match total_mem_gb {
                0..=8 => 24,   
                9..=16 => 32,  
                17..=32 => 40, 
                33..=64 => 48, 
                _ => 56,       
            };
            info!(
                "Apple Silicon detected ({} GB unified memory), using Metal GPU layers: {}",
                total_mem_gb, layers
            );
            layers
        }

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            
            info!("Intel Mac detected, using CPU-only mode (0 GPU layers)");
            0
        }
    }

    fn auto_detect_ctx_size(model_path: &str) -> u32 {
        let inferred = Self::read_ctx_size_from_model_path(model_path).unwrap_or_else(|| {
            info!("Falling back to default context size (8192)");
            8192
        });
        let adjusted = Self::adjust_ctx_size_for_system(inferred);
        info!("Final context size: {} (inferred: {})", adjusted, inferred);
        adjusted
    }

    fn read_ctx_size_from_model_path(model_path: &str) -> Option<u32> {
        
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
        } else if path_lower.contains("7b")
            || path_lower.contains("8b")
            || path_lower.contains("13b")
        {
            Some(4096)
        } else if path_lower.contains("34b") || path_lower.contains("70b") {
            Some(8192)
        } else {
            
            Some(8192)
        }
    }

    fn adjust_ctx_size_for_system(inferred_ctx: u32) -> u32 {
        let mut system = System::new_all();
        system.refresh_memory();

        let available_ram_gb = system.available_memory() / 1024 / 1024 / 1024;

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
        let limited = batch_size.clamp(16, 1024);
        match ctx_size {
            0..=2048 => limited.min(512),
            2049..=4096 => limited.min(512),
            
            4097..=8192 => limited.min(512),
            8193..=16384 => limited.min(256),
            16385..=32768 => limited.min(128),
            _ => limited.min(64),
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
        info!("- Parallel Slots: {}", self.parallel_slots);
        info!("- Ubatch Size: {}", self.ubatch_size);
        info!("- Max Streams: {}", self.max_concurrent_streams);
        info!("- API: {}:{}", self.api_host, self.api_port);
        info!("- Backend: {}:{}", self.llama_host, self.llama_port);
        info!("- Queue Size: {}", self.queue_size);
        info!("- Queue Timeout: {}s", self.queue_timeout_seconds);
        info!("- Backend URL: {}", self.backend_url);
    }

    pub fn api_addr(&self) -> SocketAddr {
        format!("{}:{}", self.api_host, self.api_port)
            .parse()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            model_path: "/test/model.gguf".to_string(),
            llama_bin: "/test/llama-server".to_string(),
            llama_host: "127.0.0.1".to_string(),
            llama_port: 8001,
            ctx_size: 8192,
            batch_size: 128,
            threads: 6,
            gpu_layers: 20,
            health_timeout_seconds: 600,
            hot_swap_grace_seconds: 25,
            max_concurrent_streams: 2,
            prometheus_port: 9000,
            api_host: "127.0.0.1".to_string(),
            api_port: 9999,
            requests_per_second: 24,
            generate_timeout_seconds: 300,
            stream_timeout_seconds: 600,
            health_check_timeout_seconds: 900,
            queue_size: 1000,
            queue_timeout_seconds: 300,
            backend_url: "http://127.0.0.1:8001".to_string(),
            openrouter_api_key: "test-api-key".to_string(),
        }
    }

    #[test]
    fn test_config_creation_with_default_values() {
        let config = create_test_config();

        assert_eq!(config.model_path, "/test/model.gguf");
        assert_eq!(config.llama_bin, "/test/llama-server");
        assert_eq!(config.api_port, 9999);
        assert_eq!(config.llama_port, 8001);
    }

    #[test]
    fn test_config_clone() {
        let config1 = create_test_config();
        let config2 = config1.clone();

        assert_eq!(config1.api_host, config2.api_host);
        assert_eq!(config1.threads, config2.threads);
        assert_eq!(config1.gpu_layers, config2.gpu_layers);
    }

    #[test]
    fn test_api_addr_parsing() {
        let config = create_test_config();
        let addr = config.api_addr();

        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), 9999);
    }

    #[test]
    fn test_api_addr_with_different_ports() {
        let mut config = create_test_config();
        config.api_port = 3000;

        let addr = config.api_addr();
        assert_eq!(addr.port(), 3000);
    }

    #[test]
    fn test_api_addr_with_zero_address() {
        let mut config = create_test_config();
        config.api_host = "0.0.0.0".to_string();
        config.api_port = 5000;

        let addr = config.api_addr();
        assert_eq!(addr.port(), 5000);
        
        assert_eq!(addr.ip().to_string(), "0.0.0.0");
    }

    #[test]
    fn test_config_timeouts_are_positive() {
        let config = create_test_config();

        assert!(config.health_timeout_seconds > 0);
        assert!(config.generate_timeout_seconds > 0);
        assert!(config.stream_timeout_seconds > 0);
        assert!(config.health_check_timeout_seconds > 0);
    }

    #[test]
    fn test_health_check_timeout_greater_than_health_timeout() {
        let config = create_test_config();

        assert!(config.health_check_timeout_seconds >= config.health_timeout_seconds);
    }

    #[test]
    fn test_max_concurrent_streams_is_positive() {
        let config = create_test_config();
        assert!(config.max_concurrent_streams > 0);
    }

    #[test]
    fn test_requests_per_second_is_reasonable() {
        let config = create_test_config();

        assert!(config.requests_per_second > 0);
        assert!(config.requests_per_second <= 1000);
    }

    #[test]
    fn test_queue_size_is_positive() {
        let config = create_test_config();
        assert!(config.queue_size > 0);
    }

    #[test]
    fn test_context_size_within_valid_range() {
        let config = create_test_config();

        assert!(config.ctx_size >= 512);
        assert!(config.ctx_size <= 32768);
    }

    #[test]
    fn test_batch_size_valid_range() {
        let config = create_test_config();

        assert!(config.batch_size >= 16);
        assert!(config.batch_size <= 1024);
    }

    #[test]
    fn test_batch_size_reasonable_vs_context() {
        let config = create_test_config();

        assert!(config.batch_size < config.ctx_size);
    }

    #[test]
    fn test_threads_is_positive() {
        let config = create_test_config();
        assert!(config.threads > 0);
    }

    #[test]
    fn test_threads_within_reasonable_range() {
        let config = create_test_config();

        assert!(config.threads <= 256);
    }

    #[test]
    fn test_gpu_layers_non_negative() {
        let config = create_test_config();
        assert!(config.gpu_layers <= config.ctx_size);
    }

    #[test]
    fn test_gpu_layers_within_range() {
        let config = create_test_config();

        assert!(config.gpu_layers <= 100);
    }

    #[test]
    fn test_api_port_valid() {
        let config = create_test_config();
        assert!(config.api_port > 0);
        assert!(config.api_port != config.llama_port);
    }

    #[test]
    fn test_llama_port_valid() {
        let config = create_test_config();
        assert!(config.llama_port > 0);
    }

    #[test]
    fn test_prometheus_port_valid() {
        let config = create_test_config();
        assert!(config.prometheus_port > 0);
    }

    #[test]
    fn test_ports_are_different() {
        let config = create_test_config();

        assert_ne!(config.api_port, config.llama_port);
        assert_ne!(config.api_port, config.prometheus_port);
        assert_ne!(config.llama_port, config.prometheus_port);
    }

    #[test]
    fn test_model_path_not_empty() {
        let config = create_test_config();
        assert!(!config.model_path.is_empty());
    }

    #[test]
    fn test_llama_bin_not_empty() {
        let config = create_test_config();
        assert!(!config.llama_bin.is_empty());
    }

    #[test]
    fn test_backend_url_not_empty() {
        let config = create_test_config();
        assert!(!config.backend_url.is_empty());
    }

    #[test]
    fn test_backend_url_format() {
        let config = create_test_config();

        assert!(
            config.backend_url.starts_with("http://") || config.backend_url.starts_with("https://")
        );
    }

    #[test]
    fn test_api_host_not_empty() {
        let config = create_test_config();
        assert!(!config.api_host.is_empty());
    }

    #[test]
    fn test_llama_host_not_empty() {
        let config = create_test_config();
        assert!(!config.llama_host.is_empty());
    }

    #[test]
    fn test_hot_swap_grace_positive() {
        let config = create_test_config();
        assert!(config.hot_swap_grace_seconds > 0);
    }

    #[test]
    fn test_hot_swap_grace_reasonable() {
        let config = create_test_config();

        assert!(config.hot_swap_grace_seconds < 300);
    }

    #[test]
    fn test_auto_detect_threads_returns_positive() {
        let threads = Config::auto_detect_threads();
        assert!(threads > 0);
    }

    #[test]
    fn test_auto_detect_gpu_layers_non_negative() {
        let layers = Config::auto_detect_gpu_layers("qwen2.5-coder-3b-instruct-q4_k_m.gguf", 8192, 8);
        assert!(layers <= 512);
    }

    #[test]
    fn test_layers_for_vram_full_offload() {
        
        let layers = Config::layers_for_vram(4096, "qwen2.5-coder-3b-instruct-q4_k_m.gguf", 8192, 8);
        assert_eq!(layers, 28, "3B model should fully offload on 4GB GPU");
    }

    #[test]
    fn test_layers_for_vram_partial_offload() {
        
        let layers = Config::layers_for_vram(2048, "qwen2.5-coder-7b-instruct-q4_k_m.gguf", 8192, 8);
        assert!(layers < 32, "7B model should only partially offload on 2GB GPU");
        assert!(layers > 0, "Should get at least some layers on 2GB GPU");
    }

    #[test]
    fn test_apply_batch_limits_small_context() {
        
        let batch = Config::apply_batch_limits(1024, 1024, false);
        assert!(batch <= 512);
    }

    #[test]
    fn test_apply_batch_limits_medium_context() {
        
        let batch = Config::apply_batch_limits(1024, 3000, false);
        assert!(batch <= 384);
    }

    #[test]
    fn test_apply_batch_limits_large_context() {
        
        let batch = Config::apply_batch_limits(1024, 24576, false);
        assert!(batch <= 64);
    }

    #[test]
    fn test_apply_batch_limits_minimum() {
        
        let batch = Config::apply_batch_limits(1, 8192, false);
        assert!(batch >= 16);
    }

    #[test]
    fn test_estimate_memory_per_batch_cpu() {
        let memory_cpu = Config::estimate_memory_per_batch(8192, false);
        assert!(memory_cpu > 0.0);
    }

    #[test]
    fn test_estimate_memory_per_batch_gpu() {
        let memory_gpu = Config::estimate_memory_per_batch(8192, true);
        assert!(memory_gpu > 0.0);
    }

    #[test]
    fn test_estimate_memory_gpu_less_than_cpu() {
        let memory_cpu = Config::estimate_memory_per_batch(8192, false);
        let memory_gpu = Config::estimate_memory_per_batch(8192, true);

        assert!(memory_gpu < memory_cpu);
    }

    #[test]
    fn test_queue_timeout_is_positive() {
        let config = create_test_config();
        assert!(config.queue_timeout_seconds > 0);
    }

    #[test]
    fn test_queue_timeout_less_than_generate_timeout() {
        let config = create_test_config();

        assert!(config.queue_timeout_seconds <= config.generate_timeout_seconds);
    }

    #[test]
    fn test_config_values_consistency() {
        let config = create_test_config();

        assert!(config.health_timeout_seconds <= 3600); 
        assert!(config.generate_timeout_seconds <= 1800); 
        assert!(config.stream_timeout_seconds <= 3600); 
        assert!(config.health_check_timeout_seconds <= 3600); 
    }

    #[test]
    fn test_config_backend_url_consistency() {
        let config = create_test_config();

        assert!(
            config.backend_url.contains(&config.llama_host)
                || config.backend_url.contains("127.0.0.1")
                || config.backend_url.contains("localhost")
        );
    }

    #[test]
    fn test_config_all_fields_initialized() {
        let config = create_test_config();

        assert!(!config.model_path.is_empty());
        assert!(!config.llama_bin.is_empty());
        assert!(!config.api_host.is_empty());
        assert!(!config.llama_host.is_empty());
        assert!(config.threads > 0);
        assert!(config.gpu_layers <= config.ctx_size);
        assert!(config.api_port > 0);
        assert!(config.llama_port > 0);
    }
}
