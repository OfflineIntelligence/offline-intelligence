
use axum::{
    extract::State,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    http::StatusCode,
    Json,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::convert::Infallible;
use tracing::{info, error, debug};
use reqwest;
use serde_json::Value;

use crate::memory::Message;
use crate::shared_state::UnifiedAppState;

#[derive(Debug, Deserialize)]
pub struct OnlineStreamRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub session_id: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_stream")]
    pub stream: bool,
    pub api_key: Option<String>, 
}

fn default_max_tokens() -> u32 { 2000 }
fn default_temperature() -> f32 { 0.7 }
fn default_stream() -> bool { true }

pub async fn online_stream(
    State(state): State<UnifiedAppState>,
    Json(req): Json<OnlineStreamRequest>,
) -> Response {
    info!("Online stream request for session: {}", req.session_id);

    debug!("Request api_key present: {}", 
        req.api_key.is_some()
    );

    let api_key = if let Some(key) = &req.api_key {
        if !key.is_empty() {
            key.clone()
        } else {
            state.get_openrouter_api_key().await.unwrap_or_default()
        }
    } else {
        state.get_openrouter_api_key().await.unwrap_or_default()
    };

    debug!("Final API key length: {}", api_key.len());

    if api_key.is_empty() {
        error!("OpenRouter API key is empty");
        return (StatusCode::UNAUTHORIZED, "OpenRouter API key not configured. Please add your API key in Settings.").into_response();
    }

    let openrouter_messages = req.messages.iter().map(|m| {
        serde_json::json!({
            "role": m.role,
            "content": m.content
        })
    }).collect::<Vec<_>>();

    let openrouter_request = serde_json::json!({
        "model": req.model,
        "messages": openrouter_messages,
        "max_tokens": req.max_tokens,
        "temperature": req.temperature,
        "stream": req.stream,
    });

    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://aud.io")
        .header("X-Title", "Aud.io")
        .json(&openrouter_request)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                error!("OpenRouter API error ({}): {}", status, body);
                return (StatusCode::BAD_GATEWAY, format!("OpenRouter API error: {}", body)).into_response();
            }

            let byte_stream = resp.bytes_stream();

            let sse_stream = async_stream::stream! {
                let mut buffer = String::new();

                futures_util::pin_mut!(byte_stream);

                while let Some(chunk_result) = byte_stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
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
                                        yield Ok::<_, Infallible>(Event::default().data("[DONE]"));
                                        return;
                                    }

                                    yield Ok(Event::default().data(data));
                                }
                            }
                        }
                        Err(e) => {
                            error!("Stream error: {}", e);
                            yield Ok(Event::default().data(
                                format!("{{\"error\": \"{}\"}}", e)
                            ));
                            break;
                        }
                    }
                }
            };

            Sse::new(sse_stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(std::time::Duration::from_secs(15))
                )
                .into_response()
        }
        Err(e) => {
            error!("Failed to connect to OpenRouter: {}", e);
            (StatusCode::BAD_GATEWAY, format!("Failed to connect to OpenRouter: {}", e)).into_response()
        }
    }
}
