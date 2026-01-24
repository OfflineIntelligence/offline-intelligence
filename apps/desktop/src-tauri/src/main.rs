// apps/desktop/src-tauri/src/main.rs

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use offline_intelligence::{Config, run_server};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            let mut config = Config::from_env()
                .expect("Unable to build Config from environment. Check .env and required vars.");

            config.api_host = "127.0.0.1".to_string();
            config.api_port = 8000; // Match frontend configuration
            config.max_concurrent_streams = 4;
            config.generate_timeout_seconds = 300;

            if let Err(e) = run_server(config).await {
                eprintln!("Server failed to start: {}", e);
            }
        });
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}