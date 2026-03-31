
use crate::model_runtime::platform_detector::{HardwareCapabilities, Platform, HardwareArchitecture};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub platform: Platform,
    pub architecture: HardwareArchitecture,
    pub cpu_cores: u32,
    pub total_memory_gb: f32,
    pub available_memory_gb: f32,
    pub gpu_info: Option<GPUInfo>,
    pub acceleration_support: HashMap<String, bool>,
    pub system_info: SystemInfo,
}

static PROFILE_CACHE: OnceLock<HardwareProfile> = OnceLock::new();

impl HardwareProfile {
    
    pub async fn analyze(capabilities: &HardwareCapabilities) -> Result<Self, Box<dyn std::error::Error>> {
        
        if let Some(cached) = PROFILE_CACHE.get() {
            return Ok(cached.clone());
        }
        
        let analyzer = HardwareAnalyzer::new(capabilities.clone());
        let profile = analyzer.get_hardware_profile();
        
        let _ = PROFILE_CACHE.set(profile.clone());
        
        Ok(profile)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    pub vendor: String,
    pub model: String,
    pub memory_gb: f32,
    pub compute_capability: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: Option<String>,
}

pub struct HardwareAnalyzer {
    capabilities: HardwareCapabilities,
}

impl HardwareAnalyzer {
    pub fn new(capabilities: HardwareCapabilities) -> Self {
        Self { capabilities }
    }

    pub fn get_hardware_profile(&self) -> HardwareProfile {
        let system_info = self.detect_system_info();
        let gpu_info = self.detect_gpu_info();
        let memory_info = self.detect_memory_info();
        let acceleration_support = self.detect_acceleration_support(&gpu_info);

        HardwareProfile {
            platform: self.capabilities.platform.clone(),
            architecture: self.capabilities.architecture.clone(),
            cpu_cores: self.detect_cpu_cores(),
            total_memory_gb: memory_info.total_gb,
            available_memory_gb: memory_info.available_gb,
            gpu_info,
            acceleration_support,
            system_info,
        }
    }

    fn detect_cpu_cores(&self) -> u32 {
        let logical_cores = num_cpus::get() as u32;
        debug!("Detected {} logical CPU cores", logical_cores);
        logical_cores
    }

    fn detect_memory_info(&self) -> MemoryInfo {
        let mut system = sysinfo::System::new_all();
        system.refresh_memory();
        
        let total_bytes = system.total_memory();
        let available_bytes = system.available_memory();
        
        let total_gb = total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let available_gb = available_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        
        debug!("Memory: {:.1}GB total, {:.1}GB available", total_gb, available_gb);
        
        MemoryInfo {
            total_gb,
            available_gb,
        }
    }

    fn detect_gpu_info(&self) -> Option<GPUInfo> {
        match &self.capabilities.platform {
            Platform::Windows => self.detect_windows_gpu(),
            Platform::Linux => self.detect_linux_gpu(),
            Platform::MacOS => self.detect_macos_gpu(),
        }
    }

    fn detect_windows_gpu(&self) -> Option<GPUInfo> {
        #[cfg(target_os = "windows")]
        {
            use std::process::{Command, Stdio};
            use std::os::windows::process::CommandExt;

            let child = Command::new("powershell")
                .args([
                    "-NoProfile", "-NonInteractive", "-Command",
                    "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(0x08000000) 
                .spawn();

            if let Ok(mut process) = child {
                let start = std::time::Instant::now();
                loop {
                    match process.try_wait() {
                        Ok(Some(status)) => {
                            if status.success() {
                                if let Ok(output) = process.wait_with_output() {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    for line in stdout.lines() {
                                        let trimmed = line.trim();
                                        if trimmed.contains("NVIDIA") {
                                            return Some(GPUInfo {
                                                vendor: "NVIDIA".to_string(),
                                                model: trimmed.to_string(),
                                                memory_gb: 0.0,
                                                compute_capability: None,
                                                driver_version: None,
                                            });
                                        }
                                    }
                                }
                            }
                            break;
                        }
                        Ok(None) => {
                            if start.elapsed() > std::time::Duration::from_secs(10) {
                                let _ = process.kill();
                                let _ = process.wait();
                                debug!("GPU detection via PowerShell timed out");
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        if self.capabilities.has_cuda {
            Some(GPUInfo {
                vendor: "NVIDIA".to_string(),
                model: "Unknown CUDA GPU".to_string(),
                memory_gb: 0.0,
                compute_capability: None,
                driver_version: None,
            })
        } else {
            None
        }
    }

    fn detect_linux_gpu(&self) -> Option<GPUInfo> {
        use std::process::{Command, Stdio};

        let child = Command::new("lspci")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        if let Ok(mut process) = child {
            let start = std::time::Instant::now();
            loop {
                match process.try_wait() {
                    Ok(Some(_)) => {
                        if let Ok(output) = process.wait_with_output() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            for line in stdout.lines() {
                                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                                    if line.contains("NVIDIA") {
                                        return Some(GPUInfo {
                                            vendor: "NVIDIA".to_string(),
                                            model: line.split(": ").nth(1).unwrap_or("Unknown").to_string(),
                                            memory_gb: 0.0,
                                            compute_capability: None,
                                            driver_version: None,
                                        });
                                    } else if line.contains("AMD") || line.contains("ATI") {
                                        return Some(GPUInfo {
                                            vendor: "AMD".to_string(),
                                            model: line.split(": ").nth(1).unwrap_or("Unknown").to_string(),
                                            memory_gb: 0.0,
                                            compute_capability: None,
                                            driver_version: None,
                                        });
                                    }
                                }
                            }
                        }
                        break;
                    }
                    Ok(None) => {
                        if start.elapsed() > std::time::Duration::from_secs(5) {
                            let _ = process.kill();
                            let _ = process.wait();
                            debug!("lspci GPU detection timed out");
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        }

        None
    }

    fn detect_macos_gpu(&self) -> Option<GPUInfo> {
        #[cfg(target_os = "macos")]
        {
            
            if self.capabilities.has_metal {
                return Some(GPUInfo {
                    vendor: "Apple".to_string(),
                    model: "Integrated GPU".to_string(),
                    memory_gb: 0.0,
                    compute_capability: Some("Metal".to_string()),
                    driver_version: None,
                });
            }
        }
        
        None
    }

    fn detect_acceleration_support(&self, _gpu_info: &Option<GPUInfo>) -> HashMap<String, bool> {
        let mut support = HashMap::new();
        
        support.insert("cpu".to_string(), true);
        
        match &self.capabilities.platform {
            Platform::Windows => {
                support.insert("cuda".to_string(), self.capabilities.has_cuda);
                support.insert("directml".to_string(), true); 
            }
            Platform::MacOS => {
                support.insert("metal".to_string(), self.capabilities.has_metal);
            }
            Platform::Linux => {
                support.insert("cuda".to_string(), self.capabilities.has_cuda);
                support.insert("vulkan".to_string(), self.capabilities.has_vulkan);
                
                support.insert("rocm".to_string(), self.detect_rocm_support());
            }
        }
        
        support.insert("opencl".to_string(), self.detect_opencl_support());
        
        debug!("Acceleration support: {:?}", support);
        support
    }

    fn detect_rocm_support(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::path::Path;
            
            Path::new("/opt/rocm").exists() || Path::new("/usr/lib/rocm").exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn detect_opencl_support(&self) -> bool {
        
        match &self.capabilities.platform {
            Platform::Windows => {
                
                std::path::Path::new("C:\\Windows\\System32\\OpenCL.dll").exists()
            }
            Platform::Linux => {
                std::path::Path::new("/usr/lib/libOpenCL.so").exists() ||
                std::path::Path::new("/usr/lib/x86_64-linux-gnu/libOpenCL.so").exists()
            }
            Platform::MacOS => {
                
                true
            }
        }
    }

    fn detect_system_info(&self) -> SystemInfo {
        SystemInfo {
            os_name: std::env::consts::OS.to_string(),
            os_version: "Unknown".to_string(),
            kernel_version: None,
        }
    }

    pub fn get_engine_recommendations(&self) -> Vec<EngineRecommendation> {
        let profile = self.get_hardware_profile();
        let mut recommendations = Vec::new();
        
        recommendations.push(EngineRecommendation {
            engine_type: "CPU".to_string(),
            priority: 1,
            reason: "Universal compatibility".to_string(),
            estimated_performance: self.estimate_cpu_performance(&profile),
        });
        
        if profile.acceleration_support.get("cuda").copied().unwrap_or(false) {
            recommendations.push(EngineRecommendation {
                engine_type: "CUDA".to_string(),
                priority: 2,
                reason: "NVIDIA GPU acceleration available".to_string(),
                estimated_performance: self.estimate_cuda_performance(&profile),
            });
        }
        
        if profile.acceleration_support.get("metal").copied().unwrap_or(false) {
            recommendations.push(EngineRecommendation {
                engine_type: "Metal".to_string(),
                priority: 2,
                reason: "Apple Silicon GPU acceleration".to_string(),
                estimated_performance: self.estimate_metal_performance(&profile),
            });
        }
        
        if profile.acceleration_support.get("vulkan").copied().unwrap_or(false) {
            recommendations.push(EngineRecommendation {
                engine_type: "Vulkan".to_string(),
                priority: 3,
                reason: "Cross-platform GPU acceleration".to_string(),
                estimated_performance: self.estimate_vulkan_performance(&profile),
            });
        }
        
        recommendations.sort_by(|a, b| a.priority.cmp(&b.priority));
        recommendations
    }

    fn estimate_cpu_performance(&self, profile: &HardwareProfile) -> PerformanceEstimate {
        let core_multiplier = profile.cpu_cores as f32;
        let memory_multiplier = (profile.available_memory_gb / 8.0).min(4.0);
        let base_score = 50.0;
        
        PerformanceEstimate {
            inference_speed: base_score * core_multiplier * memory_multiplier * 0.1,
            memory_efficiency: (profile.available_memory_gb / profile.total_memory_gb) * 100.0,
            power_efficiency: 70.0, 
        }
    }

    fn estimate_cuda_performance(&self, profile: &HardwareProfile) -> PerformanceEstimate {
        if let Some(gpu) = &profile.gpu_info {
            let memory_score = gpu.memory_gb * 10.0; 
            let base_score = 80.0;
            
            PerformanceEstimate {
                inference_speed: base_score + memory_score,
                memory_efficiency: 90.0, 
                power_efficiency: 85.0, 
            }
        } else {
            PerformanceEstimate {
                inference_speed: 0.0,
                memory_efficiency: 0.0,
                power_efficiency: 0.0,
            }
        }
    }

    fn estimate_metal_performance(&self, _profile: &HardwareProfile) -> PerformanceEstimate {
        
        PerformanceEstimate {
            inference_speed: 75.0,
            memory_efficiency: 95.0, 
            power_efficiency: 90.0, 
        }
    }

    fn estimate_vulkan_performance(&self, _profile: &HardwareProfile) -> PerformanceEstimate {
        
        PerformanceEstimate {
            inference_speed: 70.0,
            memory_efficiency: 85.0,
            power_efficiency: 80.0,
        }
    }
}

#[derive(Debug)]
struct MemoryInfo {
    total_gb: f32,
    available_gb: f32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EngineRecommendation {
    pub engine_type: String,
    pub priority: u32,
    pub reason: String,
    pub estimated_performance: PerformanceEstimate,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PerformanceEstimate {
    pub inference_speed: f32,      
    pub memory_efficiency: f32,    
    pub power_efficiency: f32,     
}
