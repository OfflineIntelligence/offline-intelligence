
use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::time::Duration;
use futures::{StreamExt, Stream};
use tracing::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use bytes::Bytes;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::memory::Message;
use crate::llm_integration::LLMEngine;

#[derive(Clone)]
pub struct AppState {
    pub llm_engine: Arc<LLMEngine>,
    pub cfg: crate::config::Config,
    
}

async fn optimize_context(
    state: &AppState,
    messages: Vec<Message>,
    session_id: &str,
    user_query: Option<&str>,
) -> Vec<Message> {
    
    warn!("Context optimization not available in core library");
    messages
}

struct ConversationCapturer<S> {
    inner: S,
    state: AppState,
    session_id: String,
    original_context: Vec<Message>,
    accumulated_response: String,
}

impl<S> Stream for ConversationCapturer<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                
                let chunk_str = String::from_utf8_lossy(&bytes);
                for line in chunk_str.lines() {
                    if line.starts_with("data: ") && !line.contains("[DONE]") {
                        if let Ok(val) = serde_json::from_str::<Value>(&line[6..]) {
                            if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                                self.accumulated_response.push_str(content);
                            }
                        }
                    }
                }
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(None) => {
                
                let state = self.state.clone();
                let session_id = self.session_id.clone();
                let assistant_content = self.accumulated_response.clone();

                tokio::spawn(async move {
                    if !assistant_content.is_empty() {
                        
                        warn!("Assistant response saving not available in core library");
                    }
                });

                Poll::Ready(None)
            }
            other => other,
        }
    }
}

pub async fn generate_stream_endpoint(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Response, (StatusCode, String)> {
    let start_time = std::time::Instant::now();
    info!("📥 Received generate_stream request");
    
    let session_id = payload.get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("default_session")
        .to_string();

    let raw_messages = payload.get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let messages: Vec<Message> = match serde_json::from_value::<Vec<Message>>(Value::Array(raw_messages)) {
        Ok(msgs) => {
            if msgs.len() > 1000 {
                return Err((StatusCode::BAD_REQUEST, "Too many messages (max 1000)".into()));
            }
            msgs
        }
        Err(e) => {
            warn!("❌ Invalid messages format: {}", e);
            return Err((StatusCode::BAD_REQUEST, format!("Invalid messages format: {}", e)));
        }
    };

    let user_query = messages.last().map(|m| m.content.as_str());

    let context_start = std::time::Instant::now();
    info!("🔧 Starting context optimization...");
    let optimized_messages = optimize_context(&state, messages.clone(), &session_id, user_query).await;
    crate::metrics::observe_context_optimization(context_start.elapsed().as_secs_f64());
    
    let backend_start = std::time::Instant::now();
    let llm_stream = match state.llm_engine.generate_stream(optimized_messages, session_id.clone()).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("❌ LLM generation failed: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("LLM generation failed: {}", e)));
        }
    };
    crate::metrics::observe_backend_latency(backend_start.elapsed().as_secs_f64());

    let inner_stream = llm_stream.map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

    let captured_stream = ConversationCapturer {
        inner: inner_stream,
        state: state.clone(),
        session_id,
        original_context: messages, 
        accumulated_response: String::new(),
    };

    crate::metrics::inc_request("generate_stream", "ok");
    crate::metrics::observe_response_time("generate_stream", start_time.elapsed().as_secs_f64());

    let response_builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("x-accel-buffering", "no")
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "POST, OPTIONS")
        .header("access-control-allow-headers", "content-type");

    let body = Body::from_stream(captured_stream);
    
    match response_builder.body(body) {
        Ok(response) => Ok(response),
        Err(e) => {
            error!("❌ Failed to build streaming response: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create stream".into()))
        }
    }
}

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

pub fn extract_thinking(openai_response: &Value) -> Option<String> {
    let content = extract_openai_content(openai_response);
    if content.contains("<thoughts>") && content.contains("</thoughts>") {
        if let Some(start) = content.find("<thoughts>") {
            if let Some(end) = content.find("</thoughts>") {
                let thinking = content[start + 10..end].trim().to_string();
                if !thinking.is_empty() { return Some(thinking); }
            }
        }
    }
    None
}
