//! Runtime Dependency Manager
//!
//! Manages the runtime libraries (DLLs / shared objects) that llama.cpp engine
//! binaries depend on at launch time, *beyond* the files contained in the engine
//! archive itself.
//!
//! ## Problem
//! Each acceleration backend requires its own native runtime stack:
//! - **CUDA** needs `cudart64_12.dll` / `libcudart.so.12` etc.
//! - **HIP (AMD)** needs `amdhip64.dll` (Windows, installed with drivers).
//! - **SYCL (Intel)** needs `ze_loader.dll` / `libze_loader.so.1`.
//! - **Vulkan** needs the Vulkan ICD loader, normally shipped with GPU drivers.
//!
//! If these are missing, llama-server fails to start — often with a cryptic
//! "DLL not found" or `SIGILL` error — instead of a clear diagnostic.
//!
//! ## What this module does
//! 1. **Probes** whether each required runtime is present (system paths or local cache).
//! 2. **Downloads** missing runtimes when a public URL is available:
//!    - CUDA redist packages are shipped as separate ZIPs in the same llama.cpp GitHub
//!      release used for the engine binary (`cudart-llama-bin-win-cuda-*.zip`).
//!    - Intel Level Zero loader is available from `oneapi-src/level-zero` GitHub.
//! 3. **Returns a `DepSummary`** with the set of extra library paths to prepend to
//!    `PATH` / `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH` before launching llama-server.
//! 4. **Logs install guidance** for runtimes that require manual installation
//!    (AMD HIP on Windows, ROCm on Linux, Vulkan loaders).
//!
//! Everything is stored under `PathResolver::runtimes_dir()`:
//! ```text
//! runtimes/
//!   cuda-12.4/       ← extracted cudart-llama-bin-win-cuda-12.4-x64.zip
//!   cuda-13.1/       ← extracted cudart-llama-bin-win-cuda-13.1-x64.zip
//!   level-zero/      ← extracted level-zero-win-sdk-*.zip
//! ```

use std::path::PathBuf;
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

// ── BoundedReader ─────────────────────────────────────────────────────────────
// A minimal `Read` adapter that limits how many bytes can be read from a
// `File`.  Used during .deb extraction to stream-decompress only the
// `data.tar.*` section without reading the entire file into memory.

struct BoundedReader<'a> {
    file: &'a mut std::fs::File,
    remaining: u64,
}

impl<'a> std::io::Read for BoundedReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Ok(0);
        }
        let max = buf.len().min(self.remaining as usize);
        let n = self.file.read(&mut buf[..max])?;
        self.remaining -= n as u64;
        Ok(n)
    }
}

use super::registry::{AccelerationType, EngineInfo};
use crate::utils::PathResolver;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Same llama.cpp release version used by the engine registry.
/// Referenced in `install_guide_url` strings below.
#[allow(dead_code)]
const LLAMA_CPP_VERSION: &str = "b8037";

/// Intel Level Zero SDK version auto-downloaded for the SYCL engine.
/// v1.28.6 is the earliest release that publishes pre-built binary packages
/// (Windows zip + Ubuntu .deb).  v1.21.2 and earlier are source-only.
#[allow(dead_code)]
const LEVEL_ZERO_VERSION: &str = "1.28.6";

// ─── Public types ────────────────────────────────────────────────────────────

/// Identifies which category of runtime dependency this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeDepKind {
    /// CUDA 12.4 runtime DLLs — auto-downloadable from llama.cpp GitHub.
    CudaRuntime124,
    /// CUDA 13.1 runtime DLLs — auto-downloadable from llama.cpp GitHub.
    CudaRuntime131,
    /// AMD HIP runtime (`amdhip64.dll`) — installed with AMD Radeon drivers.
    /// Guidance-only; cannot be auto-downloaded independently.
    AmdHipWindows,
    /// Intel Level Zero loader — auto-downloadable from oneapi-src/level-zero.
    IntelLevelZero,
    /// Vulkan ICD loader — installed with GPU drivers.  Guidance-only.
    VulkanLoader,
    /// AMD ROCm stack on Linux — must be installed via system package manager.
    /// Guidance-only.
    RocmLinux,
    // ── v2 auto-download variants ──────────────────────────────────────────
    /// AMD HIP runtime DLLs on Windows — downloaded from AMD's MSI package and
    /// extracted via `msiexec /a` (no elevation required).
    AmdHipWindowsRuntime,
    /// AMD ROCm minimal runtime on Linux — extracted from the official
    /// `rocm-hip-runtime` .deb without root (userspace .so files only).
    AmdRocmLinuxRuntime,
    /// Vulkan ICD loader on Windows — downloaded from LunarG's redistributable
    /// runtime components ZIP.
    VulkanLoaderWindows,
    /// Vulkan ICD loader on Linux — extracted from `libvulkan1` .deb from the
    /// Ubuntu archive without root.
    VulkanLoaderLinux,
    /// Intel Level Zero loader on Linux — downloaded from the oneapi-src
    /// GitHub release as a tar.gz.
    IntelLevelZeroLinux,
}

/// Whether a runtime dependency is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepStatus {
    /// Found in well-known system paths.
    PresentOnSystem,
    /// Found in `PathResolver::runtimes_dir()` (previously downloaded).
    PresentInCache,
    /// Not found, but a download URL is available — will be fetched automatically.
    CanDownload,
    /// Not found and cannot be auto-downloaded; user must install manually.
    NeedsManualInstall,
}

/// A single runtime dependency record.
#[derive(Debug, Clone)]
pub struct RuntimeDep {
    pub kind: RuntimeDepKind,
    /// Human-readable name shown in log messages.
    pub name: &'static str,
    /// File names (DLL / SO) to probe for presence.
    pub probe_files: &'static [&'static str],
    /// System directories to search for `probe_files`.
    pub system_search_paths: Vec<PathBuf>,
    /// Sub-directory under `runtimes_dir()` where we cache this dep.
    pub cache_subdir: &'static str,
    /// Download URL, if the dep can be fetched automatically.
    pub download_url: Option<String>,
    /// Approximate download size in bytes (used for progress display).
    pub file_size: u64,
    /// URL shown in log warnings when the dep must be installed manually.
    pub install_guide_url: &'static str,
}

/// Summary returned by [`RuntimeDepsManager::ensure_deps_ready`].
#[derive(Debug)]
pub struct DepSummary {
    /// `true` when every *critical* dep is ready (system or cache).
    /// Warning-level deps (guidance-only) do not affect this flag.
    pub all_critical_ready: bool,
    /// Status for each required dep.
    pub deps: Vec<(RuntimeDep, DepStatus)>,
    /// Extra directories to prepend to the library search path.
    pub extra_library_paths: Vec<PathBuf>,
}

// ─── Manager ─────────────────────────────────────────────────────────────────

/// Manages detection and on-demand download of engine runtime dependencies.
pub struct RuntimeDepsManager {
    client: Client,
}

impl RuntimeDepsManager {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .unwrap_or_default(),
        }
    }

    // ─── Main entry point ────────────────────────────────────────────────────

    /// Check and, where possible, download all runtime deps for `engine`.
    ///
    /// This is called by `gguf_runtime::start_server()` before spawning
    /// llama-server.  It is intentionally non-fatal: any error is logged as a
    /// warning and an empty `DepSummary` is returned so the launch can proceed.
    pub async fn ensure_deps_ready(engine: &EngineInfo) -> Result<DepSummary> {
        let mgr = Self::new();

        // ── Engine-bundle probe for AMD HIP ───────────────────────────────────
        // The llama.cpp HIP Windows ZIP sometimes ships amdhip64.dll alongside
        // the binary.  If found there, skip the dep-download path entirely and
        // add the engine directory to the library search path.
        if engine.acceleration == AccelerationType::Hip {
            if let Some(bundle_dir) = Self::check_hip_dlls_in_engine_bundle(engine) {
                info!(
                    "AMD HIP DLLs found in engine bundle at {} — skipping separate download",
                    bundle_dir.display()
                );
                return Ok(DepSummary {
                    all_critical_ready: true,
                    deps: vec![],
                    extra_library_paths: vec![bundle_dir],
                });
            }
        }

        let deps = Self::required_deps_for_engine(engine);

        if deps.is_empty() {
            return Ok(DepSummary {
                all_critical_ready: true,
                deps: vec![],
                extra_library_paths: vec![],
            });
        }

        let mut results: Vec<(RuntimeDep, DepStatus)> = Vec::new();
        let mut extra_paths: Vec<PathBuf> = Vec::new();
        let mut all_critical_ready = true;

        for dep in deps {
            let status = Self::check_status(&dep);
            match &status {
                DepStatus::PresentOnSystem => {
                    info!("Runtime dep '{}': present on system", dep.name);
                }
                DepStatus::PresentInCache => {
                    let cache_root = PathResolver::runtimes_dir().join(dep.cache_subdir);
                    // Use the exact subdirectory containing the probe files so that
                    // PATH/LD_LIBRARY_PATH resolves the DLLs even when the ZIP extracted
                    // into a subdirectory (e.g. cudart-llama-bin-win-cuda-12.4-x64/).
                    let actual_path = Self::find_probe_dir(&cache_root, dep.probe_files)
                        .unwrap_or(cache_root.clone());
                    info!(
                        "Runtime dep '{}': present in cache at {}",
                        dep.name,
                        actual_path.display()
                    );
                    extra_paths.push(actual_path);
                }
                DepStatus::CanDownload => {
                    info!("Runtime dep '{}': not found — downloading…", dep.name);
                    match mgr.download_dep(&dep).await {
                        Ok(path) => {
                            info!("Runtime dep '{}': downloaded to {}", dep.name, path.display());
                            extra_paths.push(path);
                        }
                        Err(e) => {
                            warn!(
                                "Runtime dep '{}': download failed ({}). \
                                 Install manually from: {}",
                                dep.name, e, dep.install_guide_url
                            );
                            all_critical_ready = false;
                        }
                    }
                }
                DepStatus::NeedsManualInstall => {
                    warn!(
                        "Runtime dep '{}' not found. \
                         Install it manually to enable GPU acceleration: {}",
                        dep.name, dep.install_guide_url
                    );
                    // Guidance-only deps don't block startup — the engine may
                    // still load if the file is somewhere on the system PATH.
                }
            }
            results.push((dep, status));
        }

        Ok(DepSummary {
            all_critical_ready,
            deps: results,
            extra_library_paths: extra_paths,
        })
    }

    // ─── Dep catalogue ───────────────────────────────────────────────────────

    /// Return the list of runtime dependencies required by `engine`.
    ///
    /// The mapping is based on `AccelerationType` and the engine ID so we can
    /// distinguish CUDA 12.4 from CUDA 13.1.
    pub fn required_deps_for_engine(engine: &EngineInfo) -> Vec<RuntimeDep> {
        match &engine.acceleration {
            AccelerationType::CUDA => Self::cuda_deps_for_engine(engine),
            AccelerationType::Sycl => {
                // Windows: existing Level Zero download path (already CanDownload).
                // Linux: new auto-download path for libze_loader.so.1.
                if cfg!(target_os = "linux") {
                    Self::sycl_linux_deps()
                } else {
                    Self::sycl_deps()
                }
            }
            AccelerationType::Hip => {
                // Windows: auto-download amdhip64.dll via AMD MSI + engine bundle probe.
                // Linux: extract rocm-hip-runtime .deb locally, no root.
                if cfg!(target_os = "windows") {
                    Self::hip_windows_auto_deps()
                } else {
                    Self::rocm_linux_auto_deps()
                }
            }
            AccelerationType::Vulkan => {
                // Windows: auto-download vulkan-1.dll from LunarG if missing.
                // Linux: auto-download libvulkan.so.1 from Ubuntu archive if missing.
                // macOS: Vulkan not used (Metal is the GPU path); no deps.
                if cfg!(target_os = "windows") {
                    Self::vulkan_windows_auto_deps()
                } else if cfg!(target_os = "linux") {
                    Self::vulkan_linux_auto_deps()
                } else {
                    Self::vulkan_deps()
                }
            }
            AccelerationType::Metal | AccelerationType::CPU | AccelerationType::OpenCL
            | AccelerationType::DirectML => vec![],
        }
    }

    fn cuda_deps_for_engine(engine: &EngineInfo) -> Vec<RuntimeDep> {
        // Platform-specific: CUDA DLLs only on Windows
        if cfg!(not(target_os = "windows")) {
            // On Linux the system libcudart is used; we provide guidance only.
            return vec![RuntimeDep {
                kind: RuntimeDepKind::RocmLinux, // reuse guidance pattern
                name: "CUDA Runtime (Linux)",
                probe_files: &["libcudart.so.12", "libcublas.so.12"],
                system_search_paths: vec![
                    PathBuf::from("/usr/local/cuda/lib64"),
                    PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                    PathBuf::from("/usr/lib64"),
                ],
                cache_subdir: "cuda-linux",
                download_url: None,
                file_size: 0,
                install_guide_url: "https://developer.nvidia.com/cuda-downloads",
            }];
        }

        // On Windows: pick the right redist package based on the engine ID.
        if engine.id.contains("cuda131") || engine.id.contains("cuda-13.1") {
            vec![Self::cuda_131_dep()]
        } else {
            // Default to 12.4 for any other CUDA engine
            vec![Self::cuda_124_dep()]
        }
    }

    fn cuda_124_dep() -> RuntimeDep {
        RuntimeDep {
            kind: RuntimeDepKind::CudaRuntime124,
            name: "CUDA 12.4 Runtime",
            probe_files: &["cudart64_12.dll", "cublas64_12.dll"],
            system_search_paths: vec![
                PathBuf::from("C:\\Windows\\System32"),
                PathBuf::from("C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v12.4\\bin"),
            ],
            cache_subdir: "cuda-12.4",
            download_url: Some(
                // llama.cpp ships the CUDA runtime redist in the same GitHub release
                "https://github.com/ggml-org/llama.cpp/releases/download/\
                 b8037/cudart-llama-bin-win-cuda-12.4-x64.zip".to_string()
            ),
            file_size: 391 * 1024 * 1024,
            install_guide_url:
                "https://developer.nvidia.com/cuda-12-4-0-download-archive",
        }
    }

    fn cuda_131_dep() -> RuntimeDep {
        RuntimeDep {
            kind: RuntimeDepKind::CudaRuntime131,
            name: "CUDA 13.1 Runtime",
            probe_files: &["cudart64_130.dll", "cublas64_13.dll"],
            system_search_paths: vec![
                PathBuf::from("C:\\Windows\\System32"),
                PathBuf::from("C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v13.1\\bin"),
            ],
            cache_subdir: "cuda-13.1",
            download_url: Some(
                "https://github.com/ggml-org/llama.cpp/releases/download/\
                 b8037/cudart-llama-bin-win-cuda-13.1-x64.zip".to_string()
            ),
            file_size: 402 * 1024 * 1024,
            install_guide_url:
                "https://developer.nvidia.com/cuda-downloads",
        }
    }

    fn sycl_deps() -> Vec<RuntimeDep> {
        vec![RuntimeDep {
            kind: RuntimeDepKind::IntelLevelZero,
            name: "Intel Level Zero Runtime",
            probe_files: if cfg!(target_os = "windows") {
                &["ze_loader.dll"]
            } else {
                &["libze_loader.so.1"]
            },
            system_search_paths: if cfg!(target_os = "windows") {
                vec![PathBuf::from("C:\\Windows\\System32")]
            } else {
                vec![
                    PathBuf::from("/usr/lib"),
                    PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                    PathBuf::from("/usr/local/lib"),
                ]
            },
            cache_subdir: "level-zero",
            download_url: if cfg!(target_os = "windows") {
                Some(
                    // Intel Level Zero SDK — v1.28.6 is the earliest release
                    // that ships a pre-built binary ZIP (v1.21.2 was source only).
                    "https://github.com/oneapi-src/level-zero/releases/download/\
                     v1.28.6/level-zero-win-sdk-1.28.6.zip".to_string()
                )
            } else {
                None // Linux: see sycl_linux_deps() for the .deb auto-download path
            },
            file_size: 15 * 1024 * 1024,
            install_guide_url:
                "https://www.intel.com/content/www/us/en/developer/tools/oneapi/\
                 base-toolkit-download.html",
        }]
    }

    // Kept for existing tests and external callers that reference the original
    // guidance-only behaviour.  The new dispatch path uses hip_windows_auto_deps().
    #[allow(dead_code)]
    fn hip_deps() -> Vec<RuntimeDep> {
        // AMD HIP on Windows: amdhip64.dll ships with the Radeon driver
        vec![RuntimeDep {
            kind: RuntimeDepKind::AmdHipWindows,
            name: "AMD HIP Runtime (amdhip64.dll)",
            probe_files: &["amdhip64.dll"],
            system_search_paths: vec![PathBuf::from("C:\\Windows\\System32")],
            cache_subdir: "amd-hip",
            download_url: None, // part of AMD driver — cannot be downloaded separately
            file_size: 0,
            install_guide_url:
                "https://www.amd.com/en/developer/resources/rocm-hub/hip-sdk.html",
        }]
    }

    fn vulkan_deps() -> Vec<RuntimeDep> {
        let (probe_files, system_search_paths): (&'static [&'static str], Vec<PathBuf>) =
            if cfg!(target_os = "windows") {
                (
                    &["vulkan-1.dll"],
                    vec![PathBuf::from("C:\\Windows\\System32")],
                )
            } else {
                (
                    &["libvulkan.so.1"],
                    vec![
                        PathBuf::from("/usr/lib"),
                        PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                        PathBuf::from("/usr/lib64"),
                    ],
                )
            };

        vec![RuntimeDep {
            kind: RuntimeDepKind::VulkanLoader,
            name: "Vulkan Runtime Loader",
            probe_files,
            system_search_paths,
            cache_subdir: "vulkan",
            download_url: None, // installed with GPU drivers
            file_size: 0,
            install_guide_url:
                "https://vulkan.lunarg.com/sdk/home",
        }]
    }

    // ═════════════════════════════════════════════════════════════════════════
    // Linux distro detection
    // ═════════════════════════════════════════════════════════════════════════

    /// Reads `/etc/os-release` and returns a coarse distro tag used to select
    /// the correct package URL and format:
    ///
    /// - `"ubuntu2204"` — Ubuntu 22.04 LTS (Jammy)
    /// - `"ubuntu2404"` — Ubuntu 24.04 LTS (Noble) or later
    /// - `"rhel"`       — RHEL, Rocky Linux, AlmaLinux, Fedora
    /// - `"debian"`     — Debian (any version)
    /// - `"unknown"`    — anything else (triggers guidance-only fallback)
    fn detect_linux_distro() -> &'static str {
        let content = match std::fs::read_to_string("/etc/os-release") {
            Ok(s) => s,
            Err(_) => return "unknown",
        };
        let id = content.lines()
            .find(|l| l.starts_with("ID="))
            .map(|l| l.trim_start_matches("ID=").trim_matches('"').to_lowercase())
            .unwrap_or_default();
        let version_id = content.lines()
            .find(|l| l.starts_with("VERSION_ID="))
            .map(|l| l.trim_start_matches("VERSION_ID=").trim_matches('"').to_string())
            .unwrap_or_default();

        match id.as_str() {
            "ubuntu" => {
                if version_id.starts_with("22.") { "ubuntu2204" }
                else { "ubuntu2404" } // treat 24.04 and any future version as noble
            }
            "debian" => "debian",
            "rhel" | "rocky" | "almalinux" | "fedora" | "centos" => "rhel",
            _ => "unknown",
        }
    }

    // ═════════════════════════════════════════════════════════════════════════
    // v2 Auto-Download Dep Builders
    //
    // All functions below are ADDITIVE.  The original hip_deps(), sycl_deps(),
    // and vulkan_deps() builders remain intact and are still used by their
    // original callers and tests.  The new `required_deps_for_engine()`
    // dispatch routes to these v2 builders for platforms that can now
    // auto-download instead of falling back to manual-install guidance.
    // ═════════════════════════════════════════════════════════════════════════

    // ── AMD HIP Windows ──────────────────────────────────────────────────────

    /// AMD HIP runtime for Windows.
    ///
    /// **Primary path (most likely to succeed):** the engine bundle probe in
    /// `ensure_deps_ready` runs before this dep list is evaluated.  The
    /// llama.cpp HIP Windows ZIP (`llama-b8037-bin-win-hip-radeon-x64.zip`)
    /// bundles `amdhip64.dll` alongside `llama-server.exe`.  When the ZIP is
    /// extracted the DLL is already in the engine directory, so no further
    /// download is needed.
    ///
    /// **Fallback:** AMD does not publish `amdhip64.dll` as a standalone
    /// redistributable ZIP.  Their Windows distribution is through the HIP SDK
    /// installer at `https://www.amd.com/en/developer/resources/rocm-hub/hip-sdk.html`,
    /// which has no stable direct-download URL.  `download_url` is therefore
    /// `None` here — if the bundle probe misses the DLL, the operator receives
    /// a clear `NeedsManualInstall` message with the exact AMD download page.
    fn hip_windows_auto_deps() -> Vec<RuntimeDep> {
        vec![RuntimeDep {
            kind: RuntimeDepKind::AmdHipWindowsRuntime,
            name: "AMD HIP Runtime (amdhip64.dll)",
            probe_files: &["amdhip64.dll", "hipblas.dll"],
            system_search_paths: vec![
                PathBuf::from("C:\\Windows\\System32"),
                PathBuf::from("C:\\Program Files\\AMD\\ROCm\\6.4\\bin"),
                PathBuf::from("C:\\Program Files\\AMD\\ROCm\\7.2\\bin"),
            ],
            cache_subdir: "amd-hip",
            // No stable standalone DLL ZIP from AMD — the engine bundle probe
            // (see ensure_deps_ready) handles the common case automatically.
            download_url: None,
            file_size: 0,
            install_guide_url:
                "https://www.amd.com/en/developer/resources/rocm-hub/hip-sdk.html",
        }]
    }

    // ── AMD ROCm Linux ───────────────────────────────────────────────────────

    /// AMD ROCm minimal runtime for Linux.
    ///
    /// Downloads the `rocm-hip-runtime` .deb from AMD's official apt
    /// repository and extracts only the userspace `.so` files into
    /// `runtimes/rocm-linux/` — no root, no `apt install`, no kernel modules.
    ///
    /// The GPU kernel driver (amdgpu) must already be loaded by the system.
    /// This provides the HIP/ROCm userspace runtime on top of that driver.
    ///
    /// VERIFY: confirm package name and version string at
    ///   https://repo.radeon.com/rocm/apt/debian/pool/main/r/rocm-hip-runtime/
    fn rocm_linux_auto_deps() -> Vec<RuntimeDep> {
        // Verified 2026-05-22: only 7.x packages exist in AMD's apt repo.
        // Ubuntu 22.04 (Jammy): rocm-hip-runtime_7.2.3.70203-90~22.04_amd64.deb
        // Ubuntu 24.04 (Noble): rocm-hip-runtime_7.2.3.70203-90~24.04_amd64.deb
        // RHEL/Rocky/Alma: RPM repo at repo.radeon.com/rocm/rhel9/
        // Debian / unknown: guidance only (no pre-built deb for Debian stable)
        let distro = Self::detect_linux_distro();
        let (download_url, install_guide_url): (Option<String>, &'static str) = match distro {
            "ubuntu2204" => (
                Some("https://repo.radeon.com/rocm/apt/debian/pool/main/r/\
                      rocm-hip-runtime/rocm-hip-runtime_7.2.3.70203-90~22.04_amd64.deb".to_string()),
                "https://rocm.docs.amd.com/en/latest/deploy/linux/quick_start.html",
            ),
            "ubuntu2404" => (
                Some("https://repo.radeon.com/rocm/apt/debian/pool/main/r/\
                      rocm-hip-runtime/rocm-hip-runtime_7.2.3.70203-90~24.04_amd64.deb".to_string()),
                "https://rocm.docs.amd.com/en/latest/deploy/linux/quick_start.html",
            ),
            "rhel" => (
                // RHEL/Rocky/Alma: RPM, no auto-extract support yet → guidance only
                None,
                "https://rocm.docs.amd.com/en/latest/deploy/linux/quick_start.html",
            ),
            _ => (
                None,
                "https://rocm.docs.amd.com/en/latest/deploy/linux/quick_start.html",
            ),
        };
        vec![RuntimeDep {
            kind: RuntimeDepKind::AmdRocmLinuxRuntime,
            name: "AMD ROCm HIP Runtime (Linux)",
            probe_files: &["libamdhip64.so.6", "libhipblas.so.2"],
            system_search_paths: vec![
                PathBuf::from("/opt/rocm/lib"),
                PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                PathBuf::from("/usr/lib64"),
            ],
            cache_subdir: "rocm-linux",
            download_url,
            file_size: 150 * 1024 * 1024, // ~150 MB deb
            install_guide_url,
        }]
    }

    // ── Vulkan Windows ───────────────────────────────────────────────────────

    /// Vulkan ICD loader for Windows.
    ///
    /// Checks system paths first — `vulkan-1.dll` is installed by every major
    /// GPU driver (NVIDIA, AMD Radeon, Intel).  Only downloads if absent.
    ///
    /// Source: LunarG Vulkan Runtime components ZIP (redistributable under
    /// the LunarG Software Development Kit License Agreement).
    ///
    /// Note: the loader alone does not perform rendering — a GPU ICD
    /// (NVIDIA, AMD, or Intel driver) must also be present.  Since we only
    /// select a Vulkan engine when a compatible GPU is detected, the ICD
    /// should already be installed with the GPU driver.
    fn vulkan_windows_auto_deps() -> Vec<RuntimeDep> {
        vec![RuntimeDep {
            kind: RuntimeDepKind::VulkanLoaderWindows,
            name: "Vulkan Runtime Loader (vulkan-1.dll)",
            probe_files: &["vulkan-1.dll"],
            system_search_paths: vec![
                PathBuf::from("C:\\Windows\\System32"),
                PathBuf::from("C:\\Windows\\SysWOW64"),
            ],
            cache_subdir: "vulkan-windows",
            download_url: Some(
                "https://sdk.lunarg.com/sdk/download/1.3.283.0/windows/\
                 VulkanRT-1.3.283.0-Components.zip".to_string()
            ),
            file_size: 2 * 1024 * 1024, // ~2 MB
            install_guide_url: "https://vulkan.lunarg.com/sdk/home",
        }]
    }

    // ── Vulkan Linux ─────────────────────────────────────────────────────────

    /// Vulkan ICD loader for Linux.
    ///
    /// Checks system paths first — `libvulkan.so.1` is installed by mesa or
    /// any GPU vendor driver.  Only downloads if absent.
    ///
    /// Source: Ubuntu archive `libvulkan1` package, extracted without root.
    /// The GPU-specific ICD (mesa radv / nvidia vulkan / intel anv) must be
    /// installed with the GPU driver.
    ///
    /// VERIFY: confirm the exact .deb URL at
    ///   https://packages.ubuntu.com/jammy/libvulkan1
    fn vulkan_linux_auto_deps() -> Vec<RuntimeDep> {
        // Ubuntu 22.04: libvulkan1_1.3.204.1-2_amd64.deb (universe)
        // Ubuntu 24.04: libvulkan1_1.3.275.0-1_amd64.deb (universe/noble)
        // Debian/RHEL/unknown: guidance only
        let distro = Self::detect_linux_distro();
        let (download_url, install_guide_url): (Option<String>, &'static str) = match distro {
            "ubuntu2204" => (
                Some("http://archive.ubuntu.com/ubuntu/pool/universe/v/vulkan-loader/\
                      libvulkan1_1.3.204.1-2_amd64.deb".to_string()),
                "https://packages.ubuntu.com/jammy/libvulkan1",
            ),
            "ubuntu2404" => (
                Some("http://archive.ubuntu.com/ubuntu/pool/universe/v/vulkan-loader/\
                      libvulkan1_1.3.275.0-1_amd64.deb".to_string()),
                "https://packages.ubuntu.com/noble/libvulkan1",
            ),
            _ => (None, "https://vulkan.lunarg.com/sdk/home"),
        };
        vec![RuntimeDep {
            kind: RuntimeDepKind::VulkanLoaderLinux,
            name: "Vulkan Runtime Loader (libvulkan.so.1)",
            probe_files: &["libvulkan.so.1"],
            system_search_paths: vec![
                PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                PathBuf::from("/usr/lib"),
                PathBuf::from("/usr/lib64"),
                PathBuf::from("/lib/x86_64-linux-gnu"),
            ],
            cache_subdir: "vulkan-linux",
            download_url,
            file_size: 140 * 1024, // ~140 KB deb
            install_guide_url,
        }]
    }

    // ── Intel Level Zero Linux ───────────────────────────────────────────────

    /// Intel Level Zero loader for Linux.
    ///
    /// Used by Intel Arc / Iris Xe GPU SYCL engine.  Downloads from the
    /// official `oneapi-src/level-zero` GitHub release as a tar.gz.
    ///
    /// VERIFY: confirm asset name at
    ///   https://github.com/oneapi-src/level-zero/releases/tag/v1.21.2
    fn sycl_linux_deps() -> Vec<RuntimeDep> {
        // Verified 2026-05-22:
        // - v1.21.2 has NO binary assets (source code only).
        // - v1.28.6 is the first release with pre-built .deb packages.
        // - The '+' in filenames is URL-encoded as %2B by GitHub.
        // Ubuntu 22.04: libze1_1.28.6%2Bu22.04_amd64.deb
        // Ubuntu 24.04: libze1_1.28.6%2Bu24.04_amd64.deb
        // RHEL/Debian/unknown: guidance only
        let distro = Self::detect_linux_distro();
        let download_url: Option<String> = match distro {
            "ubuntu2204" => Some(
                "https://github.com/oneapi-src/level-zero/releases/download/\
                 v1.28.6/libze1_1.28.6%2Bu22.04_amd64.deb".to_string()
            ),
            "ubuntu2404" => Some(
                "https://github.com/oneapi-src/level-zero/releases/download/\
                 v1.28.6/libze1_1.28.6%2Bu24.04_amd64.deb".to_string()
            ),
            _ => None,
        };
        vec![RuntimeDep {
            kind: RuntimeDepKind::IntelLevelZeroLinux,
            name: "Intel Level Zero Runtime (Linux)",
            probe_files: &["libze_loader.so.1"],
            system_search_paths: vec![
                PathBuf::from("/usr/lib"),
                PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                PathBuf::from("/usr/local/lib"),
            ],
            cache_subdir: "level-zero-linux",
            download_url,
            file_size: 2 * 1024 * 1024, // ~2 MB
            install_guide_url:
                "https://dgpu-docs.intel.com/driver/installation.html",
        }]
    }

    // ── Engine bundle probe ───────────────────────────────────────────────────

    /// Scan the engine's install directory for `amdhip64.dll` (and similar
    /// AMD HIP DLLs).  The llama.cpp HIP Windows release sometimes bundles
    /// the required AMD runtime DLLs alongside `llama-server.exe`.
    ///
    /// Returns the directory path where the DLLs were found, or `None` if
    /// the engine is not yet installed or the DLLs are absent from the bundle.
    pub fn check_hip_dlls_in_engine_bundle(engine: &EngineInfo) -> Option<PathBuf> {
        let install_dir = engine.install_path.as_ref()?;
        let probe_files = ["amdhip64.dll", "hipblas.dll", "libamdhip64.so.6"];
        // Check the install directory and one level of subdirectories
        for probe in &probe_files {
            if install_dir.join(probe).exists() {
                return Some(install_dir.clone());
            }
        }
        if let Ok(entries) = std::fs::read_dir(install_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    for probe in &probe_files {
                        if p.join(probe).exists() {
                            return Some(p);
                        }
                    }
                }
            }
        }
        None
    }

    // ─── Status check ────────────────────────────────────────────────────────

    /// Recursively walk `dir` for any of `probe_files`. Returns the first
    /// directory that directly contains one, or `None`.
    ///
    /// Uses `walkdir` so .deb payloads extracted into deep paths like
    /// `rocm-linux/opt/rocm-7.2.3/lib/libamdhip64.so.6` are found correctly.
    fn find_probe_dir(dir: &PathBuf, probe_files: &[&str]) -> Option<PathBuf> {
        use walkdir::WalkDir;
        for entry in WalkDir::new(dir).min_depth(0).into_iter().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if probe_files.iter().any(|f| path.join(f).exists()) {
                    return Some(path.to_path_buf());
                }
            }
        }
        None
    }

    /// Check the availability of a single dependency without downloading it.
    pub fn check_status(dep: &RuntimeDep) -> DepStatus {
        // 1. Check our local cache (including subdirectories — ZIPs may have a top-level dir)
        let cache_dir = PathResolver::runtimes_dir().join(dep.cache_subdir);
        if cache_dir.exists() {
            if Self::find_probe_dir(&cache_dir, dep.probe_files).is_some() {
                return DepStatus::PresentInCache;
            }
        }

        // 2. Check system paths
        for dir in &dep.system_search_paths {
            if dep.probe_files.iter().any(|f| dir.join(f).exists()) {
                return DepStatus::PresentOnSystem;
            }
        }

        // 3. Also scan PATH / LD_LIBRARY_PATH for the probe files
        let path_env = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(target_os = "windows") { ';' } else { ':' };
        for dir in path_env.split(sep) {
            let dir = PathBuf::from(dir);
            if dep.probe_files.iter().any(|f| dir.join(f).exists()) {
                return DepStatus::PresentOnSystem;
            }
        }

        // 4. Not found — can we download it?
        if dep.download_url.is_some() {
            DepStatus::CanDownload
        } else {
            DepStatus::NeedsManualInstall
        }
    }

    // ─── Download ────────────────────────────────────────────────────────────

    /// Download and extract `dep` into `runtimes_dir() / dep.cache_subdir`.
    ///
    /// Uses the same streaming download + extraction approach as `EngineDownloader`.
    async fn download_dep(&self, dep: &RuntimeDep) -> Result<PathBuf> {
        let url = dep.download_url.as_deref().ok_or_else(|| {
            anyhow::anyhow!("Dep '{}' has no download URL", dep.name)
        })?;

        let dest_dir = PathResolver::runtimes_dir().join(dep.cache_subdir);
        fs::create_dir_all(&dest_dir).await?;

        // Derive a temp filename from the URL
        let filename = url.rsplit('/').next().unwrap_or("dep_archive.zip");
        let tmp_path = std::env::temp_dir().join(format!("oi_dep_{}", filename));

        info!("Downloading runtime dep '{}' from {}", dep.name, url);

        // Stream download
        let response = self.client.get(url).send().await?.error_for_status()?;
        let mut downloaded: u64 = 0;
        let mut file = fs::File::create(&tmp_path).await?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
        }
        file.flush().await?;
        info!("Download complete: {} bytes for '{}'", downloaded, dep.name);

        // Extract based on format
        let tmp_owned = tmp_path.clone();
        let dest_owned = dest_dir.clone();
        let ext = tmp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        tokio::task::spawn_blocking(move || {
            if ext == "zip" || ext == "nupkg" {
                // .nupkg (NuGet) packages are standard ZIP archives
                Self::extract_zip_blocking(&tmp_owned, &dest_owned)
            } else if ext == "gz" || tmp_owned.to_string_lossy().ends_with(".tar.gz") {
                Self::extract_tar_gz_blocking(&tmp_owned, &dest_owned)
            } else if ext == "msi" {
                // AMD HIP SDK — use msiexec /a (administrative install, no elevation)
                Self::extract_msi_blocking(&tmp_owned, &dest_owned)
            } else if ext == "deb" {
                // AMD ROCm / Vulkan Linux / Level Zero packages
                Self::extract_deb_blocking(&tmp_owned, &dest_owned)
            } else {
                Err(anyhow::anyhow!("Unsupported archive format: {}", ext))
            }
        })
        .await??;

        // Clean up temp file
        let _ = fs::remove_file(&tmp_path).await;

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(entries) = std::fs::read_dir(&dest_dir) {
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.path().metadata() {
                        let mut perms = meta.permissions();
                        perms.set_mode(0o755);
                        let _ = std::fs::set_permissions(entry.path(), perms);
                    }
                }
            }
        }

        // Find the actual directory that contains the probe files — the ZIP may have
        // extracted into a subdirectory (e.g. cudart-llama-bin-win-cuda-12.4-x64/).
        let actual_dir = Self::find_probe_dir(&dest_dir, dep.probe_files)
            .unwrap_or(dest_dir.clone());
        info!("Runtime dep '{}' installed to {}", dep.name, actual_dir.display());
        Ok(actual_dir)
    }

    fn extract_zip_blocking(archive: &PathBuf, dest: &PathBuf) -> Result<()> {
        let file = std::fs::File::open(archive)?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|e| anyhow::anyhow!("ZIP open error: {}", e))?;

        // Detect whether the ZIP has a single top-level directory so we can strip it,
        // keeping behaviour consistent with extract_tar_gz_blocking.
        let top_dir: Option<String> = {
            let mut candidate: Option<String> = None;
            let mut is_single_root = true;
            for i in 0..zip.len() {
                if let Ok(e) = zip.by_index(i) {
                    if let Some(p) = e.enclosed_name() {
                        if let Some(first) = p.components().next() {
                            let name = first.as_os_str().to_string_lossy().to_string();
                            match &candidate {
                                None => candidate = Some(name),
                                Some(c) if *c == name => {}
                                _ => { is_single_root = false; break; }
                            }
                        }
                    }
                }
            }
            if is_single_root { candidate } else { None }
        };

        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| anyhow::anyhow!("ZIP read error: {}", e))?;
            let raw = match entry.enclosed_name() {
                Some(p) => p.to_path_buf(),
                None => continue,
            };

            // Strip top-level directory component if present
            let stripped: std::path::PathBuf = match &top_dir {
                Some(root) => {
                    let mut comps = raw.components();
                    match comps.next() {
                        Some(std::path::Component::Normal(c)) if c == root.as_str() => {
                            comps.collect()
                        }
                        _ => raw.clone(),
                    }
                }
                None => raw.clone(),
            };

            if stripped.as_os_str().is_empty() {
                continue; // was the root directory entry itself
            }

            let out = dest.join(&stripped);
            if entry.name().ends_with('/') {
                std::fs::create_dir_all(&out)?;
            } else {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&out)?;
                std::io::copy(&mut entry, &mut outfile)?;
            }
        }
        Ok(())
    }

    // ── .deb extraction (no root, no apt) ────────────────────────────────────

    /// Extract a Debian `.deb` archive into `dest` without root access or any
    /// system package manager.
    ///
    /// A `.deb` is an `ar` archive containing:
    /// ```text
    /// debian-binary   — format version ("2.0\n")
    /// control.tar.*   — package metadata
    /// data.tar.*      — the actual files (xz / zst / gz compression)
    /// ```
    ///
    /// We parse the `ar` format manually (60-byte headers, simple ASCII
    /// fields) to locate `data.tar.*` and then stream-decompress it into
    /// `dest`.  Supports xz (via `xz2`), zstd (via `zstd`), and gz (via
    /// `flate2` — already in our deps).
    fn extract_deb_blocking(deb_path: &PathBuf, dest: &PathBuf) -> Result<()> {
        use std::io::{Read, Seek, SeekFrom};

        let mut f = std::fs::File::open(deb_path)
            .map_err(|e| anyhow::anyhow!("Cannot open .deb file: {}", e))?;

        // Validate ar magic header
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic)
            .map_err(|e| anyhow::anyhow!("Cannot read .deb header: {}", e))?;
        if &magic != b"!<arch>\n" {
            return Err(anyhow::anyhow!(
                "Not a valid .deb file — ar magic header mismatch"
            ));
        }

        std::fs::create_dir_all(dest)?;

        loop {
            // Each ar entry starts with a 60-byte header.
            let mut hdr = [0u8; 60];
            match f.read_exact(&mut hdr) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            // Validate end-of-header marker (`\n at bytes 58-59)
            if hdr[58] != b'`' || hdr[59] != b'\n' {
                return Err(anyhow::anyhow!(
                    "Corrupt ar entry header (missing end marker)"
                ));
            }

            let name = std::str::from_utf8(&hdr[0..16])
                .unwrap_or("")
                .trim_end_matches(' ')
                .trim_end_matches('/');

            let size: u64 = std::str::from_utf8(&hdr[48..58])
                .unwrap_or("0")
                .trim()
                .parse()
                .unwrap_or(0);

            let entry_start = f.stream_position()?;

            if name.starts_with("data.tar") {
                // Stream-decompress the data archive directly from the file
                // to avoid reading the whole .deb into RAM.
                let bounded = BoundedReader { file: &mut f, remaining: size };

                if name.ends_with(".xz") || name == "data.tar.xz" {
                    let decoder = xz2::read::XzDecoder::new(bounded);
                    let mut archive = tar::Archive::new(decoder);
                    Self::extract_tar_archive_to(&mut archive, dest)?;
                } else if name.ends_with(".zst") || name == "data.tar.zst" {
                    let decoder = zstd::stream::read::Decoder::new(bounded)
                        .map_err(|e| anyhow::anyhow!("zstd decoder error: {}", e))?;
                    let mut archive = tar::Archive::new(decoder);
                    Self::extract_tar_archive_to(&mut archive, dest)?;
                } else if name.ends_with(".gz") || name == "data.tar.gz" {
                    let decoder = flate2::read::GzDecoder::new(bounded);
                    let mut archive = tar::Archive::new(decoder);
                    Self::extract_tar_archive_to(&mut archive, dest)?;
                } else {
                    return Err(anyhow::anyhow!(
                        "Unknown compression in .deb data archive: {}",
                        name
                    ));
                }
                break; // data.tar is always the last meaningful entry
            }

            // Skip this entry, advance with even-byte padding
            let padded = if size % 2 == 0 { size } else { size + 1 };
            f.seek(SeekFrom::Start(entry_start + padded))?;
        }

        Ok(())
    }

    /// Extract every file from a `tar::Archive` into `dest`, stripping the
    /// leading `./` or first path component and guarding against path
    /// traversal.  Shared by both deb extraction paths.
    fn extract_tar_archive_to<R: std::io::Read>(
        archive: &mut tar::Archive<R>,
        dest: &PathBuf,
    ) -> Result<()> {
        for entry in archive.entries()? {
            let mut entry = entry?;
            let raw = entry.path()?.into_owned();

            // Guard: reject any path that escapes the destination
            if raw.components().any(|c| c == std::path::Component::ParentDir) {
                continue;
            }

            // Strip leading "./" component that many tarballs include
            let stripped: std::path::PathBuf = {
                let mut comps = raw.components().peekable();
                if let Some(std::path::Component::CurDir) = comps.peek() {
                    comps.next();
                }
                comps.collect()
            };

            if stripped.as_os_str().is_empty() {
                continue;
            }

            let dst = dest.join(&stripped);
            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&dst)?;
            } else {
                if let Some(p) = dst.parent() {
                    std::fs::create_dir_all(p)?;
                }
                entry.unpack(&dst)?;
            }
        }
        Ok(())
    }

    // ── MSI extraction (Windows only) ────────────────────────────────────────

    /// Extract an MSI package using `msiexec /a` (administrative install).
    ///
    /// `msiexec /a` extracts the MSI payload to `TARGETDIR` without running
    /// any installation scripts, registering anything in the Windows registry,
    /// or requiring elevation.  It is always available on any Windows system.
    #[cfg(target_os = "windows")]
    fn extract_msi_blocking(msi_path: &PathBuf, dest: &PathBuf) -> Result<()> {
        std::fs::create_dir_all(dest)?;

        // msiexec requires an absolute path for TARGETDIR
        let dest_abs = dest
            .canonicalize()
            .unwrap_or_else(|_| dest.clone());

        let status = std::process::Command::new("msiexec")
            .args([
                "/a",
                msi_path.to_str().unwrap_or(""),
                "/quiet",
                "/qn",
                &format!("TARGETDIR={}", dest_abs.display()),
            ])
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to launch msiexec: {}", e))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "msiexec /a exited with status {:?} for {}",
                status.code(),
                msi_path.display()
            ));
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn extract_msi_blocking(_msi_path: &PathBuf, _dest: &PathBuf) -> Result<()> {
        Err(anyhow::anyhow!(
            "MSI extraction is only supported on Windows"
        ))
    }

    fn extract_tar_gz_blocking(archive: &PathBuf, dest: &PathBuf) -> Result<()> {
        let file = std::fs::File::open(archive)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(gz);
        for entry in tar.entries()? {
            let mut entry = entry?;
            let raw = entry.path()?.into_owned();
            // Strip leading version directory (same logic as EngineDownloader)
            let mut comps = raw.components();
            let first = comps.next();
            let stripped = match first {
                Some(std::path::Component::CurDir) => {
                    let rest: PathBuf = comps.collect();
                    if rest.as_os_str().is_empty() {
                        continue;
                    }
                    let mut inner = rest.components();
                    inner.next();
                    let final_path: PathBuf = inner.collect();
                    if final_path.as_os_str().is_empty() {
                        continue;
                    }
                    final_path
                }
                Some(_) => {
                    let rest: PathBuf = comps.collect();
                    if rest.as_os_str().is_empty() {
                        continue;
                    }
                    rest
                }
                None => continue,
            };
            if stripped.components().any(|c| c == std::path::Component::ParentDir) {
                continue;
            }
            let dst = dest.join(&stripped);
            if let Some(p) = dst.parent() {
                std::fs::create_dir_all(p)?;
            }
            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&dst)?;
            } else {
                entry.unpack(&dst)?;
            }
        }
        Ok(())
    }
}

impl Default for RuntimeDepsManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_management::registry::{AccelerationType, EngineInfo, EngineStatus};
    use crate::model_runtime::platform_detector::{HardwareArchitecture, Platform};

    fn make_engine(id: &str, accel: AccelerationType) -> EngineInfo {
        EngineInfo {
            id: id.to_string(),
            name: id.to_string(),
            version: LLAMA_CPP_VERSION.to_string(),
            platform: Platform::Windows,
            architecture: HardwareArchitecture::X86_64,
            acceleration: accel,
            download_url: String::new(),
            file_size: 0,
            checksum: String::new(),
            compatibility_score: 90.0,
            status: EngineStatus::Installed,
            install_path: None,
            binary_name: "llama-server.exe".to_string(),
            required_dependencies: vec![],
        }
    }

    #[test]
    fn test_no_deps_for_cpu_engine() {
        let engine = make_engine("llama-cpu-windows-x64-b8037", AccelerationType::CPU);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(deps.is_empty(), "CPU engine should have no runtime deps");
    }

    #[test]
    fn test_no_deps_for_metal_engine() {
        let engine = make_engine("llama-metal-macos-arm64-b8037", AccelerationType::Metal);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(deps.is_empty(), "Metal engine should have no runtime deps");
    }

    #[test]
    fn test_cuda_124_dep_for_default_cuda_engine() {
        let engine = make_engine("llama-cuda-windows-x64-b8037", AccelerationType::CUDA);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::CudaRuntime124);
        assert!(deps[0].download_url.is_some(), "CUDA 12.4 dep should have a download URL");
    }

    #[test]
    fn test_cuda_131_dep_for_cuda131_engine() {
        let engine = make_engine("llama-cuda131-windows-x64-b8037", AccelerationType::CUDA);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::CudaRuntime131);
    }

    #[test]
    fn test_level_zero_dep_for_sycl_engine() {
        let engine = make_engine("llama-sycl-windows-x64-b8037", AccelerationType::Sycl);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::IntelLevelZero);
    }

    #[test]
    fn test_hip_dep_for_hip_engine() {
        // v2 behaviour: dispatch now returns platform-specific v2 kinds.
        // Windows: AmdHipWindowsRuntime — primary path is the engine bundle probe.
        //          AMD does not publish a standalone DLL ZIP, so download_url is None.
        // Linux:   AmdRocmLinuxRuntime  — AMD ROCm .deb with a real download URL.
        // The original AmdHipWindows kind (guidance-only) is preserved in the enum
        // and still returned by the original hip_deps() — it was not removed.
        let engine = make_engine("llama-hip-windows-x64-b8037", AccelerationType::Hip);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(!deps.is_empty(), "HIP engine must have runtime deps");
        #[cfg(target_os = "windows")]
        {
            assert_eq!(deps[0].kind, RuntimeDepKind::AmdHipWindowsRuntime);
            // On Windows the engine bundle probe is primary; no standalone DLL ZIP.
            assert!(deps[0].download_url.is_none());
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(deps[0].kind, RuntimeDepKind::AmdRocmLinuxRuntime);
            // URL is Some on Ubuntu, None on unknown distro — both are valid
        }
    }

    #[test]
    fn test_check_status_finds_file_in_temp_cache() {
        // Create a fake cache directory with a probe file
        let tmp = std::env::temp_dir().join("oi_dep_test_cache");
        let _ = std::fs::create_dir_all(&tmp);
        let fake_dll = tmp.join("cudart64_12.dll");
        std::fs::write(&fake_dll, b"fake").unwrap();

        let dep = RuntimeDep {
            kind: RuntimeDepKind::CudaRuntime124,
            name: "CUDA 12.4 Runtime",
            probe_files: &["cudart64_12.dll"],
            system_search_paths: vec![tmp.clone()],
            cache_subdir: "cuda-12.4",
            download_url: Some("https://example.com/fake.zip".to_string()),
            file_size: 0,
            install_guide_url: "https://example.com",
        };

        let status = RuntimeDepsManager::check_status(&dep);
        // Should find the file in system_search_paths (tmp dir)
        assert!(
            matches!(status, DepStatus::PresentOnSystem),
            "Expected PresentOnSystem, got {:?}",
            status
        );

        // Clean up
        let _ = std::fs::remove_file(&fake_dll);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn test_check_status_can_download_when_missing() {
        let dep = RuntimeDep {
            kind: RuntimeDepKind::CudaRuntime124,
            name: "CUDA 12.4 Runtime",
            probe_files: &["nonexistent_file_xyz.dll"],
            system_search_paths: vec![PathBuf::from("C:\\NonExistentPath")],
            cache_subdir: "cuda-12.4-test-missing",
            download_url: Some("https://example.com/fake.zip".to_string()),
            file_size: 0,
            install_guide_url: "https://example.com",
        };
        let status = RuntimeDepsManager::check_status(&dep);
        assert_eq!(status, DepStatus::CanDownload);
    }

    #[test]
    fn test_check_status_needs_manual_when_no_url() {
        let dep = RuntimeDep {
            kind: RuntimeDepKind::AmdHipWindows,
            name: "AMD HIP",
            probe_files: &["amdhip64_nonexistent_test.dll"],
            system_search_paths: vec![PathBuf::from("C:\\NonExistentPath")],
            cache_subdir: "amd-hip-test-missing",
            download_url: None,
            file_size: 0,
            install_guide_url: "https://example.com",
        };
        let status = RuntimeDepsManager::check_status(&dep);
        assert_eq!(status, DepStatus::NeedsManualInstall);
    }

    #[test]
    fn test_runtimes_dir_under_data_dir() {
        let runtimes = PathResolver::runtimes_dir();
        let base = PathResolver::desktop_data_dir();
        assert!(
            runtimes.starts_with(&base)
                || runtimes.starts_with(PathResolver::server_data_dir()),
            "runtimes_dir {:?} must be under app data root",
            runtimes
        );
    }

    // ── v2 auto-download dep builder tests ───────────────────────────────────

    #[test]
    fn test_hip_windows_auto_dep_kind_and_probes() {
        // AMD HIP Windows: the primary path is the engine bundle probe
        // (ensure_deps_ready checks for amdhip64.dll in the llama.cpp HIP ZIP).
        // AMD does not publish a standalone redistributable DLL ZIP, so
        // download_url is None — NeedsManualInstall is the last-resort fallback.
        let deps = RuntimeDepsManager::hip_windows_auto_deps();
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::AmdHipWindowsRuntime);
        assert!(
            !deps[0].probe_files.is_empty(),
            "AMD HIP Windows v2 dep must have probe files"
        );
        assert!(
            !deps[0].system_search_paths.is_empty(),
            "AMD HIP Windows v2 dep must have system search paths"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_rocm_linux_auto_dep_has_download_url() {
        let deps = RuntimeDepsManager::rocm_linux_auto_deps();
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::AmdRocmLinuxRuntime);
        // On Ubuntu, a concrete URL is selected; on unknown distros, None is expected.
        if let Some(url) = deps[0].download_url.as_deref() {
            assert!(url.contains("repo.radeon.com"), "ROCm dep URL must point to AMD's repo");
            assert!(url.ends_with(".deb"), "ROCm dep must be a .deb package");
        }
    }

    #[test]
    fn test_vulkan_windows_auto_dep_has_download_url() {
        let deps = RuntimeDepsManager::vulkan_windows_auto_deps();
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::VulkanLoaderWindows);
        assert!(
            deps[0].download_url.is_some(),
            "Vulkan Windows dep must have a download URL"
        );
        let url = deps[0].download_url.as_deref().unwrap();
        assert!(url.contains("lunarg.com"), "Vulkan Windows dep URL must point to LunarG");
        assert!(url.ends_with(".zip"), "Vulkan Windows dep must be a .zip");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_vulkan_linux_auto_dep_has_download_url() {
        let deps = RuntimeDepsManager::vulkan_linux_auto_deps();
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::VulkanLoaderLinux);
        if let Some(url) = deps[0].download_url.as_deref() {
            assert!(url.ends_with(".deb"), "Vulkan Linux dep must be a .deb");
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_sycl_linux_deps_has_download_url() {
        let deps = RuntimeDepsManager::sycl_linux_deps();
        assert!(!deps.is_empty());
        assert_eq!(deps[0].kind, RuntimeDepKind::IntelLevelZeroLinux);
        if let Some(url) = deps[0].download_url.as_deref() {
            assert!(
                url.contains("oneapi-src/level-zero"),
                "Level Zero Linux dep URL must point to Intel's GitHub"
            );
        }
    }

    #[test]
    fn test_required_deps_hip_engine_always_has_deps() {
        // HIP engine must always return a non-empty dep list so that
        // ensure_deps_ready's engine bundle probe and system path scan run.
        // On Linux the dep also has a download_url (AMD ROCm .deb).
        // On Windows the primary path is the engine bundle probe; download_url is None.
        let engine = make_engine("llama-hip-windows-x64-b8037", AccelerationType::Hip);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(!deps.is_empty(), "HIP engine must always have runtime deps");
        // On Linux/Ubuntu, download_url is populated; on unknown distro, None is fine
    }

    #[test]
    fn test_required_deps_vulkan_windows_returns_downloadable() {
        let engine = make_engine("llama-vulkan-windows-x64-b8037", AccelerationType::Vulkan);
        let deps = RuntimeDepsManager::required_deps_for_engine(&engine);
        assert!(!deps.is_empty(), "Vulkan engine must have runtime deps");
        assert!(
            deps[0].download_url.is_some(),
            "Vulkan engine runtime dep must be auto-downloadable"
        );
    }

    #[test]
    fn test_extract_deb_roundtrip() {
        // Build a minimal synthetic .deb (ar archive with data.tar.gz)
        // and verify extract_deb_blocking extracts a file from it.
        use std::io::Write;

        let tmp_dir = std::env::temp_dir().join("oi_deb_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let deb_path = tmp_dir.join("test.deb");
        let extract_dir = tmp_dir.join("extracted");
        std::fs::create_dir_all(&extract_dir).unwrap();

        // Build data.tar.gz in memory (contains one file: usr/lib/libtest.so)
        let data_tar_gz: Vec<u8> = {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            {
                let mut builder = tar::Builder::new(&mut enc);
                let content = b"fake shared library";
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, "usr/lib/libtest.so", content.as_ref()).unwrap();
                builder.finish().unwrap();
            }
            enc.finish().unwrap()
        };

        // Build the ar archive (.deb format)
        let mut deb: Vec<u8> = Vec::new();
        deb.extend_from_slice(b"!<arch>\n");

        // Helper to write one ar entry
        let mut write_ar_entry = |name: &str, content: &[u8]| {
            let mut hdr = format!(
                "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                name, "0", "0", "0", "100644", content.len()
            );
            // Ensure header is exactly 60 bytes
            hdr.truncate(60);
            deb.extend_from_slice(hdr.as_bytes());
            deb.extend_from_slice(content);
            if content.len() % 2 != 0 {
                deb.push(b'\n'); // ar padding byte
            }
        };

        write_ar_entry("debian-binary", b"2.0\n");
        write_ar_entry("control.tar.gz", b"placeholder");
        write_ar_entry("data.tar.gz", &data_tar_gz);

        std::fs::write(&deb_path, &deb).unwrap();

        // Extract and verify
        RuntimeDepsManager::extract_deb_blocking(&deb_path, &extract_dir).unwrap();
        assert!(
            extract_dir.join("usr/lib/libtest.so").exists(),
            "extract_deb_blocking must unpack files from data.tar.gz"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
