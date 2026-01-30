use axum::{
    extract::{State, Path},
    response::{IntoResponse, Response},
    Json,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, error};
use crate::shared_state::UnifiedAppState;
/
#[derive(Debug, Serialize)]
pub struct ConversationsResponse {
    pub conversations: Vec<ConversationSummary>,
}
/
#[derive(Debug, Serialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub last_accessed: String,
    pub message_count: usize,
    pub pinned: bool,
}
/
#[derive(Debug, Serialize)]
pub struct ConversationDetailResponse {
    pub id: String,
    pub title: String,
    pub messages: Vec<MessageResponse>,
}
/
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub role: String,
    pub content: String,
}
/
pub async fn get_conversations(
    State(state): State<UnifiedAppState>,
) -> Result<Json<ConversationsResponse>, Response> {
    info!("Fetching all conversations");

    let orchestrator_lock = state.context_orchestrator.read().await;

    if let Some(ref orchestrator) = *orchestrator_lock {
        match orchestrator.database().conversations.get_all_sessions() {
            Ok(sessions) => {
                let mut conversations = Vec::new();

                for session in sessions {


                    if let Some(ref title) = session.metadata.title {

                        let message_count = orchestrator.database().conversations
                            .get_session_message_count(&session.id)
                            .unwrap_or(0);

                        conversations.push(ConversationSummary {
                            id: session.id.clone(),
                            title: title.clone(),
                            created_at: session.created_at.to_rfc3339(),
                            last_accessed: session.last_accessed.to_rfc3339(),
                            message_count,
                            pinned: session.metadata.pinned,
                        });
                    }
                }

                info!("Found {} conversations", conversations.len());
                Ok(Json(ConversationsResponse { conversations }))
            }
            Err(e) => {
                error!("Failed to fetch conversations: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response())
            }
        }
    } else {
        error!("Context orchestrator not initialized");
        Err((StatusCode::SERVICE_UNAVAILABLE, "Memory system not available").into_response())
    }
}
/
pub async fn get_conversation(
    State(state): State<UnifiedAppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ConversationDetailResponse>, Response> {
    info!("Fetching conversation: {}", session_id);

    let orchestrator_lock = state.context_orchestrator.read().await;

    if let Some(ref orchestrator) = *orchestrator_lock {

        let session = match orchestrator.database().conversations.get_session(&session_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                return Err((StatusCode::NOT_FOUND, "Conversation not found").into_response());
            }
            Err(e) => {
                error!("Failed to fetch session: {}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response());
            }
        };


        let messages = match orchestrator.database().conversations.get_session_messages(&session_id, None, None) {
            Ok(msgs) => msgs.into_iter()
                .map(|msg| MessageResponse {
                    role: msg.role,
                    content: msg.content,
                })
                .collect(),
            Err(e) => {
                error!("Failed to fetch messages: {}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response());
            }
        };

        Ok(Json(ConversationDetailResponse {
            id: session.id,
            title: session.metadata.title.unwrap_or_else(|| "New Chat".to_string()),
            messages,
        }))
    } else {
        error!("Context orchestrator not initialized");
        Err((StatusCode::SERVICE_UNAVAILABLE, "Memory system not available").into_response())
    }
}
/
#[derive(Debug, Deserialize)]
pub struct UpdateTitleRequest {
    pub title: String,
}
/
pub async fn update_conversation_title(
    State(state): State<UnifiedAppState>,
    Path(session_id): Path<String>,
    Json(req): Json<UpdateTitleRequest>,
) -> Result<Json<Value>, Response> {
    info!("Updating title for conversation: {}", session_id);

    if req.title.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Title cannot be empty").into_response());
    }

    let orchestrator_lock = state.context_orchestrator.read().await;

    if let Some(ref orchestrator) = *orchestrator_lock {
        match orchestrator.database().conversations.update_session_title(&session_id, &req.title) {
            Ok(_) => {
                info!("Successfully updated title for conversation: {}", session_id);
                Ok(Json(serde_json::json!({
                    "success": true,
                    "id": session_id,
                    "title": req.title
                })))
            }
            Err(e) => {

                error!("Failed to update conversation title for session {}: {}", session_id, e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response())
            }
        }
    } else {
        error!("Context orchestrator not initialized");
        Err((StatusCode::SERVICE_UNAVAILABLE, "Memory system not available").into_response())
    }
}
/
/
/
pub async fn delete_conversation(
    State(state): State<UnifiedAppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, Response> {
    info!("Deleting conversation: {}", session_id);

    let orchestrator_lock = state.context_orchestrator.read().await;

    if let Some(ref orchestrator) = *orchestrator_lock {
        match orchestrator.database().conversations.delete_session(&session_id) {
            Ok(deleted_count) => {
                if deleted_count == 0 {
                    info!("Conversation not found for deletion: {}", session_id);
                    Err((StatusCode::NOT_FOUND, format!("Conversation not found: {}", session_id)).into_response())
                } else {
                    info!("Successfully deleted conversation: {}", session_id);
                    Ok(Json(serde_json::json!({
                        "success": true,
                        "id": session_id
                    })))
                }
            }
            Err(e) => {
                error!("Failed to delete conversation: {}", e);

                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response())
            }
        }
    } else {
        error!("Context orchestrator not initialized");
        Err((StatusCode::SERVICE_UNAVAILABLE, "Memory system not available").into_response())
    }
}
/
#[derive(Debug, Deserialize)]
pub struct UpdatePinnedRequest {
    pub pinned: bool,
}
/
pub async fn update_conversation_pinned(
    State(state): State<UnifiedAppState>,
    Path(session_id): Path<String>,
    Json(req): Json<UpdatePinnedRequest>,
) -> Result<Json<Value>, Response> {
    info!("Updating pinned status for conversation: {} to {}", session_id, req.pinned);

    let orchestrator_lock = state.context_orchestrator.read().await;

    if let Some(ref orchestrator) = *orchestrator_lock {
        match orchestrator.database().conversations.update_session_pinned(&session_id, req.pinned) {
            Ok(_) => {
                info!("Successfully updated pinned status for conversation: {}", session_id);
                Ok(Json(serde_json::json!({
                    "success": true,
                    "id": session_id,
                    "pinned": req.pinned
                })))
            }
            Err(e) => {
                let error_msg = e.to_string();

                if error_msg.contains("not found") {
                    error!("Conversation not found: {}", session_id);
                    Err((StatusCode::NOT_FOUND, format!("Conversation not found: {}", session_id)).into_response())
                } else {
                    error!("Failed to update conversation pinned status: {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response())
                }
            }
        }
    } else {
        error!("Context orchestrator not initialized");
        Err((StatusCode::SERVICE_UNAVAILABLE, "Memory system not available").into_response())
    }
}


