// _Aud.io/offline-intelligence/crates/src/main.rs

#[cfg(feature = "cli")]
use offline_intelligence::{config::Config, run_thread_server};
#[cfg(feature = "cli")]
use dotenvy::dotenv;

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    let cfg = Config::from_env()?;
    
    println!("🚀 Starting with thread-based architecture (only mode available)");
    run_thread_server(cfg).await
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("CLI feature not enabled. Enable with --features cli");
}