
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::info;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HardwareArchitecture {
    X86_64,
    Aarch64, 
    Other(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardwareCapabilities {
    pub platform: Platform,
    pub architecture: HardwareArchitecture,
    pub has_cuda: bool,
    pub has_metal: bool, 
    pub has_vulkan: bool,
}

static HARDWARE_CACHE: OnceLock<HardwareCapabilities> = OnceLock::new();

impl Default for HardwareCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

impl HardwareCapabilities {
    
    pub fn detect() -> Self {
        
        if let Some(cached) = HARDWARE_CACHE.get() {
            return cached.clone();
        }

        let platform = Self::detect_platform();
        let architecture = Self::detect_architecture();
        let has_cuda = Self::detect_cuda_support();
        let has_metal = Self::detect_metal_support(&architecture);
        let has_vulkan = Self::detect_vulkan_support();

        info!(
            "Detected platform: {:?}, architecture: {:?}, CUDA: {}, Metal: {}, Vulkan: {}",
            platform, architecture, has_cuda, has_metal, has_vulkan
        );

        let capabilities = Self {
            platform,
            architecture,
            has_cuda,
            has_metal,
            has_vulkan,
        };

        let _ = HARDWARE_CACHE.set(capabilities.clone());

        capabilities
    }

    fn detect_platform() -> Platform {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else {
            
            #[cfg(target_os = "windows")]
            return Platform::Windows;
            #[cfg(target_os = "linux")]
            return Platform::Linux;
            #[cfg(target_os = "macos")]
            return Platform::MacOS;
            #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
            return Platform::Linux; 
        }
    }

    fn detect_architecture() -> HardwareArchitecture {
        if cfg!(target_arch = "x86_64") {
            HardwareArchitecture::X86_64
        } else if cfg!(target_arch = "aarch64") {
            HardwareArchitecture::Aarch64
        } else {
            HardwareArchitecture::Other(std::env::consts::ARCH.to_string())
        }
    }

    fn detect_cuda_support() -> bool {
        
        use std::process::{Command, Stdio};

        #[cfg(target_os = "windows")]
        let child = {
            use std::os::windows::process::CommandExt;
            Command::new("nvidia-smi")
                .arg("--query-gpu=name")
                .arg("--format=csv,noheader,nounits")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(0x08000000) 
                .spawn()
        };

        #[cfg(not(target_os = "windows"))]
        let child = Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader,nounits")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(mut process) => {
                
                let start = std::time::Instant::now();
                loop {
                    match process.try_wait() {
                        Ok(Some(status)) => return status.success(),
                        Ok(None) => {
                            if start.elapsed() > std::time::Duration::from_secs(5) {
                                let _ = process.kill();
                                let _ = process.wait();
                                return false;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                        Err(_) => return false,
                    }
                }
            }
            Err(_) => false,
        }
    }

    fn detect_metal_support(architecture: &HardwareArchitecture) -> bool {
        if cfg!(target_os = "macos") {
            
            let _ = architecture; 
            true
        } else {
            false
        }
    }

    fn detect_vulkan_support() -> bool {
        false
    }

    pub fn get_runtime_binary_path(&self) -> Option<PathBuf> {
        let resources_dir = self.get_resources_dir()?;

        let os_folder = match &self.platform {
            Platform::Windows => "Windows",
            Platform::MacOS   => "MacOS",
            Platform::Linux   => "Linux",
        };
        let platform_dir = resources_dir.join(os_folder);

        match &self.platform {
            Platform::Windows => {
                
                if self.has_cuda {
                    Some(
                        platform_dir
                            .join("llama-b6970-bin-win-cuda-12.4-x64")
                            .join("llama-server.exe"),
                    )
                } else {
                    Some(platform_dir.join("llama-cpu").join("llama-server.exe"))
                }
            }
            Platform::Linux => {
                
                None
            }
            Platform::MacOS => {
                
                if self.has_metal {
                    Some(platform_dir.join("llama-metal").join("llama-server"))
                } else {
                    Some(platform_dir.join("llama-cpu").join("llama-server"))
                }
            }
        }
    }

    fn get_resources_dir(&self) -> Option<PathBuf> {
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                
                if let Some(contents_dir) = exe_dir.parent() {
                    let candidate = contents_dir.join("Resources").join("bin");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }

                for resource_folder in &["Resources", "resources"] {
                    let candidate = exe_dir.join(resource_folder).join("bin");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }

        for resource_folder in &["Resources", "resources"] {
            let candidate = std::path::PathBuf::from(resource_folder).join("bin");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        #[cfg(debug_assertions)]
        {
            let dev_candidate = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("Resources")
                .join("bin");
            if dev_candidate.exists() {
                return Some(dev_candidate);
            }
        }

        None
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Windows => write!(f, "Windows"),
            Platform::Linux => write!(f, "Linux"),
            Platform::MacOS => write!(f, "MacOS"),
        }
    }
}

impl std::fmt::Display for HardwareArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareArchitecture::X86_64 => write!(f, "x86_64"),
            HardwareArchitecture::Aarch64 => write!(f, "aarch64"),
            HardwareArchitecture::Other(s) => write!(f, "{}", s),
        }
    }
}
