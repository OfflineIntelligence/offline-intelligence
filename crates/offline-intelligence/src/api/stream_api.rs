//!
//! Flow: Client POST â†’ SharedState (session + cache lookup) â†’ LLM Worker (HTTP to llama-server) â†’ SSE stream back
//! All state access is in-process via Arc/shared memory. The only network hop is to localhost llama-server.
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
use crate::memory::Message;
use crate::memory_db::schema::Embedding;
use crate::shared_state::UnifiedAppState;
/
#[derive(Debug, Deserialize)]
pub struct StreamChatRequest {
    pub model: Option<String>,
    pub messages: Vec<Message>,
    pub session_id: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_stream")]
    pub stream: bool,
}
fn default_max_tokens() -> u32 { 2000 }
fn default_temperature() -> f32 { 0.7 }
fn default_stream() -> bool { true }
/
/
/
/
/
/
pub async fn generate_stream(
    State(state): State<UnifiedAppState>,
    Json(req): Json<StreamChatRequest>,
) -> Response {
    let request_num = state.shared_state.counters.inc_total_requests();
    info!("Stream request #{} for session: {}", request_num, req.session_id);
    if req.messages.is_empty() {
        return (StatusCode::BAD_REQUEST, "Messages array cannot be empty").into_response();
    }
    let session_id = req.session_id.clone();

    let session = state.shared_state.get_or_create_session(&session_id).await;

    {
        if let Ok(mut session_data) = session.write() {
            session_data.last_accessed = std::time::Instant::now();
            session_data.messages = req.messages.clone();
        }
    }


    let user_msg_content = req.messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone());
    if let Some(ref content) = user_msg_content {
        let db = state.shared_state.database_pool.clone();
        let sid = session_id.clone();
        let content = content.clone();
        let msg_count = req.messages.len() as i32;
        tokio::spawn(async move {

            let _ = db.conversations.create_session_with_id(&sid, None);

            if let Err(e) = db.conversations.store_messages_batch(
                &sid,
                &[("user".to_string(), content, msg_count - 1, 0, 0.5)],
            ) {
                error!("Failed to persist user message: {}", e);
            }
        });
    }





    let context_messages = {
        let orchestrator_guard = state.context_orchestrator.read().await;
        if let Some(ref orchestrator) = *orchestrator_guard {
            let user_query = user_msg_content.as_deref();
            match orchestrator.process_conversation(&session_id, &req.messages, user_query).await {
                Ok(optimized) => {
                    if optimized.len() != req.messages.len() {
                        info!("Context engine optimized: {} â†’ {} messages (retrieved past context)",
                            req.messages.len(), optimized.len());
                    }
                    optimized
                }
                Err(e) => {
                    error!("Context engine error (falling back to raw messages): {}", e);
                    req.messages.clone()
                }
            }
        } else {
            debug!("Context orchestrator not initialized, using raw messages");
            req.messages.clone()
        }
    };

    let llm_worker = state.llm_worker.clone();
    let max_tokens = req.max_tokens;
    let temperature = req.temperature;
    let db_for_persist = state.shared_state.database_pool.clone();
    let session_id_for_persist = session_id.clone();
    let msg_index = req.messages.len() as i32;

    let llm_worker_for_embed = state.llm_worker.clone();
    let db_for_embed_persist = state.shared_state.database_pool.clone();
    let session_id_for_embed = session_id.clone();
    let user_msg_for_embed = user_msg_content.clone();
    match llm_worker.stream_response(context_messages, max_tokens, temperature).await {
        Ok(llm_stream) => {

            let output_stream = async_stream::stream! {
                let mut full_response = String::new();
                futures_util::pin_mut!(llm_stream);
                while let Some(item) = llm_stream.next().await {
                    match item {
                        Ok(sse_line) => {

                            if sse_line.starts_with("data: ") && !sse_line.contains("[DONE]") {
                                if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(&sse_line[6..].trim()) {
                                    if let Some(content) = chunk
                                        .get("choices")
                                        .and_then(|c| c.get(0))
                                        .and_then(|c| c.get("delta"))
                                        .and_then(|d| d.get("content"))
                                        .and_then(|c| c.as_str())
                                    {
                                        full_response.push_str(content);
                                    }
                                }
                            }

                            let data = sse_line.trim_start_matches("data: ").trim_end().to_string();
                            yield Ok::<_, Infallible>(Event::default().data(data));
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

                if !full_response.is_empty() {
                    match db_for_persist.conversations.store_messages_batch(
                        &session_id_for_persist,
                        &[("assistant".to_string(), full_response.clone(), msg_index, 0, 0.5)],
                    ) {
                        Ok(stored_msgs) => {
                            debug!("Persisted assistant response ({} chars) for session {}",
                                full_response.len(), session_id_for_persist);



                            let llm_for_embed = llm_worker_for_embed.clone();
                            let db_for_embed = db_for_embed_persist.clone();
                            let assistant_content = full_response.clone();
                            let user_content_for_embed = user_msg_for_embed.clone();
                            let stored = stored_msgs;
                            tokio::spawn(async move {

                                let mut texts = Vec::new();
                                let mut message_ids = Vec::new();

                                if let Some(ref user_text) = user_content_for_embed {


                                    if let Ok(msgs) = db_for_embed.search_messages_by_keywords(
                                        &session_id_for_embed,
                                        &[user_text.clone()],
                                        1,
                                    ).await {
                                        if let Some(user_stored) = msgs.first() {
                                            texts.push(user_text.clone());
                                            message_ids.push(user_stored.id);
                                        }
                                    }
                                }

                                if let Some(assistant_stored) = stored.first() {
                                    texts.push(assistant_content);
                                    message_ids.push(assistant_stored.id);
                                }
                                if texts.is_empty() {
                                    return;
                                }

                                match llm_for_embed.generate_embeddings(texts).await {
                                    Ok(embeddings) => {
                                        let now = chrono::Utc::now();
                                        for (embedding_vec, msg_id) in embeddings.into_iter().zip(message_ids.iter()) {
                                            let emb = Embedding {
                                                id: 0,
                                                message_id: *msg_id,
                                                embedding: embedding_vec,
                                                embedding_model: "llama-server".to_string(),
                                                generated_at: now,
                                            };
                                            if let Err(e) = db_for_embed.embeddings.store_embedding(&emb) {
                                                debug!("Failed to store embedding for msg {}: {}", msg_id, e);
                                            }
                                        }

                                        for msg_id in &message_ids {
                                            let _ = db_for_embed.conversations.mark_embedding_generated(*msg_id);
                                        }
                                        debug!("Stored {} embeddings for session {}", message_ids.len(), session_id_for_embed);
                                    }
                                    Err(e) => {
                                        debug!("Embedding generation skipped (llama-server may not support /v1/embeddings): {}", e);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to persist assistant message: {}", e);
                        }
                    }
                }
            };
            Sse::new(output_stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(std::time::Duration::from_secs(15))
                )
                .into_response()
        }
        Err(e) => {
            error!("Failed to start LLM stream: {}", e);
            (StatusCode::BAD_GATEWAY, format!("LLM backend error: {}", e)).into_response()
        }
    }
}


