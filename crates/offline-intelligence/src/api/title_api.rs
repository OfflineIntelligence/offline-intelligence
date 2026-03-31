
use axum::{
    extract::{State, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::shared_state::UnifiedAppState;

#[derive(Debug, Deserialize)]
pub struct GenerateTitleRequest {
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    
    #[serde(default)]
    pub api_key: Option<String>,
    
    #[serde(default)]
    pub model_id: Option<String>,
}

fn default_max_tokens() -> u32 {
    20
}

#[derive(Debug, Serialize)]
pub struct GenerateTitleResponse {
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn generate_title(
    State(state): State<UnifiedAppState>,
    Json(req): Json<GenerateTitleRequest>,
) -> Result<Json<GenerateTitleResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Generating title for prompt ({} chars)", req.prompt.len());

    if req.prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Prompt cannot be empty".to_string(),
            }),
        ));
    }

    if let (Some(api_key), Some(model_id)) = (&req.api_key, &req.model_id) {
        if !api_key.is_empty() && !model_id.is_empty() {
            let model_id = model_id.replace("openrouter:", "").replace("openrouter/", "");
            let prompt_snippet = req.prompt.chars().take(200).collect::<String>();
            let user_content = format!(
                "Create a short chat title (1-5 words) that captures the topic of this message. \
                 Reply with ONLY the title, no quotes or punctuation:\n\"{prompt_snippet}\""
            );

            let client = reqwest::Client::new();
            let body = serde_json::json!({
                "model": model_id,
                "messages": [{ "role": "user", "content": user_content }],
                "max_tokens": req.max_tokens.min(20),
                "temperature": 0.3,
            });

            match client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(res) if res.status().is_success() => {
                    if let Ok(data) = res.json::<serde_json::Value>().await {
                        if let Some(title) = data["choices"][0]["message"]["content"]
                            .as_str()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                        {
                            info!("OpenRouter title: '{title}'");
                            return Ok(Json(GenerateTitleResponse {
                                title: title.to_string(),
                            }));
                        }
                    }
                }
                Ok(res) => {
                    warn!("OpenRouter title request returned HTTP {}", res.status());
                }
                Err(e) => {
                    warn!("OpenRouter title request failed: {e}");
                }
            }

        }
    }

    let title_instruction = format!(
        "User prompt: {}\n\n\
         Create a short, meaningful chat title using 1-5 words maximum that captures the essence of this prompt.",
        req.prompt
    );

    let llm_worker = state.llm_worker.clone();
    match llm_worker.generate_title(&title_instruction, req.max_tokens.min(20)).await {
        Ok(title) => {
            info!("Local LLM title: '{title}'");
            Ok(Json(GenerateTitleResponse { title }))
        }
        Err(e) => {
            info!("Title generation failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Title generation failed: {e}"),
                }),
            ))
        }
    }
}
