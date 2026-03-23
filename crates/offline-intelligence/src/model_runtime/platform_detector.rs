//! Platform and Hardware Detection
//!
//! Detects the appropriate runtime binary based on the platform (Windows, Linux, macOS)
//! and hardware capabilities (Intel, Apple Silicon, NVIDIA CUDA).

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
    Aarch64, // Apple Silicon, ARM
    Other(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardwareCapabilities {
    pub platform: Platform,
    pub architecture: HardwareArchitecture,
    pub has_cuda: bool,
    pub has_metal: bool, // For Apple GPUs
    pub has_vulkan: bool,
}

// Static cache for hardware capabilities to avoid repeated detection
static HARDWARE_CACHE: OnceLock<HardwareCapabilities> = OnceLock::new();

impl Default for HardwareCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

impl HardwareCapabilities {
    /// Detect hardware capabilities automatically (cached)
    pub fn detect() -> Self {
        // Return cached result if available
        if let Some(cached) = HARDWARE_CACHE.get() {
            return cached.clone();
        }

        // Perform detection
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

        // Cache the result (ignore if already set by another thread)
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
            // Default to current platform if unknown
            #[cfg(target_os = "windows")]
            return Platform::Windows;
            #[cfg(target_os = "linux")]
            return Platform::Linux;
            #[cfg(target_os = "macos")]
            return Platform::MacOS;
            #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
            return Platform::Linux; // Default fallback
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
        // Check for NVIDIA GPU via nvidia-smi with a timeout to prevent hangs
        // on systems with broken driver installations
        use std::process::{Command, Stdio};

        // Create command with hidden window on Windows
        #[cfg(target_os = "windows")]
        let child = {
            use std::os::windows::process::CommandExt;
            Command::new("nvidia-smi")
                .arg("--query-gpu=name")
                .arg("--format=csv,noheader,nounits")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
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
                // Wait up to 5 seconds for nvidia-smi to respond
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

    /// Metal is available on all Apple Silicon Macs and on Intel Macs with a
    /// compatible GPU (all Macs from ~2012 onward support Metal).
    /// However, llama.cpp Metal acceleration is only effective on Apple Silicon
    /// where unified memory is shared between CPU and GPU.  We return `true`
    /// for all macOS targets so the engine registry correctly recommends the
    /// Metal-enabled binary; gpu_layers is set to 0 for Intel in config.rs.
    fn detect_metal_support(architecture: &HardwareArchitecture) -> bool {
        if cfg!(target_os = "macos") {
            // Both Apple Silicon and Intel Mac support Metal API
            // (config.rs sets gpu_layers=0 for Intel to keep CPU-only inference)
            let _ = architecture; // suppress unused-variable warning
            true
        } else {
            false
        }
    }

    fn detect_vulkan_support() -> bool {
        false
    }

    /// Get the appropriate runtime binary path based on platform and hardware.
    ///
    /// Directory naming uses the **exact capitalisation** as stored on disk
    /// (`Windows`, `MacOS`, `Linux`) to match what `config.rs` expects and
    /// what is present in the repo's `Resources/bin/` tree.
    pub fn get_runtime_binary_path(&self) -> Option<PathBuf> {
        let resources_dir = self.get_resources_dir()?;

        // Use the canonical, mixed-case folder name that matches the on-disk
        // directory — do NOT call .to_lowercase() here, because on case-sensitive
        // APFS that would silently break the lookup.
        let os_folder = match &self.platform {
            Platform::Windows => "Windows",
            Platform::MacOS   => "MacOS",
            Platform::Linux   => "Linux",
        };
        let platform_dir = resources_dir.join(os_folder);

        match &self.platform {
            Platform::Windows => {
                // On Windows, prefer CUDA if available, otherwise use CPU
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
                // On Linux, return None to use the config.rs / engine-registry path
                None
            }
            Platform::MacOS => {
                // On macOS, use Metal-compiled binary for both Apple Silicon and Intel.
                // llama.cpp's macOS release builds always include Metal support;
                // gpu_layers is controlled at runtime (0 for Intel, >0 for AS).
                if self.has_metal {
                    Some(platform_dir.join("llama-metal").join("llama-server"))
                } else {
                    Some(platform_dir.join("llama-cpu").join("llama-server"))
                }
            }
        }
    }

    /// Locate the `Resources/bin` directory.
    ///
    /// Search order (most to least specific):
    ///  1. macOS .app bundle standard: `<exe>/../Resources/bin`
    ///     i.e. `App.app/Contents/Resources/bin`
    ///  2. Sibling `Resources/bin` next to the executable
    ///  3. Current working directory `Resources/bin`
    ///  4. Development path relative to crate root
    fn get_resources_dir(&self) -> Option<PathBuf> {
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                // --- (1) macOS .app bundle ---
                // exe_dir = App.app/Contents/MacOS/
                // parent  = App.app/Contents/
                // Resources live at App.app/Contents/Resources/
                if let Some(contents_dir) = exe_dir.parent() {
                    let candidate = contents_dir.join("Resources").join("bin");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }

                // --- (2) Resources/ sibling to executable ---
                // (Linux AppImage, dev `cargo run` from workspace root, etc.)
                for resource_folder in &["Resources", "resources"] {
                    let candidate = exe_dir.join(resource_folder).join("bin");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }

        // --- (3) CWD-relative ---
        for resource_folder in &["Resources", "resources"] {
            let candidate = std::path::PathBuf::from(resource_folder).join("bin");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        // --- (4) Development: relative to crate manifest ---
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
