
use std::sync::{Arc, RwLock};
use futures_util::StreamExt;
use tracing::{info, debug, warn};
use serde::{Deserialize, Serialize};

use crate::{
    memory::Message,
    model_runtime::RuntimeManager,
};

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    
    cache_prompt: bool,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: Option<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Option<ChatDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ChatDelta {
    content: Option<String>,
}

pub struct LLMWorker {
    backend_url: String,
    http_client: reqwest::Client,
    runtime_manager: RwLock<Option<Arc<RuntimeManager>>>,
}

impl LLMWorker {
    
    pub fn new(shared_state: std::sync::Arc<crate::shared_state::SharedState>) -> Self {
        let backend_url = shared_state.config.backend_url.clone();
        Self {
            backend_url,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .unwrap_or_default(),
            runtime_manager: RwLock::new(None),
        }
    }

    pub fn new_with_backend(backend_url: String) -> Self {
        info!("LLM worker initialized with backend: {}", backend_url);
        Self {
            backend_url,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .unwrap_or_default(),
            runtime_manager: RwLock::new(None),
        }
    }

    pub fn set_runtime_manager(&self, runtime_manager: Arc<RuntimeManager>) {
        if let Ok(mut guard) = self.runtime_manager.write() {
            *guard = Some(runtime_manager);
            info!("✅ Runtime manager linked to LLM worker");
        } else {
            warn!("⚠️  Failed to acquire lock to set runtime manager on LLM worker");
        }
    }
    
    fn get_runtime_manager(&self) -> Option<Arc<RuntimeManager>> {
        self.runtime_manager.read().ok().and_then(|guard| (*guard).clone())
    }

    pub async fn is_runtime_ready(&self) -> bool {
        let has_runtime_manager = self.runtime_manager.read().is_ok();
        let result = if let Some(ref rm) = self.get_runtime_manager() {
            let ready = rm.is_ready().await;
            info!("LLMWorker is_ready: runtime_manager exists={}, is_ready={}", has_runtime_manager, ready);
            ready
        } else {
            info!("LLMWorker is_ready: no runtime_manager set");
            false
        };
        result
    }

    fn to_chat_messages(messages: &[Message]) -> Vec<ChatMessage> {
        messages.iter().map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect()
    }

    pub async fn generate_response(
        &self,
        _session_id: String,
        context: Vec<Message>,
    ) -> anyhow::Result<String> {
        debug!("LLM worker generating response (non-streaming)");

        let request = ChatCompletionRequest {
            model: "local-llm".to_string(),
            messages: Self::to_chat_messages(&context),
            max_tokens: 2000,
            temperature: 0.7,
            stream: false,
            cache_prompt: true,
        };

        let url = if let Some(ref rm) = self.get_runtime_manager() {
            
            if rm.is_ready().await {
                if let Some(base_url) = rm.get_base_url().await {
                    format!("{}/v1/chat/completions", base_url)
                } else {
                    
                    return Err(anyhow::anyhow!(
                        "Model engine is initializing. Please wait a moment and try again, or load a model from the Models panel."
                    ));
                }
            } else {
                
                return Err(anyhow::anyhow!(
                    "Model engine is not ready yet. Please load a model from the Models panel first."
                ));
            }
        } else {
            
            return Err(anyhow::anyhow!(
                "No model loaded. Please download an engine and load a model from the Models panel."
            ));
        };
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    anyhow::anyhow!(
                        "Cannot connect to local LLM server. Please download and load a model from the Models panel."
                    )
                } else {
                    anyhow::anyhow!("LLM backend request failed: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM backend returned {}: {}", status, body));
        }

        let completion: ChatCompletionResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))?;

        let content = completion.choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    pub async fn stream_response(
        &self,
        messages: Vec<Message>,
        max_tokens: u32,
        temperature: f32,
    ) -> anyhow::Result<impl futures_util::Stream<Item = Result<String, anyhow::Error>>> {
        debug!("LLM worker starting streaming response");

        let request = ChatCompletionRequest {
            model: "local-llm".to_string(),
            messages: Self::to_chat_messages(&messages),
            max_tokens,
            temperature,
            stream: true,
            cache_prompt: true,
        };

        let url = if let Some(ref rm) = self.get_runtime_manager() {
            
            if rm.is_ready().await {
                if let Some(base_url) = rm.get_base_url().await {
                    format!("{}/v1/chat/completions", base_url)
                } else {
                    
                    return Err(anyhow::anyhow!(
                        "Model engine is initializing. Please wait a moment and try again, or load a model from the Models panel."
                    ));
                }
            } else {
                
                return Err(anyhow::anyhow!(
                    "Model engine is not ready yet. Please load a model from the Models panel first."
                ));
            }
        } else {
            
            return Err(anyhow::anyhow!(
                "No model loaded. Please download an engine and load a model from the Models panel."
            ));
        };
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    anyhow::anyhow!(
                        "Cannot connect to local LLM server. Please download and load a model from the Models panel."
                    )
                } else {
                    anyhow::anyhow!("LLM backend request failed: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM backend returned {}: {}", status, body));
        }

        let byte_stream = response.bytes_stream();

        let sse_stream = async_stream::try_stream! {
            let mut buffer = String::new();

            futures_util::pin_mut!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = chunk_result
                    .map_err(|e| anyhow::anyhow!("Stream read error: {}", e))?;

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with("data: ") {
                        let data = &line[6..];

                        if data == "[DONE]" {
                            yield "data: [DONE]\n\n".to_string();
                            return;
                        }

                        match serde_json::from_str::<StreamChunk>(data) {
                            Ok(chunk) => {
                                let finished = chunk.choices.iter()
                                    .any(|c| c.finish_reason.is_some());

                                yield format!("data: {}\n\n", data);

                                if finished {
                                    yield "data: [DONE]\n\n".to_string();
                                    return;
                                }
                            }
                            Err(_) => {
                                yield format!("data: {}\n\n", data);
                            }
                        }
                    }
                }
            }

            yield "data: [DONE]\n\n".to_string();
        };

        Ok(sse_stream)
    }

    pub async fn batch_process(
        &self,
        prompts: Vec<(String, Vec<Message>)>,
    ) -> anyhow::Result<Vec<String>> {
        debug!("LLM worker batch processing {} prompts", prompts.len());

        let mut responses = Vec::new();
        for (session_id, messages) in prompts {
            match self.generate_response(session_id.clone(), messages).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    warn!("Batch item {} failed: {}", session_id, e);
                    responses.push(format!("Error: {}", e));
                }
            }
        }

        info!("Batch processed {} prompts", responses.len());
        Ok(responses)
    }

    pub async fn initialize_model(&self, model_path: &str) -> anyhow::Result<()> {
        debug!("LLM worker model init (HTTP proxy mode): {}", model_path);
        Ok(())
    }

    pub async fn generate_embeddings(
        &self,
        texts: Vec<String>,
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Generating embeddings for {} text(s) via llama-server", texts.len());

        let request = EmbeddingRequest {
            model: "local-llm".to_string(),
            input: texts,
        };

        let url = if let Some(ref rm) = self.get_runtime_manager() {
            
            if rm.is_ready().await {
                if let Some(base_url) = rm.get_base_url().await {
                    format!("{}/v1/embeddings", base_url)
                } else {
                    
                    return Err(anyhow::anyhow!(
                        "Model engine is initializing. Please wait a moment and try again."
                    ));
                }
            } else {
                
                return Err(anyhow::anyhow!(
                    "Model engine is not ready yet. Please load a model from the Models panel first."
                ));
            }
        } else {
            
            return Err(anyhow::anyhow!(
                "No model loaded. Please download an engine and load a model from the Models panel."
            ));
        };
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Embedding request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Embedding endpoint returned {}: {}", status, body));
        }

        let embedding_response: EmbeddingResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse embedding response: {}", e))?;

        let embeddings: Vec<Vec<f32>> = embedding_response.data
            .into_iter()
            .map(|d| d.embedding)
            .collect();

        debug!("Generated {} embeddings (dim={})",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0));

        Ok(embeddings)
    }

    pub async fn generate_title(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> anyhow::Result<String> {
        debug!("LLM worker generating title for prompt ({} chars)", prompt.len());

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        let request = ChatCompletionRequest {
            model: "local-llm".to_string(),
            messages: Self::to_chat_messages(&messages),
            max_tokens: max_tokens.min(20),
            temperature: 0.3,
            stream: false,
            cache_prompt: true,
        };

        let url = if let Some(ref rm) = self.get_runtime_manager() {
            
            if rm.is_ready().await {
                if let Some(base_url) = rm.get_base_url().await {
                    format!("{}/v1/chat/completions", base_url)
                } else {
                    
                    return Err(anyhow::anyhow!(
                        "Model engine is initializing. Please wait a moment and try again, or load a model from the Models panel."
                    ));
                }
            } else {
                
                return Err(anyhow::anyhow!(
                    "Model engine is not ready yet. Please load a model from the Models panel first."
                ));
            }
        } else {
            
            return Err(anyhow::anyhow!(
                "No model loaded. Please download an engine and load a model from the Models panel."
            ));
        };
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Title generation request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Title generation failed ({}): {}", status, body));
        }

        let completion: ChatCompletionResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse title response: {}", e))?;

        let title = completion.choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.trim().to_string())
            .unwrap_or_else(|| "New Chat".to_string());

        let title = title.trim_matches('"').trim_matches('\'').to_string();

        info!("Generated title: '{}'", title);
        Ok(title)
    }
}
