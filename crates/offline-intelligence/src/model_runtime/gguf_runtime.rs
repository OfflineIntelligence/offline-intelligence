
use async_trait::async_trait;
use super::runtime_trait::*;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tracing::{info, warn};
use tokio::time::sleep;

pub struct GGUFRuntime {
    config: Option<RuntimeConfig>,
    server_process: Option<Child>,
    http_client: reqwest::Client,
    base_url: String,
}

impl GGUFRuntime {
    pub fn new() -> Self {
        Self {
            config: None,
            server_process: None,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(600))
                .build()
                .unwrap_or_default(),
            base_url: String::new(),
        }
    }

    async fn start_server(&mut self, config: &RuntimeConfig) -> anyhow::Result<()> {
        let binary_path = config.runtime_binary.as_ref()
            .ok_or_else(|| anyhow::anyhow!("GGUF runtime requires runtime_binary path"))?;

        if !binary_path.exists() {
            return Err(anyhow::anyhow!(
                "llama-server binary not found at: {}",
                binary_path.display()
            ));
        }

        info!("Starting llama-server for GGUF model: {}", config.model_path.display());
        info!("  Binary: {}", binary_path.display());
        info!("  Port: {}", config.port);
        info!("  Context Size: {}", config.context_size);
        info!("  GPU Layers: {}", config.gpu_layers);

        if !config.model_path.exists() {
            return Err(anyhow::anyhow!(
                "Model file not found at: {}",
                config.model_path.display()
            ));
        }

        let mut cmd = Command::new(binary_path);
        cmd.arg("--model").arg(&config.model_path)
            .arg("--host").arg(&config.host)
            .arg("--port").arg(config.port.to_string())
            .arg("--ctx-size").arg(config.context_size.to_string())
            .arg("--batch-size").arg(config.batch_size.to_string())
            
            .arg("--ubatch-size").arg(config.ubatch_size.to_string())
            .arg("--threads").arg(config.threads.to_string())
            .arg("--n-gpu-layers").arg(config.gpu_layers.to_string())
            
            .arg("--parallel").arg(config.parallel_slots.to_string())
            
            .arg("--cont-batching")
            
            .arg("--flash-attn").arg("on")
            
            .arg("--cache-type-k").arg("q8_0")
            .arg("--cache-type-v").arg("q8_0")
            
            .arg("--defrag-thold").arg("0.1")
            
            .arg("--prio").arg("2")
            
            .arg("--mlock");

        if let Some(ref draft_path) = config.draft_model_path {
            if draft_path.exists() {
                cmd.arg("--model-draft").arg(draft_path)
                    .arg("--draft-max").arg(config.speculative_draft_max.to_string())
                    .arg("--draft-min").arg("1")
                    .arg("--draft-p-min").arg(config.speculative_draft_p_min.to_string());
                info!("Speculative decoding enabled: draft_model={}", draft_path.display());
            } else {
                info!("Speculative decoding disabled: draft model not found at {}", draft_path.display());
            }
        }

        info!("Full llama-server command: {:?} --model {} --host {} --port {} --ctx-size {} --batch-size {} --ubatch-size {} --threads {} --n-gpu-layers {} --parallel {} --cont-batching --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0 --defrag-thold 0.1 --prio 2 --mlock",
            binary_path,
            config.model_path.display(), config.host, config.port,
            config.context_size, config.batch_size, config.ubatch_size,
            config.threads, config.gpu_layers, config.parallel_slots);

        #[cfg(target_os = "macos")]
        {
            if let Some(binary_dir) = binary_path.parent() {
                let lib_path = binary_dir.to_string_lossy().to_string();
                info!("macOS: setting DYLD_LIBRARY_PATH={}", lib_path);
                
                let existing = std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
                let new_val = if existing.is_empty() {
                    lib_path
                } else {
                    format!("{}:{}", lib_path, existing)
                };
                cmd.env("DYLD_LIBRARY_PATH", new_val);
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn llama-server: {}", e))?;

        self.server_process = Some(child);
        self.base_url = format!("http://{}:{}", config.host, config.port);

        info!("llama-server process started, waiting for health check...");

        let _start = std::time::Instant::now();
        let mut delay_ms: u64 = 100;
        let mut last_log_secs: u64 = 0;
        loop {
            sleep(Duration::from_millis(delay_ms)).await;

            if self.is_ready().await {
                info!("✅ GGUF runtime ready after {:.1}s", _start.elapsed().as_secs_f64());

                let warmup_url = format!("{}/v1/chat/completions", self.base_url);
                let warmup_payload = serde_json::json!({
                    "model": "local-llm",
                    "messages": [{"role": "user", "content": "hi"}],
                    "max_tokens": 1,
                    "temperature": 0.0,
                    "stream": false,
                    "cache_prompt": true,
                });
                info!("Pre-warming CUDA kernels (max_tokens=1 dummy request)...");
                match self.http_client
                    .post(&warmup_url)
                    .json(&warmup_payload)
                    .timeout(Duration::from_secs(30))
                    .send()
                    .await
                {
                    Ok(_) => info!("CUDA pre-warm complete — first user request will get warm TTFT"),
                    Err(e) => warn!("CUDA pre-warm failed (non-fatal, first request may be slow): {}", e),
                }

                return Ok(());
            }
            let elapsed_secs = _start.elapsed().as_secs();
            if elapsed_secs >= 120 {
                break;
            }
            if elapsed_secs >= last_log_secs + 10 {
                info!("Still waiting for llama-server... ({}/120s)", elapsed_secs);
                last_log_secs = elapsed_secs;
            }
            delay_ms = (delay_ms * 2).min(2_000);
        }

        Err(anyhow::anyhow!("llama-server failed to become ready within 120 seconds"))
    }

    #[cfg(unix)]
    fn send_sigterm_and_wait(child: &mut Child, grace_secs: u64) -> bool {
        if let Some(pid) = child.id() {
            
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();

            let deadline = std::time::Instant::now() + Duration::from_secs(grace_secs);
            while std::time::Instant::now() < deadline {
                if let Ok(Some(_)) = child.try_wait() {
                    return true; 
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        false 
    }
}

impl Default for GGUFRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelRuntime for GGUFRuntime {
    fn supported_format(&self) -> ModelFormat {
        ModelFormat::GGUF
    }

    async fn initialize(&mut self, config: RuntimeConfig) -> anyhow::Result<()> {
        info!("Initializing GGUF runtime");

        if config.format != ModelFormat::GGUF {
            return Err(anyhow::anyhow!(
                "GGUF runtime received wrong format: {:?}",
                config.format
            ));
        }

        if !config.model_path.exists() {
            return Err(anyhow::anyhow!(
                "Model file not found: {}",
                config.model_path.display()
            ));
        }

        self.config = Some(config.clone());
        self.start_server(&config).await?;

        Ok(())
    }

    async fn is_ready(&self) -> bool {
        if self.base_url.is_empty() {
            return false;
        }

        let health_url = format!("{}/health", self.base_url);
        
        match self.http_client
            .get(&health_url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    async fn health_check(&self) -> anyhow::Result<String> {
        if self.base_url.is_empty() {
            return Err(anyhow::anyhow!("Runtime not initialized"));
        }

        let health_url = format!("{}/health", self.base_url);
        let resp = self.http_client.get(&health_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Health check failed: {}", e))?;

        if resp.status().is_success() {
            Ok("healthy".to_string())
        } else {
            Err(anyhow::anyhow!("Health check returned: {}", resp.status()))
        }
    }

    fn base_url(&self) -> String {
        self.base_url.clone()
    }

    async fn generate(
        &self,
        request: InferenceRequest,
    ) -> anyhow::Result<InferenceResponse> {
        let url = self.completions_url();

        let payload = serde_json::json!({
            "model": "local-llm",
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": false,
        });

        let resp = self.http_client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Inference request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Inference failed ({}): {}", status, body));
        }

        let response: serde_json::Value = resp.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = response["choices"][0]["finish_reason"]
            .as_str()
            .map(|s| s.to_string());

        Ok(InferenceResponse {
            content,
            finish_reason,
        })
    }

    async fn generate_stream(
        &self,
        request: InferenceRequest,
    ) -> anyhow::Result<Box<dyn futures_util::Stream<Item = Result<String, anyhow::Error>> + Send + Unpin>> {
        use futures_util::StreamExt;

        let url = self.completions_url();

        let payload = serde_json::json!({
            "model": "local-llm",
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": true,
        });

        let resp = self.http_client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Stream request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Stream failed ({}): {}", status, body));
        }

        let byte_stream = resp.bytes_stream();

        let sse_stream = async_stream::try_stream! {
            let mut buffer = String::new();
            futures_util::pin_mut!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = chunk_result.map_err(|e| anyhow::anyhow!("Stream read error: {}", e))?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() || !line.starts_with("data: ") {
                        continue;
                    }

                    let data = &line[6..];
                    if data == "[DONE]" {
                        return;
                    }

                    yield format!("data: {}\n\n", data);
                }
            }
        };

        Ok(Box::new(Box::pin(sse_stream)))
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        info!("Shutting down GGUF runtime");

        if let Some(mut child) = self.server_process.take() {
            
            #[cfg(unix)]
            {
                
                let exited_gracefully = Self::send_sigterm_and_wait(&mut child, 1);
                if exited_gracefully {
                    info!("llama-server shut down gracefully after SIGTERM");
                    return Ok(());
                }
                info!("llama-server did not exit after SIGTERM — sending SIGKILL");
            }

            match child.kill() {
                Ok(_) => {
                    info!("llama-server process killed");
                    
                    let _ = child.wait();
                }
                Err(e) => {
                    
                    warn!("Failed to kill llama-server (may have already exited): {}", e);
                }
            }
        }

        self.config = None;
        self.base_url.clear();
        Ok(())
    }

    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata {
            format: ModelFormat::GGUF,
            runtime_name: "llama.cpp (llama-server)".to_string(),
            version: "latest".to_string(),
            supports_gpu: true,
            supports_streaming: true,
        }
    }
}

impl Drop for GGUFRuntime {
    fn drop(&mut self) {
        if let Some(mut child) = self.server_process.take() {
            
            let _ = child.kill();
        }
    }
}
