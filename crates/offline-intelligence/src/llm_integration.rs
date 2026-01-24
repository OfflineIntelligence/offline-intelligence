// Server/src/llm_integration.rs
// Direct LLM integration module for unified server architecture

use crate::config::Config;
use crate::memory::Message;
use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;
use bytes::Bytes;

pub struct LLMEngine {
    config: Config,
    backend_process: Arc<Mutex<Option<BackendProcess>>>,
}

struct BackendProcess {
    child: Child,
    port: u16,
    model_path: String,
}

impl LLMEngine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            backend_process: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        let model_path = self.config.model_path.clone();
        self.load_model(model_path).await
    }

    pub async fn load_model(&self, model_path: String) -> Result<()> {
        let mut process_guard = self.backend_process.lock().await;
        
        // Stop existing process if running
        if let Some(existing) = process_guard.as_mut() {
            info!("Stopping existing backend process on port {}", existing.port);
            let _ = existing.child.kill().await;
            let _ = existing.child.wait().await;
        }

        // Start new backend process
        let port = self.config.llama_port;
        let child = self.spawn_backend_process(&model_path, port).await?;
        
        let model_path_clone = model_path.clone();
        *process_guard = Some(BackendProcess {
            child,
            port,
            model_path: model_path_clone,
        });

        // Wait for backend to be ready
        self.wait_for_backend_ready(port).await?;

        info!("LLM engine initialized with model: {}", model_path);
        Ok(())
    }

    async fn spawn_backend_process(&self, model_path: &str, port: u16) -> Result<Child> {
        let mut args = vec![
            "--host".to_string(),
            self.config.llama_host.clone(),
            "--port".to_string(),
            port.to_string(),
            "-m".to_string(),
            model_path.to_string(),
            "-n".to_string(),
            "-1".to_string(),
            "--keep".to_string(),
            "32".to_string(),
            "-c".to_string(),
            self.config.ctx_size.to_string(),
            "-b".to_string(),
            self.config.batch_size.to_string(),
            "-t".to_string(),
            self.config.threads.to_string(),
        ];

        if self.config.gpu_layers > 0 {
            args.push("-ngl".to_string());
            args.push(self.config.gpu_layers.to_string());
        }

        let mut cmd = Command::new(&self.config.llama_bin);
        cmd.args(&args);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let child = cmd.spawn().context("Failed to spawn llama backend process")?;
        info!("Spawned LLM backend (pid={:?}) on port {}", child.id(), port);
        
        Ok(child)
    }

    async fn wait_for_backend_ready(&self, port: u16) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(self.config.health_timeout_seconds);
        
        loop {
            if self.probe_backend_health(port).await {
                info!("Backend is healthy on port {}", port);
                return Ok(());
            }
            
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Backend did not start within timeout"));
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn probe_backend_health(&self, port: u16) -> bool {
        let url = format!("http://{}:{}/health", self.config.llama_host, port);
        match reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn generate_stream(
        &self,
        messages: Vec<Message>,
        _session_id: String,
    ) -> Result<impl Stream<Item = Result<Bytes, std::io::Error>>> {
        let process_guard = self.backend_process.lock().await;
        
        let backend = process_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("LLM backend not initialized"))?;

        let openai_payload = json!({
            "messages": messages,
            "stream": true,
            "temperature": 0.7,
            "max_tokens": 2048
        });

        let url = format!("http://{}:{}/v1/chat/completions", self.config.llama_host, backend.port);
        
        let response = reqwest::Client::new()
            .post(&url)
            .timeout(Duration::from_secs(self.config.generate_timeout_seconds))
            .header("content-type", "application/json")
            .header("connection", "keep-alive")
            .header("accept", "text/event-stream")
            .json(&openai_payload)
            .send()
            .await
            .context("Failed to send request to LLM backend")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Backend status {}: {}", status, body));
        }

        // Convert response to stream of bytes
        let byte_stream = response
            .bytes_stream()
            .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

        Ok(byte_stream)
    }

    pub async fn get_backend_info(&self) -> Option<(String, u16)> {
        let process_guard = self.backend_process.lock().await;
        process_guard.as_ref().map(|b| (b.model_path.clone(), b.port))
    }

    pub async fn stop(&self) -> Result<()> {
        let mut process_guard = self.backend_process.lock().await;
        
        if let Some(mut backend) = process_guard.take() {
            info!("Stopping LLM backend on port {}...", backend.port);
            backend.child.kill().await.context("Failed to kill backend process")?;
            backend.child.wait().await?;
            info!("LLM backend process stopped.");
        }
        
        Ok(())
    }
}

// Helper function to extract content from OpenAI-style responses
pub fn extract_openai_content(openai_response: &Value) -> String {
    openai_response["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            openai_response["choices"][0]["text"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "No response content found".to_string())
        })
}

// Helper function to extract thinking content
pub fn extract_thinking(openai_response: &Value) -> Option<String> {
    let content = extract_openai_content(openai_response);
    if content.contains("<thoughts>") && content.contains("</thoughts>") {
        if let Some(start) = content.find("<thoughts>") {
            if let Some(end) = content.find("</thoughts>") {
                let thinking = content[start + 10..end].trim().to_string();
                if !thinking.is_empty() {
                    return Some(thinking);
                }
            }
        }
    }
    None
}