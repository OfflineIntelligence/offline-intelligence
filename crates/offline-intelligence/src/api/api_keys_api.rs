//! API Keys Management Endpoints
//!
//! Provides REST endpoints for managing API keys:
//! - Save/update HuggingFace and OpenRouter keys (encrypted with machine-specific key, stored in SQLite)
//! - Retrieve keys (plaintext - decrypted using machine-specific key)
//! - Delete keys
//! - Mark keys as used with mode tracking
//!
//! Encryption/decryption is handled entirely inside `ApiKeysStore`.
//! This layer only deals with plain-text values.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::{
    memory_db::{ApiKeyType, ApiKeyRecord},
    shared_state::UnifiedAppState,
};

// ─── Request / Response types ────────────────────────────────────────────────

/// Request body for saving or updating an API key.
#[derive(Debug, Deserialize)]
pub struct SaveApiKeyRequest {
    pub key_type: String, // "huggingface" or "openrouter"
    pub value: String,    // Plain-text key value
}

/// Confirmation after saving an API key.
#[derive(Debug, Serialize)]
pub struct SaveApiKeyResponse {
    pub success: bool,
    pub message: String,
}

/// Response carrying a single API key value + metadata.
#[derive(Debug, Serialize)]
pub struct GetApiKeyResponse {
    pub key_type: String,
    pub value: Option<String>,
    pub created_at: Option<String>,
    pub last_used_at: Option<String>,
    pub last_mode: Option<String>,
    pub usage_count: Option<i64>,
}

/// Response carrying all API keys.
#[derive(Debug, Serialize)]
pub struct GetAllApiKeysResponse {
    pub keys: Vec<GetApiKeyResponse>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn record_to_response(record: &ApiKeyRecord, value: Option<String>) -> GetApiKeyResponse {
    GetApiKeyResponse {
        key_type: record.key_type.clone(),
        value,
        created_at: Some(record.created_at.to_rfc3339()),
        last_used_at: record.last_used_at.map(|dt| dt.to_rfc3339()),
        last_mode: record.last_mode.clone(),
        usage_count: Some(record.usage_count),
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// `POST /api-keys` — save or update an API key.
///
/// The plaintext value is encrypted using machine-specific encryption and stored in SQLite.
pub async fn save_api_key(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<SaveApiKeyRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let key_type = ApiKeyType::from_str(&payload.key_type).ok_or(StatusCode::BAD_REQUEST)?;

    info!("Saving API key for: {}", key_type.as_str());

    match state
        .shared_state
        .database_pool
        .api_keys
        .save_key(key_type, &payload.value)
    {
        Ok(_) => Ok(Json(SaveApiKeyResponse {
            success: true,
            message: format!("API key saved for {}", payload.key_type),
        })),
        Err(e) => {
            error!("Failed to save API key: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// `GET /api-keys?key_type=<type>` — retrieve a single API key (plaintext).
pub async fn get_api_key(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let key_type_str = params.get("key_type").ok_or(StatusCode::BAD_REQUEST)?;
    let key_type = ApiKeyType::from_str(key_type_str).ok_or(StatusCode::BAD_REQUEST)?;

    // Get plaintext (decrypts using machine-specific key).
    let value = state
        .shared_state
        .database_pool
        .api_keys
        .get_key_plaintext(&key_type)
        .map_err(|e| {
            error!("Failed to retrieve API key '{}': {}", key_type_str, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fetch metadata (created_at, last_used_at, …) for the response body.
    let metadata = state
        .shared_state
        .database_pool
        .api_keys
        .get_key_metadata(&key_type)
        .map_err(|e| {
            error!("Failed to get API key metadata '{}': {}", key_type_str, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(match metadata {
        Some(record) => record_to_response(&record, value),
        None => GetApiKeyResponse {
            key_type: key_type_str.clone(),
            value: None,
            created_at: None,
            last_used_at: None,
            last_mode: None,
            usage_count: None,
        },
    }))
}

/// `GET /api-keys/all` — retrieve all API keys with their plaintext values.
pub async fn get_all_api_keys(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    match state
        .shared_state
        .database_pool
        .api_keys
        .get_all_keys_with_values()
    {
        Ok(entries) => {
            let keys: Vec<GetApiKeyResponse> = entries
                .into_iter()
                .map(|(record, value)| record_to_response(&record, Some(value)))
                .collect();
            Ok(Json(GetAllApiKeysResponse { keys }))
        }
        Err(e) => {
            error!("Failed to retrieve all API keys: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// `DELETE /api-keys?key_type=<type>` — delete an API key from storage.
pub async fn delete_api_key(
    State(state): State<UnifiedAppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let key_type_str = params.get("key_type").ok_or(StatusCode::BAD_REQUEST)?;
    let key_type = ApiKeyType::from_str(key_type_str).ok_or(StatusCode::BAD_REQUEST)?;

    match state
        .shared_state
        .database_pool
        .api_keys
        .delete_key(key_type)
    {
        Ok(true) => {
            info!("API key deleted for: {}", key_type_str);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("API key deleted for {}", key_type_str)
            })))
        }
        Ok(false) => {
            warn!("API key not found for deletion: {}", key_type_str);
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("API key not found for {}", key_type_str)
            })))
        }
        Err(e) => {
            error!("Failed to delete API key: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// `POST /api-keys/mark-used` — record that a key was used with a given mode.
#[derive(Debug, Deserialize)]
pub struct MarkKeyUsedRequest {
    pub key_type: String,
    pub mode: String, // "offline" or "online"
}

pub async fn mark_key_used(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<MarkKeyUsedRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let key_type = ApiKeyType::from_str(&payload.key_type).ok_or(StatusCode::BAD_REQUEST)?;

    match state
        .shared_state
        .database_pool
        .api_keys
        .mark_used(key_type, &payload.mode)
    {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Key usage recorded"
        }))),
        Err(e) => {
            error!("Failed to mark key as used: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Request body for verifying an API key.
#[derive(Debug, Deserialize)]
pub struct VerifyApiKeyRequest {
    pub key_type: String,
    pub api_key: String,
}

/// Response for API key verification.
#[derive(Debug, Serialize)]
pub struct VerifyApiKeyResponse {
    pub valid: bool,
    pub message: String,
}

/// `POST /api-keys/verify` — verify an API key by calling the provider's API.
///
/// - For OpenRouter: Calls GET https://openrouter.ai/api/v1/key
/// - For HuggingFace: Calls GET https://huggingface.co/api/whoami
pub async fn verify_api_key(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<VerifyApiKeyRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let key_type = ApiKeyType::from_str(&payload.key_type).ok_or(StatusCode::BAD_REQUEST)?;

    if payload.api_key.trim().is_empty() {
        return Ok(Json(VerifyApiKeyResponse {
            valid: false,
            message: "API key cannot be empty".to_string(),
        }));
    }

    let client = reqwest::Client::new();
    let http_client = state.http_client.clone();

    match key_type {
        ApiKeyType::OpenRouter => {
            let url = "https://openrouter.ai/api/v1/key";
            match http_client
                .get(url)
                .header("Authorization", format!("Bearer {}", payload.api_key))
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info!("OpenRouter API key verified successfully, saving to database");
                        
                        // Save the verified key to the database
                        if let Err(e) = state.shared_state.database_pool.api_keys.save_key(key_type, &payload.api_key) {
                            error!("Failed to save verified OpenRouter API key: {}", e);
                            return Ok(Json(VerifyApiKeyResponse {
                                valid: false,
                                message: "Key verified but failed to save. Please try again.".to_string(),
                            }));
                        }
                        
                        Ok(Json(VerifyApiKeyResponse {
                            valid: true,
                            message: "OpenRouter API key is valid".to_string(),
                        }))
                    } else if resp.status() == StatusCode::UNAUTHORIZED || resp.status() == StatusCode::FORBIDDEN {
                        info!("OpenRouter API key verification failed: invalid credentials");
                        Ok(Json(VerifyApiKeyResponse {
                            valid: false,
                            message: "Invalid OpenRouter API key. Please check and try again.".to_string(),
                        }))
                    } else {
                        let status = resp.status();
                        error!("OpenRouter API key verification returned unexpected status: {}", status);
                        Ok(Json(VerifyApiKeyResponse {
                            valid: false,
                            message: format!("OpenRouter API error: {}", status),
                        }))
                    }
                }
                Err(e) => {
                    error!("Failed to verify OpenRouter API key: {}", e);
                    Ok(Json(VerifyApiKeyResponse {
                        valid: false,
                        message: "Failed to connect to OpenRouter API. Please check your internet connection.".to_string(),
                    }))
                }
            }
        }
        ApiKeyType::HuggingFace => {
            let url = "https://huggingface.co/api/whoami";
            match http_client
                .get(url)
                .header("Authorization", format!("Bearer {}", payload.api_key))
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info!("HuggingFace token verified successfully, saving to database");
                        
                        // Save the verified token to the database
                        if let Err(e) = state.shared_state.database_pool.api_keys.save_key(key_type, &payload.api_key) {
                            error!("Failed to save verified HuggingFace token: {}", e);
                            return Ok(Json(VerifyApiKeyResponse {
                                valid: false,
                                message: "Token verified but failed to save. Please try again.".to_string(),
                            }));
                        }
                        
                        Ok(Json(VerifyApiKeyResponse {
                            valid: true,
                            message: "HuggingFace token is valid".to_string(),
                        }))
                    } else if resp.status() == StatusCode::UNAUTHORIZED || resp.status() == StatusCode::FORBIDDEN {
                        info!("HuggingFace token verification failed: invalid credentials");
                        Ok(Json(VerifyApiKeyResponse {
                            valid: false,
                            message: "Invalid HuggingFace token. Please check and try again.".to_string(),
                        }))
                    } else {
                        let status = resp.status();
                        error!("HuggingFace API key verification returned unexpected status: {}", status);
                        Ok(Json(VerifyApiKeyResponse {
                            valid: false,
                            message: format!("HuggingFace API error: {}", status),
                        }))
                    }
                }
                Err(e) => {
                    error!("Failed to verify HuggingFace token: {}", e);
                    Ok(Json(VerifyApiKeyResponse {
                        valid: false,
                        message: "Failed to connect to HuggingFace API. Please check your internet connection.".to_string(),
                    }))
                }
            }
        }
    }
}
