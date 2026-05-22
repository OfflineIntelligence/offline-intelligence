// Title generation API: summarize first prompt into 1-5 word chat title via local LLM inference.
use axum::{
    extract::{State, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::shared_state::UnifiedAppState;

#[derive(Debug, Deserialize)]
pub struct GenerateTitleRequest {
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
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

/// Generate a concise chat title (1-5 words) from a user prompt using the local LLM.
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

    // Local LLM worker
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
