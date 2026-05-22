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

/// Detected CUDA toolkit version.  Parsed from `nvidia-smi` output so we can
/// select the engine binary that matches the installed CUDA runtime.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CudaVersion {
    /// CUDA 12.4 runtime (matches `cudart64_12.dll` / `libcudart.so.12.4`)
    Cuda124,
    /// CUDA 13.1 runtime
    Cuda131,
    /// Any other version — use the 12.4 engine as the safest default
    Other(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardwareCapabilities {
    pub platform: Platform,
    pub architecture: HardwareArchitecture,
    pub has_cuda: bool,
    pub has_metal: bool,
    pub has_vulkan: bool,
    /// True when an AMD GPU is detected (Windows or Linux).
    /// On Windows this enables the Vulkan engine build.
    /// On Linux this may enable the ROCm engine build if has_rocm is also true.
    pub has_amd_gpu: bool,
    /// True when the AMD ROCm compute stack is installed (Linux only).
    pub has_rocm: bool,

    // ── Intel GPU family ────────────────────────────────────────────────────
    /// True when any Intel GPU is present (UHD, Iris Xe, Arc, Gaudi).
    pub has_intel_gpu: bool,
    /// True when a discrete Intel Arc GPU is detected.
    /// On Windows this selects the SYCL engine; on Linux the Vulkan engine.
    pub has_intel_arc: bool,
    /// True when a Habana/Intel Gaudi HPU is detected (Linux only via lspci).
    /// The standard llama.cpp release does not yet have a Gaudi build;
    /// this flag is recorded so callers can surface appropriate guidance.
    pub has_intel_gaudi: bool,
    /// True when the Intel Level Zero runtime loader is installed.
    /// Required for the SYCL (oneAPI) engine build on Windows.
    pub has_level_zero: bool,

    // ── AMD Windows ─────────────────────────────────────────────────────────
    /// True when `amdhip64.dll` is present in System32 (Windows only).
    /// Installed automatically with AMD Radeon graphics drivers.
    /// Required for the HIP/Radeon engine build on Windows.
    pub has_amd_hip_windows: bool,

    // ── CUDA version ────────────────────────────────────────────────────────
    /// Detected CUDA runtime version, parsed from `nvidia-smi`.
    /// `None` when CUDA is not present.
    pub cuda_version: Option<CudaVersion>,

    /// True when running on Apple Silicon (M1 / M2 / M3 / M4 and later).
    /// False on Intel Mac, Windows, and Linux.
    ///
    /// Convenience field — equivalent to checking
    /// `platform == MacOS && architecture == Aarch64`, but readable at every
    /// call site without pattern-matching two fields.
    pub has_apple_silicon: bool,
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
        let cuda_version = Self::detect_cuda_version();
        let has_cuda = cuda_version.is_some();
        let has_metal = Self::detect_metal_support(&architecture);
        let has_amd_gpu = Self::detect_amd_gpu(&platform);
        let has_rocm = Self::detect_rocm_support(&platform);
        let has_vulkan = Self::detect_vulkan_support(&platform);
        let has_intel_gpu = Self::detect_intel_gpu(&platform);
        let has_intel_arc = Self::detect_intel_arc(&platform);
        let has_intel_gaudi = Self::detect_intel_gaudi(&platform);
        let has_level_zero = Self::detect_level_zero(&platform);
        let has_amd_hip_windows = Self::detect_amd_hip_windows(&platform);
        let has_apple_silicon = cfg!(all(target_os = "macos", target_arch = "aarch64"));

        info!(
            "Detected platform: {:?}, arch: {:?}, AppleSilicon={}, \
             CUDA={} ({:?}), Metal={}, Vulkan={}, \
             AMD={}, ROCm={}, AMDHipWin={}, \
             IntelGPU={}, IntelArc={}, IntelGaudi={}, LevelZero={}",
            platform, architecture, has_apple_silicon,
            has_cuda, cuda_version, has_metal, has_vulkan,
            has_amd_gpu, has_rocm, has_amd_hip_windows,
            has_intel_gpu, has_intel_arc, has_intel_gaudi, has_level_zero
        );

        let capabilities = Self {
            platform,
            architecture,
            has_cuda,
            has_metal,
            has_vulkan,
            has_amd_gpu,
            has_rocm,
            has_intel_gpu,
            has_intel_arc,
            has_intel_gaudi,
            has_level_zero,
            has_amd_hip_windows,
            cuda_version,
            has_apple_silicon,
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

    /// Detect CUDA availability AND version by running `nvidia-smi`.
    ///
    /// Returns `None` when no NVIDIA GPU / driver is found.
    /// Returns `Some(CudaVersion::…)` when CUDA is available, with the version
    /// parsed from the "CUDA Version: X.Y" header line in `nvidia-smi` output.
    fn detect_cuda_version() -> Option<CudaVersion> {
        use std::process::{Command, Stdio};

        #[cfg(target_os = "windows")]
        let child = {
            use std::os::windows::process::CommandExt;
            Command::new("nvidia-smi")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
        };
        #[cfg(not(target_os = "windows"))]
        let child = Command::new("nvidia-smi")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut process = child.ok()?;
        let start = std::time::Instant::now();
        loop {
            match process.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return None;
                    }
                    // Parse "CUDA Version: 12.4" from stdout
                    if let Ok(out) = process.wait_with_output() {
                        let text = String::from_utf8_lossy(&out.stdout);
                        for line in text.lines() {
                            if line.contains("CUDA Version:") {
                                // e.g. "| CUDA Version: 12.4     |"
                                let version_str = line
                                    .split("CUDA Version:")
                                    .nth(1)
                                    .unwrap_or("")
                                    .trim()
                                    .trim_matches('|')
                                    .trim();
                                let major_minor: String = version_str
                                    .chars()
                                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                                    .collect();
                                return Some(match major_minor.as_str() {
                                    v if v.starts_with("12.4") => CudaVersion::Cuda124,
                                    v if v.starts_with("13.1") || v.starts_with("13.") => {
                                        CudaVersion::Cuda131
                                    }
                                    "" => CudaVersion::Cuda124, // present but unparseable → safe default
                                    other => CudaVersion::Other(other.to_string()),
                                });
                            }
                        }
                        // nvidia-smi succeeded but no "CUDA Version:" line found
                        return Some(CudaVersion::Cuda124);
                    }
                    return None;
                }
                Ok(None) => {
                    if start.elapsed() > std::time::Duration::from_secs(5) {
                        let _ = process.kill();
                        let _ = process.wait();
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => return None,
            }
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

    /// Detect Vulkan support by probing the Vulkan loader library.
    ///
    /// We check for the presence of the platform's Vulkan ICD loader rather
    /// than spawning `vulkaninfo` (which can hang or produce noise on headless
    /// servers).  The loader is always present on machines where *any* Vulkan
    /// driver is installed; its absence definitively means no Vulkan.
    fn detect_vulkan_support(platform: &Platform) -> bool {
        match platform {
            Platform::Windows => {
                std::path::Path::new("C:\\Windows\\System32\\vulkan-1.dll").exists()
            }
            Platform::Linux => {
                std::path::Path::new("/usr/lib/x86_64-linux-gnu/libvulkan.so.1").exists()
                    || std::path::Path::new("/usr/lib/libvulkan.so.1").exists()
                    || std::path::Path::new("/usr/lib64/libvulkan.so.1").exists()
            }
            Platform::MacOS => {
                // MoltenVK ships with most macOS Metal installs; not needed for
                // our engine selection (we use the Metal build on macOS).
                false
            }
        }
    }

    /// Detect an AMD GPU on Windows (via PowerShell) or Linux (via lspci).
    /// Returns true if at least one AMD/ATI GPU is present.
    /// Does not block — runs a quick probe with a short timeout.
    fn detect_amd_gpu(platform: &Platform) -> bool {
        use std::process::{Command, Stdio};

        match platform {
            Platform::Windows => {
                #[cfg(target_os = "windows")]
                {
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

                    if let Ok(mut proc) = child {
                        let start = std::time::Instant::now();
                        loop {
                            match proc.try_wait() {
                                Ok(Some(_)) => {
                                    if let Ok(out) = proc.wait_with_output() {
                                        let text = String::from_utf8_lossy(&out.stdout);
                                        return text.lines().any(|l| {
                                            let l = l.to_uppercase();
                                            l.contains("AMD") || l.contains("RADEON") || l.contains("ATI")
                                        });
                                    }
                                    break;
                                }
                                Ok(None) => {
                                    if start.elapsed() > std::time::Duration::from_secs(8) {
                                        let _ = proc.kill();
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }
                                Err(_) => break,
                            }
                        }
                    }
                }
                false
            }
            Platform::Linux => {
                let child = Command::new("lspci")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn();

                if let Ok(mut proc) = child {
                    let start = std::time::Instant::now();
                    loop {
                        match proc.try_wait() {
                            Ok(Some(_)) => {
                                if let Ok(out) = proc.wait_with_output() {
                                    let text = String::from_utf8_lossy(&out.stdout);
                                    return text.lines().any(|l| {
                                        (l.contains("VGA") || l.contains("3D"))
                                            && (l.contains("AMD") || l.contains("ATI") || l.contains("Radeon"))
                                    });
                                }
                                break;
                            }
                            Ok(None) => {
                                if start.elapsed() > std::time::Duration::from_secs(5) {
                                    let _ = proc.kill();
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
                            Err(_) => break,
                        }
                    }
                }
                false
            }
            Platform::MacOS => false,
        }
    }

    /// Detect AMD ROCm compute stack on Linux.
    ///
    /// Checks (in order):
    /// 1. Common installation paths (`/opt/rocm`, `/usr/local/rocm`, …)
    /// 2. Environment variables (`ROCM_PATH`, `HIP_PATH`, `ROCM_HOME`)
    /// 3. `rocm-smi --version` command as a final probe
    fn detect_rocm_support(platform: &Platform) -> bool {
        if !matches!(platform, Platform::Linux) {
            return false;
        }

        // 1. Well-known paths (covers standard and distro-specific installs)
        let standard_paths = [
            "/opt/rocm",
            "/usr/local/rocm",
            "/usr/lib/rocm",
            "/usr/lib/amdgpu",
        ];
        for p in &standard_paths {
            if std::path::Path::new(p).exists() {
                return true;
            }
        }

        // 2. Environment variable overrides (container / custom install)
        for var in &["ROCM_PATH", "HIP_PATH", "ROCM_HOME"] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() && std::path::Path::new(&val).exists() {
                    return true;
                }
            }
        }

        // 3. rocm-smi probe (covers versioned installs like /opt/rocm-6.2.0)
        use std::process::{Command, Stdio};
        let child = Command::new("rocm-smi")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut proc) = child {
            let start = std::time::Instant::now();
            loop {
                match proc.try_wait() {
                    Ok(Some(status)) => return status.success(),
                    Ok(None) => {
                        if start.elapsed() > std::time::Duration::from_secs(5) {
                            let _ = proc.kill();
                            let _ = proc.wait();
                            return false;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => return false,
                }
            }
        }

        false
    }

    // ─── Intel GPU detection ──────────────────────────────────────────────────

    /// Returns true when any Intel GPU is present (UHD, Iris Xe, Arc, or Gaudi).
    ///
    /// Windows: queries `Win32_VideoController` for "Intel" in the name.
    /// Linux: `lspci` with vendor `8086` and VGA/3D/Display class.
    fn detect_intel_gpu(platform: &Platform) -> bool {
        use std::process::{Command, Stdio};
        match platform {
            Platform::Windows => {
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    let child = Command::new("powershell")
                        .args([
                            "-NoProfile", "-NonInteractive", "-Command",
                            "Get-CimInstance Win32_VideoController | \
                             Select-Object -ExpandProperty Name",
                        ])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .creation_flags(0x08000000)
                        .spawn();
                    if let Ok(mut proc) = child {
                        let start = std::time::Instant::now();
                        loop {
                            match proc.try_wait() {
                                Ok(Some(_)) => {
                                    if let Ok(out) = proc.wait_with_output() {
                                        let text = String::from_utf8_lossy(&out.stdout);
                                        return text.lines().any(|l| {
                                            let u = l.to_uppercase();
                                            u.contains("INTEL") && !u.contains("ETHERNET")
                                        });
                                    }
                                    break;
                                }
                                Ok(None) => {
                                    if start.elapsed() > std::time::Duration::from_secs(8) {
                                        let _ = proc.kill();
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }
                                Err(_) => break,
                            }
                        }
                    }
                }
                false
            }
            Platform::Linux => {
                let child = Command::new("lspci")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn();
                if let Ok(mut proc) = child {
                    let start = std::time::Instant::now();
                    loop {
                        match proc.try_wait() {
                            Ok(Some(_)) => {
                                if let Ok(out) = proc.wait_with_output() {
                                    let text = String::from_utf8_lossy(&out.stdout);
                                    return text.lines().any(|l| {
                                        (l.contains("VGA")
                                            || l.contains("3D")
                                            || l.contains("Display"))
                                            && l.contains("Intel")
                                    });
                                }
                                break;
                            }
                            Ok(None) => {
                                if start.elapsed() > std::time::Duration::from_secs(5) {
                                    let _ = proc.kill();
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
                            Err(_) => break,
                        }
                    }
                }
                false
            }
            Platform::MacOS => false,
        }
    }

    /// Returns true when a discrete Intel Arc GPU is detected.
    ///
    /// Windows: model name contains "Arc".
    /// Linux: Intel VGA device with PCI device ID in the 0x56xx range (Arc).
    fn detect_intel_arc(platform: &Platform) -> bool {
        use std::process::{Command, Stdio};
        match platform {
            Platform::Windows => {
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    let child = Command::new("powershell")
                        .args([
                            "-NoProfile", "-NonInteractive", "-Command",
                            "Get-CimInstance Win32_VideoController | \
                             Select-Object -ExpandProperty Name",
                        ])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .creation_flags(0x08000000)
                        .spawn();
                    if let Ok(mut proc) = child {
                        let start = std::time::Instant::now();
                        loop {
                            match proc.try_wait() {
                                Ok(Some(_)) => {
                                    if let Ok(out) = proc.wait_with_output() {
                                        let text = String::from_utf8_lossy(&out.stdout);
                                        return text.lines().any(|l| {
                                            let u = l.to_uppercase();
                                            u.contains("INTEL")
                                                && (u.contains("ARC")
                                                    || u.contains("IRIS XE"))
                                        });
                                    }
                                    break;
                                }
                                Ok(None) => {
                                    if start.elapsed() > std::time::Duration::from_secs(8) {
                                        let _ = proc.kill();
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }
                                Err(_) => break,
                            }
                        }
                    }
                }
                false
            }
            Platform::Linux => {
                // Intel Arc device IDs live in the 0x56xx range (A-series).
                // We also match on the string "Arc" appearing in lspci -v output.
                let child = Command::new("lspci")
                    .arg("-nn")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn();
                if let Ok(mut proc) = child {
                    let start = std::time::Instant::now();
                    loop {
                        match proc.try_wait() {
                            Ok(Some(_)) => {
                                if let Ok(out) = proc.wait_with_output() {
                                    let text = String::from_utf8_lossy(&out.stdout);
                                    return text.lines().any(|l| {
                                        // Match lines like "VGA ... Intel Corporation ... [8086:56a0]"
                                        (l.contains("VGA") || l.contains("3D") || l.contains("Display"))
                                            && l.contains("[8086:")
                                            && (l.contains(":56")   // Arc A-series
                                                || l.contains("Arc")
                                                || l.contains("Iris Xe"))
                                    });
                                }
                                break;
                            }
                            Ok(None) => {
                                if start.elapsed() > std::time::Duration::from_secs(5) {
                                    let _ = proc.kill();
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
                            Err(_) => break,
                        }
                    }
                }
                false
            }
            Platform::MacOS => false,
        }
    }

    /// Returns true when an Intel Gaudi / Gaudi2 HPU is detected.
    ///
    /// Habana Labs (acquired by Intel) uses PCI vendor ID `1da3`.
    /// Only meaningful on Linux data-centre hardware.
    fn detect_intel_gaudi(platform: &Platform) -> bool {
        if !matches!(platform, Platform::Linux) {
            return false;
        }

        // Fast path: device nodes created by the Habana driver
        if std::path::Path::new("/dev/accel0").exists() {
            return true;
        }

        // lspci probe: `-d 1da3:` filters to Habana Labs vendor
        use std::process::{Command, Stdio};
        let child = Command::new("lspci")
            .args(["-d", "1da3:"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut proc) = child {
            let start = std::time::Instant::now();
            loop {
                match proc.try_wait() {
                    Ok(Some(_)) => {
                        if let Ok(out) = proc.wait_with_output() {
                            // Any output means at least one Habana device is present
                            return !out.stdout.is_empty();
                        }
                        break;
                    }
                    Ok(None) => {
                        if start.elapsed() > std::time::Duration::from_secs(5) {
                            let _ = proc.kill();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        }
        false
    }

    /// Returns true when the Intel Level Zero runtime loader is installed.
    ///
    /// Level Zero is required to run the SYCL (oneAPI) llama.cpp build.
    /// Windows: `ze_loader.dll` in System32.
    /// Linux:   `libze_loader.so.1` in standard library directories.
    fn detect_level_zero(platform: &Platform) -> bool {
        match platform {
            Platform::Windows => {
                std::path::Path::new("C:\\Windows\\System32\\ze_loader.dll").exists()
            }
            Platform::Linux => {
                std::path::Path::new("/usr/lib/libze_loader.so.1").exists()
                    || std::path::Path::new(
                        "/usr/lib/x86_64-linux-gnu/libze_loader.so.1",
                    )
                    .exists()
                    || std::path::Path::new("/usr/local/lib/libze_loader.so.1").exists()
            }
            Platform::MacOS => false,
        }
    }

    /// Returns true when `amdhip64.dll` is present on Windows.
    ///
    /// This DLL is installed automatically by the AMD Radeon graphics driver
    /// and is the sole runtime requirement for the HIP/Radeon llama.cpp build.
    fn detect_amd_hip_windows(platform: &Platform) -> bool {
        if !matches!(platform, Platform::Windows) {
            return false;
        }
        std::path::Path::new("C:\\Windows\\System32\\amdhip64.dll").exists()
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
                if self.has_cuda {
                    Some(platform_dir.join("llama-b6970-bin-win-cuda-12.4-x64").join("llama-server.exe"))
                } else if self.has_amd_hip_windows {
                    // AMD GPU with HIP runtime: use the HIP/Radeon build
                    Some(platform_dir.join("llama-hip").join("llama-server.exe"))
                } else if self.has_intel_arc || self.has_level_zero {
                    // Intel Arc / Iris Xe with Level Zero: use the SYCL build
                    Some(platform_dir.join("llama-sycl").join("llama-server.exe"))
                } else if self.has_amd_gpu || self.has_vulkan {
                    // Any other Vulkan-capable GPU
                    Some(platform_dir.join("llama-vulkan").join("llama-server.exe"))
                } else {
                    Some(platform_dir.join("llama-cpu").join("llama-server.exe"))
                }
            }
            Platform::Linux => {
                // On Linux, return None — engine-registry selects the correct binary
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
