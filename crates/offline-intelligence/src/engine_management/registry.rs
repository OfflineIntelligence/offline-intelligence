
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

use crate::model_runtime::platform_detector::{HardwareCapabilities, Platform, HardwareArchitecture};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccelerationType {
    CPU,
    CUDA,
    Metal,
    Vulkan,
    OpenCL,
    DirectML,
}

impl std::fmt::Display for AccelerationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccelerationType::CPU => write!(f, "CPU"),
            AccelerationType::CUDA => write!(f, "CUDA"),
            AccelerationType::Metal => write!(f, "Metal"),
            AccelerationType::Vulkan => write!(f, "Vulkan"),
            AccelerationType::OpenCL => write!(f, "OpenCL"),
            AccelerationType::DirectML => write!(f, "DirectML"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EngineStatus {
    NotInstalled,
    Available,
    Downloading,
    Installed,
    Active,
    Corrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub platform: Platform,
    pub architecture: HardwareArchitecture,
    pub acceleration: AccelerationType,
    pub download_url: String,
    pub file_size: u64,
    pub checksum: String,
    pub compatibility_score: f32,
    pub status: EngineStatus,
    pub install_path: Option<PathBuf>,
    pub binary_name: String,
    pub required_dependencies: Vec<String>,
}

impl EngineInfo {
    
    pub fn calculate_compatibility(&self, hardware: &HardwareCapabilities) -> f32 {
        let mut score: f32 = 0.0;
        
        if self.platform == hardware.platform {
            score += 50.0;
        } else {
            return 0.0; 
        }
        
        if self.architecture == hardware.architecture {
            score += 20.0;
        }
        
        match (&self.acceleration, hardware) {
            (AccelerationType::CPU, _) => score += 15.0,
            (AccelerationType::CUDA, hw) if hw.has_cuda => score += 25.0,
            (AccelerationType::Metal, hw) if hw.has_metal => score += 25.0,
            (AccelerationType::Vulkan, hw) if hw.has_vulkan => score += 20.0,
            _ => {
                
                if self.acceleration != AccelerationType::CPU {
                    score -= 10.0;
                }
            }
        }
        
        if self.is_recent_version() {
            score += 5.0;
        }
        
        score.clamp(0.0, 100.0)
    }
    
    fn is_recent_version(&self) -> bool {
        
        self.version.contains("b") || self.version.contains("latest")
    }
}

pub struct EngineRegistry {
    pub installed_engines: HashMap<String, EngineInfo>,
    pub available_engines: Vec<EngineInfo>,
    pub default_engine: Option<String>,
    pub storage_path: PathBuf,
    
    download_in_progress: Arc<RwLock<bool>>,
}

impl EngineRegistry {
    pub fn new() -> Result<Self> {
        let storage_path = Self::get_engine_storage_path()?;
        std::fs::create_dir_all(&storage_path)?;
        
        Ok(Self {
            installed_engines: HashMap::new(),
            available_engines: Vec::new(),
            default_engine: None,
            download_in_progress: Arc::new(RwLock::new(false)),
            storage_path,
        })
    }
    
    fn get_engine_storage_path() -> Result<PathBuf> {
        let base_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get APPDATA directory"))?
                .join("Aud.io")
                .join("engines")
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get Library directory"))?
                .join("Aud.io")
                .join("engines")
        } else {
            dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get .local/share directory"))?
                .join("aud.io")
                .join("engines")
        };
        
        Ok(base_dir)
    }
    
    pub async fn scan_installed_engines(&mut self, hardware_capabilities: &HardwareCapabilities) -> Result<()> {
        self.installed_engines.clear();

        if self.storage_path.exists() {
            for entry in std::fs::read_dir(&self.storage_path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let engine_dir = entry.path();
                    match self.load_engine_metadata(engine_dir.clone()).await {
                        Some(engine_info) => {
                            self.installed_engines.insert(engine_info.id.clone(), engine_info);
                        }
                        None => {
                            
                            if self.has_binary(&engine_dir) {
                                warn!("Found orphaned engine at {} (missing or invalid metadata.json). Consider re-downloading this engine.", engine_dir.display());
                            }
                        }
                    }
                }
            }
        }

        self.refresh_available_engines(hardware_capabilities).await?;

        debug!("Found {} installed engines", self.installed_engines.len());
        Ok(())
    }

    fn has_binary(&self, engine_dir: &PathBuf) -> bool {
        let binary_names = ["llama-server.exe", "llama-server", "llama-cli.exe", "llama-cli"];
        let platform_dirs = ["Windows", "windows", "Linux", "MacOS", "macos"];

        for platform_dir in platform_dirs.iter() {
            let platform_path = engine_dir.join(platform_dir);
            if platform_path.exists() {
                for binary_name in binary_names.iter() {
                    if platform_path.join(binary_name).exists() {
                        return true;
                    }
                }
            }
        }

        false
    }
    
    async fn load_engine_metadata(&self, engine_dir: PathBuf) -> Option<EngineInfo> {
        let metadata_path = engine_dir.join("metadata.json");
        if !metadata_path.exists() {
            return None;
        }
        
        match std::fs::read_to_string(&metadata_path) {
            Ok(content) => {
                match serde_json::from_str::<EngineInfo>(&content) {
                    Ok(mut engine_info) => {
                        
                        let binary_path = engine_dir.join(&engine_info.binary_name);
                        if binary_path.exists() {
                            engine_info.status = EngineStatus::Installed;
                            engine_info.install_path = Some(engine_dir);
                            Some(engine_info)
                        } else {
                            warn!("Engine binary not found: {:?}", binary_path);
                            None
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse engine metadata: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read engine metadata: {}", e);
                None
            }
        }
    }
    
    pub fn get_compatible_engines(&self, hardware: &HardwareCapabilities) -> Vec<&EngineInfo> {
        self.installed_engines
            .values()
            .filter(|engine| {
                let compatibility = engine.calculate_compatibility(hardware);
                compatibility > 30.0 && engine.status == EngineStatus::Installed
            })
            .collect()
    }
    
    pub fn select_best_compatible_engine(&self, hardware: &HardwareCapabilities) -> Option<EngineInfo> {
        let mut compatible_engines: Vec<_> = self.get_compatible_engines(hardware)
            .into_iter()
            .map(|engine| {
                let score = engine.calculate_compatibility(hardware);
                (engine, score)
            })
            .collect();
            
        compatible_engines.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        compatible_engines.first().map(|(engine, _)| (*engine).clone())
    }
    
    pub fn get_recommended_engine(&self, hardware: &HardwareCapabilities) -> Option<EngineInfo> {
        
        if let Some(best_installed) = self.select_best_compatible_engine(hardware) {
            if best_installed.calculate_compatibility(hardware) > 70.0 {
                return Some(best_installed);
            }
        }
        
        if !self.available_engines.is_empty() {
            let mut compatible_engines: Vec<_> = self.available_engines
                .iter()
                .map(|engine| {
                    let score = engine.calculate_compatibility(hardware);
                    (engine, score)
                })
                .filter(|(_, score)| *score > 0.0) 
                .collect();
            
            compatible_engines.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            if let Some((engine, _)) = compatible_engines.first() {
                let mut engine_clone = (*engine).clone();
                engine_clone.status = EngineStatus::Available; 
                return Some(engine_clone);
            }
        }
        
        self.get_official_engine_recommendation(hardware)
    }
    
    fn get_latest_version(&self) -> String {
        
        "b8037".to_string()
    }
    
    fn get_official_engine_recommendation(&self, hardware: &HardwareCapabilities) -> Option<EngineInfo> {
        let latest_version = self.get_latest_version();
        
        match (&hardware.platform, &hardware.architecture, hardware.has_cuda, hardware.has_metal) {
            (Platform::Windows, HardwareArchitecture::X86_64, true, _) => {
                
                Some(EngineInfo {
                    id: format!("llama-cuda-windows-x64-{}", latest_version),
                    name: format!("llama.cpp CUDA Windows x64 ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Windows,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CUDA,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-cuda-12.4-x64.zip", latest_version, latest_version),
                    file_size: 373 * 1024 * 1024, 
                    checksum: "".to_string(), 
                    compatibility_score: 95.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server.exe".to_string(),
                    required_dependencies: vec!["CUDA 12.4+ Runtime".to_string()],
                })
            }
            (Platform::Windows, HardwareArchitecture::X86_64, false, _) => {
                
                Some(EngineInfo {
                    id: format!("llama-cpu-windows-x64-{}", latest_version),
                    name: format!("llama.cpp CPU Windows x64 ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Windows,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-x64.zip", latest_version, latest_version),
                    file_size: 50 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 85.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server.exe".to_string(),
                    required_dependencies: vec![],
                })
            }
            (Platform::MacOS, HardwareArchitecture::Aarch64, _, true) => {
                
                Some(EngineInfo {
                    id: format!("llama-metal-macos-arm64-{}", latest_version),
                    name: format!("llama.cpp Metal macOS ARM64 ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::MacOS,
                    architecture: HardwareArchitecture::Aarch64,
                    acceleration: AccelerationType::Metal,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-arm64.tar.gz", latest_version, latest_version),
                    file_size: 29 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 95.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                })
            }
            (Platform::MacOS, HardwareArchitecture::X86_64, _, _) => {
                
                Some(EngineInfo {
                    id: format!("llama-cpu-macos-x64-{}", latest_version),
                    name: format!("llama.cpp CPU macOS x64 ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::MacOS,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-x64.tar.gz", latest_version, latest_version),
                    file_size: 82 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 85.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                })
            }
            (Platform::Linux, HardwareArchitecture::X86_64, true, _) => {
                
                Some(EngineInfo {
                    id: format!("llama-cuda-linux-x64-{}", latest_version),
                    name: format!("llama.cpp CUDA Linux x64 ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Linux,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CUDA,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-ubuntu-x64-cuda-12.4.tar.gz", latest_version, latest_version), 
                    file_size: 180 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 90.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec!["CUDA 12.4+ Runtime".to_string()],
                })
            }
            (Platform::Linux, HardwareArchitecture::X86_64, false, _) => {
                
                Some(EngineInfo {
                    id: format!("llama-cpu-linux-x64-{}", latest_version),
                    name: format!("llama.cpp CPU Linux x64 ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Linux,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-ubuntu-x64.tar.gz", latest_version, latest_version),
                    file_size: 45 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 80.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                })
            }
            _ => None,
        }
    }
    
    pub fn set_default_engine(&mut self, engine_id: &str) -> Result<()> {
        if self.installed_engines.contains_key(engine_id) {
            self.default_engine = Some(engine_id.to_string());
            info!("Set default engine: {}", engine_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Engine not found: {}", engine_id))
        }
    }

    pub fn has_installed_engine(&self) -> bool {
        !self.installed_engines.is_empty()
    }

    pub fn get_default_engine(&self) -> Option<&EngineInfo> {
        if let Some(ref engine_id) = self.default_engine {
            self.installed_engines.get(engine_id)
        } else {
            
            self.installed_engines.values().next()
        }
    }
    
    pub async fn add_installed_engine(&mut self, mut engine: EngineInfo) -> Result<()> {
        engine.status = EngineStatus::Installed;
        self.installed_engines.insert(engine.id.clone(), engine);
        Ok(())
    }
    
    pub async fn refresh_available_engines(&mut self, hardware_capabilities: &HardwareCapabilities) -> Result<()> {
        
        self.available_engines.clear();
        
        let all_engines = self.get_all_compatible_engines(hardware_capabilities);
        
        for engine in all_engines {
            if !self.available_engines.iter().any(|e| e.id == engine.id) {
                self.available_engines.push(engine);
            }
        }
        
        let fallback_engines = self.get_additional_engine_recommendations(hardware_capabilities);
        for engine in fallback_engines {
            if !self.available_engines.iter().any(|e| e.id == engine.id) {
                self.available_engines.push(engine);
            }
        }
        
        info!("Refreshed available engines: {} found", self.available_engines.len());
        Ok(())
    }
    
    fn get_all_compatible_engines(&self, hardware: &HardwareCapabilities) -> Vec<EngineInfo> {
        let mut engines = Vec::new();
        let latest_version = self.get_latest_version();
        
        match (&hardware.platform, &hardware.architecture) {
            (Platform::Windows, HardwareArchitecture::X86_64) => {
                
                engines.push(EngineInfo {
                    id: format!("llama-cpu-windows-x64-{}", latest_version),
                    name: format!("llama.cpp CPU (Windows x64) ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Windows,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-x64.zip", latest_version, latest_version),
                    file_size: 50 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: if !hardware.has_cuda { 95.0 } else { 80.0 },
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server.exe".to_string(),
                    required_dependencies: vec![],
                });
                
                if hardware.has_cuda {
                    engines.push(EngineInfo {
                        id: format!("llama-cuda-windows-x64-{}", latest_version),
                        name: format!("llama.cpp CUDA (Windows x64) ({})", latest_version),
                        version: latest_version.to_string(),
                        platform: Platform::Windows,
                        architecture: HardwareArchitecture::X86_64,
                        acceleration: AccelerationType::CUDA,
                        download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-cuda-12.4-x64.zip", latest_version, latest_version),
                        file_size: 373 * 1024 * 1024, 
                        checksum: "".to_string(),
                        compatibility_score: 100.0,
                        status: EngineStatus::Available,
                        install_path: None,
                        binary_name: "llama-server.exe".to_string(),
                        required_dependencies: vec!["NVIDIA GPU with CUDA support".to_string()],
                    });
                    
                    engines.push(EngineInfo {
                        id: format!("llama-cuda13-windows-x64-{}", latest_version),
                        name: format!("llama.cpp CUDA 13 (Windows x64) ({})", latest_version),
                        version: latest_version.to_string(),
                        platform: Platform::Windows,
                        architecture: HardwareArchitecture::X86_64,
                        acceleration: AccelerationType::CUDA,
                        download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-cuda-13.1-x64.zip", latest_version, latest_version),
                        file_size: 384 * 1024 * 1024, 
                        checksum: "".to_string(),
                        compatibility_score: 95.0,
                        status: EngineStatus::Available,
                        install_path: None,
                        binary_name: "llama-server.exe".to_string(),
                        required_dependencies: vec!["CUDA 13.1+ Runtime".to_string()],
                    });
                }
                
                if hardware.has_vulkan || hardware.has_cuda {
                    engines.push(EngineInfo {
                        id: format!("llama-vulkan-windows-x64-{}", latest_version),
                        name: format!("llama.cpp Vulkan (Windows x64) ({})", latest_version),
                        version: latest_version.to_string(),
                        platform: Platform::Windows,
                        architecture: HardwareArchitecture::X86_64,
                        acceleration: AccelerationType::Vulkan,
                        download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-vulkan-x64.zip", latest_version, latest_version),
                        file_size: 80 * 1024 * 1024,
                        checksum: "".to_string(),
                        compatibility_score: 85.0,
                        status: EngineStatus::Available,
                        install_path: None,
                        binary_name: "llama-server.exe".to_string(),
                        required_dependencies: vec!["Vulkan-compatible GPU".to_string()],
                    });
                }
            }
            
            (Platform::MacOS, HardwareArchitecture::Aarch64) => {
                
                engines.push(EngineInfo {
                    id: format!("llama-metal-macos-arm64-{}", latest_version),
                    name: format!("llama.cpp Metal (macOS Apple Silicon) ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::MacOS,
                    architecture: HardwareArchitecture::Aarch64,
                    acceleration: AccelerationType::Metal,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-arm64.tar.gz", latest_version, latest_version),
                    file_size: 29 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 100.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                });
                
                engines.push(EngineInfo {
                    id: format!("llama-cpu-macos-arm64-{}", latest_version),
                    name: format!("llama.cpp CPU (macOS Apple Silicon) ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::MacOS,
                    architecture: HardwareArchitecture::Aarch64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-arm64.tar.gz", latest_version, latest_version),
                    file_size: 29 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 90.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                });
            }
            
            (Platform::MacOS, HardwareArchitecture::X86_64) => {
                
                engines.push(EngineInfo {
                    id: format!("llama-cpu-macos-x64-{}", latest_version),
                    name: format!("llama.cpp CPU (macOS Intel) ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::MacOS,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-x64.tar.gz", latest_version, latest_version),
                    file_size: 82 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 95.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                });
            }
            
            (Platform::Linux, HardwareArchitecture::X86_64) => {
                
                engines.push(EngineInfo {
                    id: format!("llama-cpu-linux-x64-{}", latest_version),
                    name: format!("llama.cpp CPU (Linux x64) ({})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Linux,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-ubuntu-x64.tar.gz", latest_version, latest_version),
                    file_size: 45 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: if !hardware.has_cuda { 95.0 } else { 80.0 },
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                });
                
                if hardware.has_cuda {
                    engines.push(EngineInfo {
                        id: format!("llama-cuda-linux-x64-{}", latest_version),
                        name: format!("llama.cpp CUDA (Linux x64) ({})", latest_version),
                        version: latest_version.to_string(),
                        platform: Platform::Linux,
                        architecture: HardwareArchitecture::X86_64,
                        acceleration: AccelerationType::CUDA,
                        download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-ubuntu-x64-cuda-12.4.tar.gz", latest_version, latest_version),
                        file_size: 180 * 1024 * 1024,
                        checksum: "".to_string(),
                        compatibility_score: 100.0,
                        status: EngineStatus::Available,
                        install_path: None,
                        binary_name: "llama-server".to_string(),
                        required_dependencies: vec!["NVIDIA GPU with CUDA support".to_string()],
                    });
                }
            }
            
            _ => {
                
                info!("Unknown platform/architecture: {:?}/{:?}", hardware.platform, hardware.architecture);
            }
        }
        
        engines.sort_by(|a, b| b.compatibility_score.partial_cmp(&a.compatibility_score).unwrap());
        
        engines
    }
    
    fn get_additional_engine_recommendations(&self, hardware: &HardwareCapabilities) -> Vec<EngineInfo> {
        let mut engines = Vec::new();
        let latest_version = self.get_latest_version();
        
        match &hardware.platform {
            Platform::Windows => {
                engines.push(EngineInfo {
                    id: format!("llama-cpu-windows-x64-fallback-{}", latest_version),
                    name: format!("llama.cpp CPU Engine (Fallback {}) ", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Windows,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-win-x64.zip", latest_version, latest_version),
                    file_size: 50 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 70.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server.exe".to_string(),
                    required_dependencies: vec![],
                });
            }
            Platform::Linux => {
                engines.push(EngineInfo {
                    id: format!("llama-cpu-linux-x64-fallback-{}", latest_version),
                    name: format!("llama.cpp CPU Engine (Fallback {})", latest_version),
                    version: latest_version.to_string(),
                    platform: Platform::Linux,
                    architecture: HardwareArchitecture::X86_64,
                    acceleration: AccelerationType::CPU,
                    download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-ubuntu-x64.tar.gz", latest_version, latest_version),
                    file_size: 45 * 1024 * 1024,
                    checksum: "".to_string(),
                    compatibility_score: 70.0,
                    status: EngineStatus::Available,
                    install_path: None,
                    binary_name: "llama-server".to_string(),
                    required_dependencies: vec![],
                });
            }
            Platform::MacOS => {
                
                match &hardware.architecture {
                    HardwareArchitecture::Aarch64 => {
                        engines.push(EngineInfo {
                            id: format!("llama-metal-macos-arm64-fallback-{}", latest_version),
                            name: format!("llama.cpp Metal (Apple Silicon) fallback ({})", latest_version),
                            version: latest_version.to_string(),
                            platform: Platform::MacOS,
                            architecture: HardwareArchitecture::Aarch64,
                            acceleration: AccelerationType::Metal,
                            download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-arm64.tar.gz", latest_version, latest_version),
                            file_size: 29 * 1024 * 1024,
                            checksum: "".to_string(),
                            compatibility_score: 85.0,
                            status: EngineStatus::Available,
                            install_path: None,
                            binary_name: "llama-server".to_string(),
                            required_dependencies: vec![],
                        });
                        
                        engines.push(EngineInfo {
                            id: format!("llama-cpu-macos-arm64-fallback-{}", latest_version),
                            name: format!("llama.cpp CPU (Apple Silicon) fallback ({})", latest_version),
                            version: latest_version.to_string(),
                            platform: Platform::MacOS,
                            architecture: HardwareArchitecture::Aarch64,
                            acceleration: AccelerationType::CPU,
                            download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-arm64.tar.gz", latest_version, latest_version),
                            file_size: 29 * 1024 * 1024,
                            checksum: "".to_string(),
                            compatibility_score: 70.0,
                            status: EngineStatus::Available,
                            install_path: None,
                            binary_name: "llama-server".to_string(),
                            required_dependencies: vec![],
                        });
                    }
                    _ => {
                        
                        engines.push(EngineInfo {
                            id: format!("llama-cpu-macos-x64-fallback-{}", latest_version),
                            name: format!("llama.cpp CPU (macOS Intel) fallback ({})", latest_version),
                            version: latest_version.to_string(),
                            platform: Platform::MacOS,
                            architecture: HardwareArchitecture::X86_64,
                            acceleration: AccelerationType::CPU,
                            download_url: format!("https://github.com/ggml-org/llama.cpp/releases/download/{}/llama-{}-bin-macos-x64.tar.gz", latest_version, latest_version),
                            file_size: 82 * 1024 * 1024,
                            checksum: "".to_string(),
                            compatibility_score: 70.0,
                            status: EngineStatus::Available,
                            install_path: None,
                            binary_name: "llama-server".to_string(),
                            required_dependencies: vec![],
                        });
                    }
                }
            }
        }
        
        engines
    }
    
    pub fn get_default_engine_binary_path(&self) -> Option<PathBuf> {
        if let Some(engine) = self.get_default_engine() {
            engine.install_path.as_ref().map(|path| path.join(&engine.binary_name))
        } else {
            None
        }
    }

    pub fn mark_download_started(&self) -> bool {
        let mut flag = self.download_in_progress.write().unwrap();
        if *flag {
            false 
        } else {
            *flag = true;
            true 
        }
    }

    pub fn mark_download_finished(&self) {
        *self.download_in_progress.write().unwrap() = false;
    }

    pub fn is_download_in_progress(&self) -> bool {
        *self.download_in_progress.read().unwrap()
    }
}
