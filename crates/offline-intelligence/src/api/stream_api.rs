
use axum::{
    extract::State,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::convert::Infallible;
use tracing::{info, error, debug, warn};
use serde_json::Value;
use reqwest;
use std::sync::Arc;

use crate::memory::Message;
use crate::memory_db::schema::Embedding;
use crate::shared_state::UnifiedAppState;
use crate::utils::{extract_content_from_bytes, estimate_tokens, truncate_to_budget, is_extraction_sentinel};
use crate::cache_management::cache_scorer::score_message_importance;
use regex::Regex;

lazy_static::lazy_static! {
    
    static ref ATTACHED_RE: Regex = Regex::new(r"\[Attached: ([^\]]+)\]").unwrap();
    
    static ref AT_FILE_RE: Regex = Regex::new(r"@(\S+\.\w+)").unwrap();
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatAttachment {
    pub name: String,
    
    pub source: String,
    
    #[serde(default)]
    pub file_path: Option<String>,
    
    #[serde(default)]
    pub all_files_id: Option<i64>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChatRequest {
    pub model: Option<String>,
    pub model_source: Option<String>, 
    pub messages: Vec<Message>,
    pub session_id: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_stream")]
    pub stream: bool,
    
    #[serde(default)]
    pub attachments: Option<Vec<ChatAttachment>>,
    
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_max_tokens() -> u32 { 2000 }
fn default_temperature() -> f32 { 0.7 }
fn default_stream() -> bool { true }

const MAX_ATTACHMENT_TOKENS: usize = 64_000;

async fn try_extract_attachment(
    attach: &ChatAttachment,
    state: &UnifiedAppState,
) -> Result<(String, String), String> {
    match attach.source.as_str() {
        "inline" => {
            let path = attach.file_path.as_deref().ok_or_else(|| {
                format!("'{}': no file path provided. Use the paperclip button to attach files.", attach.name)
            })?;

            info!("Reading inline file: {} ({})", attach.name, path);
            let bytes = tokio::fs::read(path).await.map_err(|e| {
                format!(
                    "Could not read '{}': {}.\n\nMake sure the file exists and is not stored only in the cloud (OneDrive, iCloud, etc.).",
                    attach.name, e
                )
            })?;

            info!("Read {} bytes from '{}'", bytes.len(), attach.name);
            let content = extract_content_from_bytes(&bytes, &attach.name)
                .await
                .map_err(|e| format!("Could not parse '{}': {}", attach.name, e))?;

            if is_extraction_sentinel(&content) {
                return Err(if attach.name.to_lowercase().ends_with(".pdf") {
                    format!(
                        "Could not extract text from '{}'.\n\nThe PDF is likely image-based (scanned) or password-protected. \
                        Try one of:\n  • Export/re-save as a text-based PDF\n  • Attach a DOCX version\n  • Paste the text directly into the chat",
                        attach.name
                    )
                } else {
                    format!(
                        "Could not extract text from '{}'.\n\nThe file may be corrupted or in an unsupported format. \
                        Try a different format, or paste the content directly into the chat.",
                        attach.name
                    )
                });
            }

            if content.trim().is_empty() {
                return Err(format!("'{}' appears to be empty — no text content found.", attach.name));
            }

            info!("Extracted {} chars from '{}'", content.len(), attach.name);
            Ok((attach.name.clone(), content))
        }

        "local_storage" => {
            let id = attach.all_files_id.ok_or_else(|| {
                format!("'{}': no database ID provided for local storage attachment.", attach.name)
            })?;

            info!("Reading local_storage file: {} (id={})", attach.name, id);
            let all_files = &state.shared_state.database_pool.all_files;

            let bytes = all_files.get_file_bytes(id).map_err(|e| {
                format!(
                    "Could not read '{}' from local storage: {}.\n\nTry re-adding the file through the local storage panel.",
                    attach.name, e
                )
            })?;

            info!("Read {} bytes from local_storage '{}'", bytes.len(), attach.name);

            let content = extract_content_from_bytes(&bytes, &attach.name)
                .await
                .map_err(|e| format!("Could not parse '{}': {}", attach.name, e))?;

            if is_extraction_sentinel(&content) {
                return Err(if attach.name.to_lowercase().ends_with(".pdf") {
                    format!(
                        "Could not extract text from '{}'.\n\nThe PDF is likely image-based (scanned) or password-protected. \
                        Try one of:\n  • Export/re-save as a text-based PDF\n  • Attach a DOCX version\n  • Paste the text directly into the chat",
                        attach.name
                    )
                } else {
                    format!(
                        "Could not extract text from '{}'.\n\nThe file may be corrupted or in an unsupported format. \
                        Try a different format, or paste the content directly into the chat.",
                        attach.name
                    )
                });
            }

            let _ = all_files.record_access(id);

            if content.trim().is_empty() {
                return Err(format!("'{}' from local storage appears to be empty.", attach.name));
            }

            info!("Extracted {} chars from local_storage '{}'", content.len(), attach.name);
            Ok((attach.name.clone(), content))
        }

        other => Err(format!(
            "'{}': unknown attachment source '{}'. Use the paperclip button (inline) or the local storage panel to attach files.",
            attach.name, other
        )),
    }
}

fn inject_attachment_contents(messages: &mut Vec<Message>, contents: Vec<(String, String)>) {
    let total_tokens: usize = contents.iter().map(|(_, c)| estimate_tokens(c)).sum();
    info!("Attachment total: {} tokens across {} file(s)", total_tokens, contents.len());

    let final_contents: Vec<(String, String)> = if total_tokens > MAX_ATTACHMENT_TOKENS {
        let budget_per_file = MAX_ATTACHMENT_TOKENS / contents.len().max(1);
        info!("Applying 64k budget: {} tokens/file", budget_per_file);
        contents.into_iter().map(|(name, content)| {
            let (truncated, was_cut) = truncate_to_budget(&content, budget_per_file);
            let final_content = if was_cut {
                let original_tokens = estimate_tokens(&content);
                format!(
                    "{}\n[File truncated: showing first ~{} tokens of ~{} total]",
                    truncated, budget_per_file, original_tokens
                )
            } else {
                truncated
            };
            (name, final_content)
        }).collect()
    } else {
        contents
    };

    let mut block = String::new();
    for (name, content) in &final_contents {
        block.push_str(&format!(
            "\n--- Content of attached file: {} ---\n{}\n--- End of file ---\n",
            name, content
        ));
    }

    if let Some(last_user) = messages.iter_mut().rev().find(|m| m.role == "user") {
        info!("Injecting {} chars of attachment content into user message", block.len());
        last_user.content = format!("{}\n{}", last_user.content, block);
    } else {
        error!("No user message found to inject attachment content into!");
    }
}

async fn process_file_attachments(
    messages: &mut Vec<Message>,
    state: &UnifiedAppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let local_files = &state.shared_state.database_pool.local_files;

    for msg in messages.iter_mut() {
        if msg.role == "user" {
            
            let original = msg.content.clone();
            let mut updated_content = original.clone();

            for cap in ATTACHED_RE.captures_iter(&original) {
                if let Some(m) = cap.get(1) {
                    let filename = m.as_str();
                    updated_content = replace_file_reference(
                        &updated_content,
                        &format!("[Attached: {}]", filename),
                        filename,
                        local_files,
                    ).await;
                }
            }

            for cap in AT_FILE_RE.captures_iter(&original) {
                if let Some(m) = cap.get(1) {
                    let filename = m.as_str();
                    updated_content = replace_file_reference(
                        &updated_content,
                        &format!("@{}", filename),
                        filename,
                        local_files,
                    ).await;
                }
            }

            msg.content = updated_content;
        }
    }

    Ok(())
}

async fn replace_file_reference(
    content: &str,
    marker: &str,
    filename: &str,
    local_files: &crate::memory_db::LocalFilesStore,
) -> String {
    
    match local_files.get_file_by_name(filename) {
        Ok(file) => {
            
            match local_files.get_file_content(file.id) {
                Ok(bytes) => {
                    match extract_content_from_bytes(&bytes, filename).await {
                        Ok(file_content) if !file_content.trim().is_empty() => {
                            let attachment_text = format!(
                                "\n--- Content of file: {} ---\n{}\n--- End of file ---\n",
                                filename, file_content
                            );
                            content.replace(marker, &attachment_text)
                        }
                        _ => {
                            let error_text = format!(
                                "\n[Note: Could not extract text from '{}'. The file may be in an unsupported format.]",
                                filename
                            );
                            content.replace(marker, &error_text)
                        }
                    }
                }
                Err(_) => {
                    let error_text = format!(
                        "\n[Note: Could not read file '{}'. File may be missing or corrupted.]",
                        filename
                    );
                    content.replace(marker, &error_text)
                }
            }
        }
        Err(_) => {
            
            let app_data_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("Aud.io");
            let file_path = app_data_dir.join(filename);

            match crate::utils::extract_file_content(&file_path).await {
                Ok(file_content) => {
                    let attachment_text = format!(
                        "\n--- Content of file: {} ---\n{}\n--- End of file ---\n",
                        filename, file_content
                    );
                    content.replace(marker, &attachment_text)
                }
                Err(_) => {
                    let error_text = format!(
                        "\n[Note: File '{}' not found in local files. Upload it first or check the filename.]",
                        filename
                    );
                    content.replace(marker, &error_text)
                }
            }
        }
    }
}

pub async fn generate_stream(
    State(state): State<UnifiedAppState>,
    Json(req): Json<StreamChatRequest>,
) -> Response {
    let request_num = state.shared_state.counters.inc_total_requests();
    info!("Stream request #{} for session: {}", request_num, req.session_id);
    
    if let Some(ref attachments) = req.attachments {
        info!("Request has {} attachment(s)", attachments.len());
        for (i, att) in attachments.iter().enumerate() {
            info!(
                "  Attachment {}: name={}, source={}, file_path={}, all_files_id={}",
                i, att.name,
                att.source,
                att.file_path.as_deref().unwrap_or("(none)"),
                att.all_files_id.map(|id| id.to_string()).unwrap_or_else(|| "(none)".to_string()),
            );
        }
    } else {
        debug!("Request has no attachments");
    }

    if req.messages.is_empty() {
        return (StatusCode::BAD_REQUEST, "Messages array cannot be empty").into_response();
    }

    let session_id = req.session_id.clone();

    let mut processed_messages = req.messages.clone();

    if let Err(e) = process_file_attachments(&mut processed_messages, &state).await {
        error!("Error processing legacy file text references: {}", e);
        
    }

    if let Some(ref attachments) = req.attachments {
        if !attachments.is_empty() {
            let mut extracted: Vec<(String, String)> = Vec::with_capacity(attachments.len());
            let mut errors: Vec<String> = Vec::new();

            for attach in attachments {
                
                let cache_key = crate::api::attachment_api::attachment_cache_key(attach);
                if let Some((_, cached)) = state.shared_state.attachment_cache.remove(&cache_key) {
                    if !cached.is_stale(crate::api::attachment_api::CACHE_TTL_SECS) {
                        
                        if is_extraction_sentinel(&cached.text) {
                            info!("Cached sentinel for '{}' — treating as miss, re-extracting", attach.name);
                        } else {
                            info!("Attachment cache hit for '{}' — skipping extraction", attach.name);
                            extracted.push((attach.name.clone(), cached.text));
                            continue;
                        }
                    } else {
                        info!("Stale cache entry for '{}' — re-extracting", attach.name);
                    }
                }

                match try_extract_attachment(attach, &state).await {
                    Ok(content) => extracted.push(content),
                    Err(e) => {
                        warn!("Attachment extraction failed for '{}': {}", attach.name, e);
                        errors.push(e);
                    }
                }
            }

            if !errors.is_empty() {
                let error_msg = errors.join("\n\n");
                error!("Rejecting request — {} attachment(s) could not be processed", errors.len());
                return (StatusCode::UNPROCESSABLE_ENTITY, error_msg).into_response();
            }

            inject_attachment_contents(&mut processed_messages, extracted);
        }
    }

    {
        let db = &state.shared_state.database_pool;

        if let Some(ref attachments) = req.attachments {
            if !attachments.is_empty() {
                let refs: Vec<crate::memory_db::AttachmentRef<'_>> = attachments
                    .iter()
                    .map(|a| crate::memory_db::AttachmentRef {
                        name: &a.name,
                        source: &a.source,
                        file_path: a.file_path.as_deref(),
                        all_files_id: a.all_files_id,
                        size_bytes: a.size_bytes,
                    })
                    .collect();
                if let Err(e) = db.session_file_contexts.store_attachments(&session_id, &refs) {
                    warn!("Failed to persist session file context references: {}", e);
                }
            }
        }

        match db.session_file_contexts.get_for_session(&session_id) {
            Ok(historical) if !historical.is_empty() => {
                
                let current_names: std::collections::HashSet<&str> = req
                    .attachments
                    .as_ref()
                    .map(|a| a.iter().map(|att| att.name.as_str()).collect())
                    .unwrap_or_default();

                let prior: Vec<_> = historical
                    .iter()
                    .filter(|h| !current_names.contains(h.file_name.as_str()))
                    .collect();

                if !prior.is_empty() {
                    info!(
                        "Re-injecting {} historical file(s) as persistent context for session {}",
                        prior.len(),
                        session_id
                    );

                    const HIST_BUDGET: usize = 32_000;
                    let budget_per_file = HIST_BUDGET / prior.len().max(1);

                    let mut context_block = String::from(
                        "Files previously shared in this conversation (always available as context):\n",
                    );

                    for hist in &prior {
                        let chat_att = ChatAttachment {
                            name: hist.file_name.clone(),
                            source: hist.source.clone(),
                            file_path: hist.file_path.clone(),
                            all_files_id: hist.all_files_id,
                            size_bytes: hist.size_bytes,
                        };
                        match try_extract_attachment(&chat_att, &state).await {
                            Ok((name, content)) => {
                                let (truncated, was_cut) =
                                    truncate_to_budget(&content, budget_per_file);
                                context_block.push_str(&format!(
                                    "\n--- {} ---\n{}{}\n--- end of {} ---\n",
                                    name,
                                    truncated,
                                    if was_cut { "\n[... file truncated for context ...]" } else { "" },
                                    name
                                ));
                            }
                            Err(e) => {
                                
                                warn!(
                                    "Could not re-read historical attachment '{}': {}",
                                    hist.file_name, e
                                );
                            }
                        }
                    }

                    if context_block.len() > 80 {
                        
                        if let Some(first) = processed_messages.first_mut() {
                            if first.role == "system" {
                                first.content.push_str(&format!("\n\n{}", context_block));
                            } else {
                                processed_messages.insert(
                                    0,
                                    crate::memory::Message {
                                        role: "system".to_string(),
                                        content: context_block.clone(),
                                    },
                                );
                            }
                        } else {
                            processed_messages.insert(
                                0,
                                crate::memory::Message {
                                    role: "system".to_string(),
                                    content: context_block.clone(),
                                },
                            );
                        }
                        info!(
                            "Injected {} chars of persistent file context for session {}",
                            context_block.len(),
                            session_id
                        );
                    }
                }
            }
            Ok(_) => {} 
            Err(e) => {
                warn!("Could not retrieve session file contexts: {}", e);
            }
        }
    }

    let session = state.shared_state.get_or_create_session(&session_id).await;

    {
        if let Ok(mut session_data) = session.write() {
            session_data.last_accessed = std::time::Instant::now();
            session_data.messages = processed_messages.clone();
        }
    }

    let user_msg_content = req.messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone());
    if let Some(ref content) = user_msg_content {
        let db = state.shared_state.database_pool.clone();
        let sid = session_id.clone();
        let content = content.clone();
        let msg_count = processed_messages.len() as i32;
        
        if let Err(e) = db.conversations.create_session_with_id(&sid, None) {
            
            debug!("Session creation result (may already exist): {}", e);
        }
        
        tokio::spawn(async move {
            if let Err(e) = db.conversations.store_messages_batch(
                &sid,
                &[("user".to_string(), content.clone(), msg_count - 1, 0, score_message_importance("user", &content))],
            ) {
                error!("Failed to persist user message: {}", e);
            }
        });
    }

    let context_messages = {
        let orchestrator_guard = state.context_orchestrator.read().await;
        if let Some(ref orchestrator) = *orchestrator_guard {
            let user_query = user_msg_content.as_deref();
            match orchestrator.process_conversation(&session_id, &processed_messages, user_query).await {
                Ok(optimized) => {
                    if optimized.len() != processed_messages.len() {
                        info!("Context engine optimized: {} → {} messages (retrieved past context)",
                            processed_messages.len(), optimized.len());
                    }
                    optimized
                }
                Err(e) => {
                    error!("Context engine error (falling back to raw messages): {}", e);
                    processed_messages.clone()
                }
            }
        } else {
            debug!("Context orchestrator not initialized, using raw messages");
            processed_messages.clone()
        }
    };

    let max_tokens = req.max_tokens;
    let temperature = req.temperature;
    let db_for_persist = state.shared_state.database_pool.clone();
    let session_id_for_persist = session_id.clone();
    let msg_index = req.messages.len() as i32;

    let db_for_embed_persist = state.shared_state.database_pool.clone();
    let session_id_for_embed = session_id.clone();
    let user_msg_for_embed = user_msg_content.clone();

    let is_online_model = req.model_source.as_deref() == Some("openrouter");
    
    if is_online_model {
        
        let api_key = req.api_key.clone().unwrap_or_else(|| {
            std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
                
                state.shared_state.config.openrouter_api_key.clone()
            })
        });
        
        if api_key.is_empty() {
            return (StatusCode::UNAUTHORIZED, "OpenRouter API key not configured").into_response();
        }
        
        let model_id = req.model.unwrap_or_else(|| "openrouter/auto".to_string());
        let openrouter_messages = context_messages.iter().map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content
            })
        }).collect::<Vec<_>>();
        
        let openrouter_request = serde_json::json!({
            "model": model_id,
            "messages": openrouter_messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": true,
        });
        
        match stream_openrouter_response(api_key, openrouter_request, session_id_for_persist.clone(), db_for_persist.clone(), context_messages.clone(), user_msg_for_embed.clone(), db_for_embed_persist.clone(), session_id_for_embed.clone(), state.http_client.clone()).await {
            Ok(openrouter_stream) => {
                
                let output_stream = async_stream::stream! {
                    let mut full_response = String::new();
                    
                    futures_util::pin_mut!(openrouter_stream);
                    
                    while let Some(item) = tokio_stream::StreamExt::next(&mut openrouter_stream).await {
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
                                error!("OpenRouter stream error: {}", e);
                                yield Ok(Event::default().data(
                                    format!("{{\"error\": \"{}\"}}", e)
                                ));
                                break;
                            }
                        }
                    }
                    
                    if !full_response.is_empty() {
                        let importance = score_message_importance("assistant", &full_response);
                        match db_for_persist.conversations.store_messages_batch(
                            &session_id_for_persist,
                            &[("assistant".to_string(), full_response.clone(), msg_index, 0, importance)],
                        ) {
                            Ok(stored_msgs) => {
                                debug!("Persisted assistant response ({} chars) for session {}",
                                    full_response.len(), session_id_for_persist);
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
                error!("Failed to start OpenRouter stream: {}", e);
                let json_body = build_openrouter_error_json(&e.to_string());
                (StatusCode::BAD_GATEWAY, axum::Json(json_body)).into_response()
            }
        }
    } else {
        
        let runtime_ready = state.llm_worker.is_runtime_ready().await;
        info!("Offline mode: runtime_ready check = {}", runtime_ready);
        
        if !runtime_ready {
            info!("Model not ready - returning error");
            return (StatusCode::SERVICE_UNAVAILABLE, 
                "Model Not Ready: No local model is currently loaded. Please go to the Models page and activate a model by clicking \"Active Model\".").into_response();
        }
        
        let llm_worker = state.llm_worker.clone();
        let llm_worker_for_embed = state.llm_worker.clone();
        
        match llm_worker.stream_response(context_messages, max_tokens, temperature).await {
            Ok(llm_stream) => {
                
                let output_stream = async_stream::stream! {
                    let mut full_response = String::new();

                    futures_util::pin_mut!(llm_stream);

                    while let Some(item) = tokio_stream::StreamExt::next(&mut llm_stream).await {
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
                        let importance = score_message_importance("assistant", &full_response);
                        match db_for_persist.conversations.store_messages_batch(
                            &session_id_for_persist,
                            &[("assistant".to_string(), full_response.clone(), msg_index, 0, importance)],
                        ) {
                            Ok(_stored_msgs) => {
                                debug!("Persisted assistant response ({} chars) for session {}",
                                    full_response.len(), session_id_for_persist);

                                let llm_for_embed = llm_worker_for_embed.clone();
                                let db_for_embed = db_for_embed_persist.clone();
                                let assistant_content = full_response.clone();
                                let user_content_for_embed = user_msg_for_embed.clone();
                                let stored = _stored_msgs;

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

                return Sse::new(output_stream)
                    .keep_alive(
                        axum::response::sse::KeepAlive::new()
                            .interval(std::time::Duration::from_secs(15))
                    )
                    .into_response();
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                error!("Failed to start LLM stream: {}", error_msg);
                
                let (status_code, user_message) = if error_msg.contains("Cannot connect") || error_msg.contains("Connection refused") {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Local LLM server is not running. Please ensure:\\n\\n1. An engine is installed (Settings > Engines)\\n2. A model is downloaded and loaded (Settings > Models)\\n3. The engine has finished initializing".to_string()
                    )
                } else if error_msg.contains("not found") || error_msg.contains("No such file") {
                    (
                        StatusCode::NOT_FOUND,
                        "Model or engine binary not found. Please:\\n\\n1. Download an engine (Settings > Engines)\\n2. Download a model (Settings > Models)\\n3. Wait for initialization to complete".to_string()
                    )
                } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    (
                        StatusCode::GATEWAY_TIMEOUT,
                        "LLM server connection timed out. The engine may be still initializing. Please wait a moment and try again.".to_string()
                    )
                } else {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("LLM backend error: {}\\n\\nPlease check that:\\n1. Engine is installed\\n2. Model is loaded\\n3. Engine is running", error_msg)
                    )
                };
                
                return (status_code, user_message).into_response();
            }
        }
    }
}

async fn stream_openrouter_response(
    api_key: String,
    request_body: Value,
    session_id: String,
    _db_for_persist: Arc<crate::memory_db::MemoryDatabase>,
    _context_messages: Vec<crate::memory::Message>,
    _user_msg_for_embed: Option<String>,
    _db_for_embed_persist: Arc<crate::memory_db::MemoryDatabase>,
    _session_id_for_embed: String,
    client: reqwest::Client,
) -> Result<
    std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, anyhow::Error>> + Send>>,
    anyhow::Error
> {
    
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://aud.io")
        .header("X-Title", "Aud.io")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenRouter request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("OpenRouter returned {}: {}", status, body));
    }

    let byte_stream = response.bytes_stream();

    let sse_stream = async_stream::try_stream! {
        let mut buffer = String::new();

        futures_util::pin_mut!(byte_stream);

        while let Some(chunk_result) = tokio_stream::StreamExt::next(&mut byte_stream).await {
            let chunk: bytes::Bytes = chunk_result
                .map_err(|e| anyhow::anyhow!("Stream read error: {}", e))?;

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                
                buffer.drain(..=newline_pos);

                if line.is_empty() {
                    continue;
                }

                if line.starts_with("data: ") {
                    let data = &line[6..];

                    if data == "[DONE]" {
                        yield "data: [DONE]\n\n".to_string();
                        return;
                    }

                    match serde_json::from_str::<Value>(data) {
                        Ok(chunk) => {
                            
                            let finished = chunk
                                .get("choices")
                                .and_then(|c| c.as_array())
                                .map(|arr| arr.iter().any(|choice| {
                                    choice.get("finish_reason")
                                        .and_then(|fr| fr.as_str())
                                        .map(|fr| !fr.is_empty())
                                        .unwrap_or(false)
                                }))
                                .unwrap_or(false);

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

    Ok(Box::pin(sse_stream))
}

fn build_openrouter_error_json(err_str: &str) -> serde_json::Value {
    
    if let Some(brace_pos) = err_str.find('{') {
        let raw_body = &err_str[brace_pos..];
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw_body) {
            let msg = v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("OpenRouter returned an error");
            let code = v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_u64())
                .unwrap_or(0) as u16;
            let (error_type, user_message) = classify_openrouter_error(code, msg);
            return serde_json::json!({
                "error_type": error_type,
                "message": user_message,
            });
        }
    }
    
    serde_json::json!({
        "error_type": "generic",
        "message": "OpenRouter returned an error. Please try again or switch to a different model.",
    })
}

fn classify_openrouter_error(code: u16, msg: &str) -> (&'static str, String) {
    let m = msg.to_lowercase();
    if code == 402
        || m.contains("credit")
        || m.contains("insufficient")
        || m.contains("balance")
        || m.contains("billing")
        || m.contains("quota")
    {
        (
            "insufficient_credits",
            "Your OpenRouter account has insufficient credits to process this request.".to_string(),
        )
    } else if (code == 400 || code == 413)
        && (m.contains("context")
            || m.contains("too long")
            || m.contains("token")
            || m.contains("length"))
    {
        (
            "context_exceeded",
            "This conversation exceeds the model's context limit. Try a shorter message or switch to a model with a larger context window.".to_string(),
        )
    } else if code == 429
        || m.contains("rate limit")
        || m.contains("rate_limit")
        || m.contains("too many request")
    {
        (
            "rate_limit",
            "Rate limit exceeded for this model. Please wait a moment and try again, or switch to a different model.".to_string(),
        )
    } else if code == 401
        || (m.contains("invalid") && (m.contains("key") || m.contains("api")))
        || m.contains("unauthorized")
        || m.contains("authentication")
    {
        (
            "invalid_key",
            "Your OpenRouter API key is invalid or expired. Please update it in the Models page.".to_string(),
        )
    } else if m.contains("not enabled")
        || m.contains("developer instruction")
        || m.contains("not supported")
        || (m.contains("invalid request") && (m.contains("model") || m.contains("instruction")))
    {
        (
            "model_restriction",
            "This model has a restriction that prevents it from being used with this request. Try switching to a different model.".to_string(),
        )
    } else {
        ("generic", format!("OpenRouter error: {}", msg))
    }
}
