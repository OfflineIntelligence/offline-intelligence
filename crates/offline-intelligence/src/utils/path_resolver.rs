//! Centralised path resolution for all application data directories.
//!
//! All code that needs to locate engines, models, databases, or other
//! application data must go through `PathResolver` rather than calling
//! `dirs` functions directly.  This single indirection makes it trivial to
//! switch between desktop mode and on-prem server mode without touching any
//! other code.
//!
//! ## Single app-data folder layout
//!
//! ```text
//! Desktop (Windows):  C:\Users\<user>\AppData\Local\OfflineIntelligence\
//! Desktop (macOS):    ~/Library/Application Support/OfflineIntelligence/
//! Desktop (Linux):    ~/.local/share/OfflineIntelligence/
//! Server  (Linux):    /var/lib/offline-intelligence/
//! Server  (Windows):  C:\ProgramData\OfflineIntelligence\
//!
//! Sub-directories (same on every platform):
//!   engines/          — downloaded llama.cpp engine binaries
//!   models/           — downloaded GGUF / ONNX / etc. model files
//!   models/registry/  — per-model JSON metadata
//!   data/             — SQLite database (memory.db) and small state files
//!   all_files/        — user-uploaded files indexed by the server
//! ```
//!
//! ## Resolution order
//! 1. `DATA_DIR` env var — explicit admin override, highest priority.
//! 2. `DEPLOYMENT_MODE=server` env var — use system-wide server paths.
//! 3. Auto-detect: if the process is running as root/SYSTEM, assume server mode.
//! 4. Desktop paths: `AppData\Local` (Windows), standard XDG/app-support elsewhere.
//!
//! ## One-time migration
//! Older builds stored data under `AppData\Roaming\OfflineIntelligence` on Windows
//! (because `dirs::data_dir()` maps to Roaming).  On first run this module silently
//! moves the existing directory to `AppData\Local\OfflineIntelligence` so no data
//! is lost.

use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{info, warn};

static RESOLVED_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentMode {
    Desktop,
    Server,
}

pub struct PathResolver;

impl PathResolver {
    // ─── Public API ──────────────────────────────────────────────────────────

    /// Root data directory for all application files (the "single app data folder").
    ///
    /// The result is computed once and cached for the lifetime of the process.
    pub fn data_dir() -> PathBuf {
        RESOLVED_DATA_DIR
            .get_or_init(|| {
                let dir = Self::resolve_data_dir();
                // Ensure the directory exists before anything else tries to use it.
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    warn!("Could not create app data directory {}: {}", dir.display(), e);
                }
                info!("Application data directory: {}", dir.display());
                dir
            })
            .clone()
    }

    /// Directory where engine binaries are stored.
    pub fn engines_dir() -> PathBuf {
        Self::data_dir().join("engines")
    }

    /// Directory where model files are stored.
    pub fn models_dir() -> PathBuf {
        Self::data_dir().join("models")
    }

    /// Path to the SQLite database file.
    pub fn db_path() -> PathBuf {
        Self::data_dir().join("data").join("memory.db")
    }

    /// Path to the file that persists the last-loaded model path.
    pub fn last_model_path() -> PathBuf {
        Self::data_dir().join("last_model.txt")
    }

    /// Directory for user-uploaded / indexed files.
    pub fn files_dir() -> PathBuf {
        Self::data_dir().join("all_files")
    }

    /// Path to the feedback log file.
    pub fn feedback_log_path() -> PathBuf {
        Self::data_dir().join("feedback.log")
    }

    /// Path to the user-editable `.env` override inside the data directory.
    pub fn user_env_path() -> PathBuf {
        Self::data_dir().join(".env")
    }

    /// Path to the sequential user-login counter file.
    pub fn user_counter_path() -> PathBuf {
        Self::data_dir().join("data").join("user_counter.txt")
    }

    /// Directory where runtime dependency packages are cached.
    ///
    /// Each backend gets its own sub-directory:
    /// ```text
    /// runtimes/
    ///   cuda-12.4/    ← CUDA 12.4 runtime DLLs (cudart, cublas, …)
    ///   cuda-13.1/    ← CUDA 13.1 runtime DLLs
    ///   level-zero/   ← Intel Level Zero loader
    /// ```
    pub fn runtimes_dir() -> PathBuf {
        Self::data_dir().join("runtimes")
    }

    /// Directory for TLS certificate and private key files.
    ///
    /// Default locations:
    /// ```text
    /// Server  (Linux):   /var/lib/offline-intelligence/tls/
    /// Server  (Windows): C:\ProgramData\OfflineIntelligence\tls\
    /// Desktop (all):     <AppData>\OfflineIntelligence\tls\
    /// ```
    ///
    /// Expected files (configure via .env):
    ///   TLS_CERT_PATH = <tls_dir>/server.crt
    ///   TLS_KEY_PATH  = <tls_dir>/server.key
    pub fn tls_dir() -> PathBuf {
        Self::data_dir().join("tls")
    }

    /// Returns `true` when running in server deployment mode.
    pub fn is_server_mode() -> bool {
        Self::deployment_mode() == DeploymentMode::Server
    }

    /// Determine the deployment mode in use.
    pub fn deployment_mode() -> DeploymentMode {
        match std::env::var("DEPLOYMENT_MODE").as_deref() {
            Ok("server") => return DeploymentMode::Server,
            Ok("desktop") => return DeploymentMode::Desktop,
            _ => {}
        }

        if Self::is_running_as_system_account() {
            return DeploymentMode::Server;
        }

        DeploymentMode::Desktop
    }

    // ─── Internal resolution ──────────────────────────────────────────────────

    fn resolve_data_dir() -> PathBuf {
        // 1. Explicit override — highest priority.
        if let Ok(explicit) = std::env::var("DATA_DIR") {
            if !explicit.is_empty() {
                info!("DATA_DIR override in use: {}", explicit);
                return PathBuf::from(explicit);
            }
        }

        // 2 & 3. Mode-based resolution.
        match Self::deployment_mode() {
            DeploymentMode::Server => Self::server_data_dir(),
            DeploymentMode::Desktop => {
                let target = Self::desktop_data_dir();
                // On Windows: migrate from the old Roaming location if it still exists
                // and the Local location doesn't yet.
                #[cfg(target_os = "windows")]
                Self::migrate_roaming_to_local_if_needed(&target);
                target
            }
        }
    }

    /// Desktop data directory.
    ///
    /// Windows: `%LOCALAPPDATA%\OfflineIntelligence`  (local, non-roaming)
    /// macOS:   `~/Library/Application Support/OfflineIntelligence`
    /// Linux:   `~/.local/share/OfflineIntelligence`
    ///
    /// `dirs::data_local_dir()` and `dirs::data_dir()` return the same path on
    /// macOS and Linux; on Windows `data_local_dir()` → `AppData\Local` while
    /// `data_dir()` → `AppData\Roaming`.  We always want Local for large binary
    /// data (engines, models) so we use `data_local_dir()` on all platforms.
    pub fn desktop_data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| {
                // Fallback: exe directory
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .unwrap_or_else(|| PathBuf::from("."))
            })
            .join("OfflineIntelligence")
    }

    /// Server data directory.
    ///
    /// Linux:   `/var/lib/offline-intelligence`
    /// Windows: `%ProgramData%\OfflineIntelligence`
    pub fn server_data_dir() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            let program_data =
                std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
            PathBuf::from(program_data).join("OfflineIntelligence")
        }
        #[cfg(not(target_os = "windows"))]
        {
            PathBuf::from("/var/lib/offline-intelligence")
        }
    }

    /// Returns true when the process is running as a privileged service account.
    fn is_running_as_system_account() -> bool {
        #[cfg(target_os = "windows")]
        {
            if let Ok(profile) = std::env::var("USERPROFILE") {
                return profile.to_lowercase().contains("system32");
            }
            false
        }
        #[cfg(not(target_os = "windows"))]
        {
            let username = std::env::var("USER").unwrap_or_default();
            let home = std::env::var("HOME").unwrap_or_default();
            username == "root" || home == "/root"
        }
    }

    /// One-time migration: if `AppData\Roaming\OfflineIntelligence` exists and
    /// the new `AppData\Local\OfflineIntelligence` does not, move the directory
    /// so existing data is preserved.
    ///
    /// Silently skips if the old directory does not exist or the move fails
    /// (the server will create a fresh data directory in the new location).
    #[cfg(target_os = "windows")]
    fn migrate_roaming_to_local_if_needed(local_target: &PathBuf) {
        // Build the old Roaming path using dirs::data_dir() (which is Roaming on Windows).
        let old_roaming = match dirs::data_dir() {
            Some(d) => d.join("OfflineIntelligence"),
            None => return,
        };

        if !old_roaming.exists() {
            return; // Nothing to migrate.
        }
        if local_target.exists() {
            // Both exist — the migration already happened or the user has data in
            // both places.  Leave both untouched to avoid data loss; the server
            // will use the Local path from now on.
            info!(
                "Both Roaming ({}) and Local ({}) data dirs exist — using Local.",
                old_roaming.display(),
                local_target.display()
            );
            return;
        }

        // Ensure the parent of the Local target exists.
        if let Some(parent) = local_target.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("Migration: could not create Local parent dir: {}", e);
                return;
            }
        }

        match std::fs::rename(&old_roaming, local_target) {
            Ok(_) => {
                info!(
                    "✅ Migrated app data from Roaming → Local: {} → {}",
                    old_roaming.display(),
                    local_target.display()
                );
            }
            Err(e) => {
                // rename() fails across drives. Fall back to a recursive copy + delete.
                warn!(
                    "Fast rename failed ({}), attempting copy migration…",
                    e
                );
                if Self::copy_dir_recursive(&old_roaming, local_target).is_ok() {
                    // Best-effort removal of the old directory.
                    let _ = std::fs::remove_dir_all(&old_roaming);
                    info!(
                        "✅ Copied app data from Roaming → Local (old directory removed)."
                    );
                } else {
                    warn!(
                        "Could not migrate app data from {} to {}. \
                         The server will use the Local path for new data.",
                        old_roaming.display(),
                        local_target.display()
                    );
                }
            }
        }
    }

    /// Recursively copy a directory tree from `src` to `dst`.
    #[cfg(target_os = "windows")]
    fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_data_dir_not_empty() {
        let dir = PathResolver::desktop_data_dir();
        assert!(!dir.as_os_str().is_empty());
        assert!(dir.ends_with("OfflineIntelligence"));
    }

    #[test]
    fn test_desktop_data_dir_is_local_not_roaming_on_windows() {
        // On Windows the desktop path must use AppData\Local, NOT AppData\Roaming.
        #[cfg(target_os = "windows")]
        {
            let dir = PathResolver::desktop_data_dir().to_string_lossy().to_lowercase();
            assert!(
                dir.contains("local"),
                "Expected AppData\\Local in path, got: {}",
                dir
            );
            assert!(
                !dir.contains("roaming"),
                "Path must NOT use AppData\\Roaming, got: {}",
                dir
            );
        }
    }

    #[test]
    fn test_server_data_dir() {
        let dir = PathResolver::server_data_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_sub_paths_are_under_data_dir() {
        let base = PathResolver::desktop_data_dir();
        // engines_dir must be under either desktop or server base.
        assert!(
            PathResolver::engines_dir().starts_with(&base)
                || PathResolver::engines_dir().starts_with(PathResolver::server_data_dir())
        );
    }

    #[test]
    fn test_data_dir_env_override() {
        std::env::set_var("DATA_DIR", "/tmp/test_override");
        let dir = PathResolver::resolve_data_dir();
        std::env::remove_var("DATA_DIR");
        assert_eq!(dir, PathBuf::from("/tmp/test_override"));
    }

    #[test]
    fn test_deployment_mode_env() {
        std::env::set_var("DEPLOYMENT_MODE", "server");
        assert_eq!(PathResolver::deployment_mode(), DeploymentMode::Server);
        std::env::set_var("DEPLOYMENT_MODE", "desktop");
        assert_eq!(PathResolver::deployment_mode(), DeploymentMode::Desktop);
        std::env::remove_var("DEPLOYMENT_MODE");
    }

    #[test]
    fn test_user_counter_path_inside_data_dir() {
        let counter = PathResolver::user_counter_path();
        let base = PathResolver::desktop_data_dir();
        // Counter must be inside data_dir (or server_data_dir in CI).
        assert!(
            counter.starts_with(&base)
                || counter.starts_with(PathResolver::server_data_dir()),
            "user_counter_path {:?} is not under data dir {:?}",
            counter,
            base
        );
    }

    #[test]
    fn test_all_sub_paths_end_under_offline_intelligence() {
        // Every path method must produce a path that is a descendant of the
        // "OfflineIntelligence" root directory — never a stray location.
        let base = PathResolver::desktop_data_dir();
        let paths = [
            PathResolver::engines_dir(),
            PathResolver::models_dir(),
            PathResolver::files_dir(),
            PathResolver::feedback_log_path(),
            PathResolver::user_env_path(),
            PathResolver::user_counter_path(),
            PathResolver::runtimes_dir(),
            PathResolver::tls_dir(),
        ];
        for p in &paths {
            assert!(
                p.starts_with(&base)
                    || p.starts_with(PathResolver::server_data_dir()),
                "Path {:?} escapes the app data root {:?}",
                p,
                base
            );
        }
    }
}
