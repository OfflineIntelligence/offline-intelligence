
fn main() {
    
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent() 
        .and_then(|p| p.parent()); 

    if let Some(root) = workspace_root {
        let env_path = root.join(".env");
        if env_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&env_path) {
                for line in contents.lines() {
                    let line = line.trim();
                    
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        
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
