/// Build script for offline-intelligence.
///
/// Reads the workspace-root `.env` file and emits `cargo:rustc-env` directives
/// for Google OAuth credentials so that `option_env!()` in thread_server.rs
/// picks them up during both `npm run tauri dev` and `npm run tauri build`
/// without requiring the developer to manually set shell env vars.
///
/// Any values already present in the real environment take priority — the
/// `.env` file only fills in missing entries.

fn main() {
    // Locate the workspace root (.env lives next to the top-level Cargo.toml).
    // CARGO_MANIFEST_DIR = <workspace>/crates/offline-intelligence
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()); // workspace root

    if let Some(root) = workspace_root {
        let env_path = root.join(".env");
        if env_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&env_path) {
                for line in contents.lines() {
                    let line = line.trim();
                    // Skip comments and blank lines.
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        // Forward OAuth credentials and SMTP settings so they're
                        // baked into the binary for distributed / installed builds
                        // (no .env file is bundled with the installer).
                        let allowed = matches!(
                            key,
                            "GOOGLE_CLIENT_ID"
                                | "GOOGLE_CLIENT_SECRET"
                                | "SMTP_HOST"
                                | "SMTP_PORT"
                                | "SMTP_USER"
                                | "SMTP_PASS"
                        );
                        if !allowed {
                            continue;
                        }
                        // Strip optional surrounding quotes from the value.
                        let value = value.trim().trim_matches('"').trim_matches('\'');
                        if value.is_empty() {
                            continue;
                        }
                        // Only set if not already overridden by the real environment.
                        if std::env::var(key).unwrap_or_default().is_empty() {
                            println!("cargo:rustc-env={}={}", key, value);
                        }
                    }
                }
            }
        }

        // Re-run this script whenever .env changes so incremental builds stay correct.
        println!(
            "cargo:rerun-if-changed={}",
            env_path.display()
        );
    }

    // Also re-run if the env vars are changed directly in the shell.
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_SECRET");
    println!("cargo:rerun-if-env-changed=SMTP_HOST");
    println!("cargo:rerun-if-env-changed=SMTP_PORT");
    println!("cargo:rerun-if-env-changed=SMTP_USER");
    println!("cargo:rerun-if-env-changed=SMTP_PASS");
}
