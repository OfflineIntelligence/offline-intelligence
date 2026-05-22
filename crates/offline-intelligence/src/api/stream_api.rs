//! Streaming chat endpoint — the core 1-hop architecture handler.
//!
//! Flow: Client POST → SharedState (session + cache lookup) → LLM Worker (HTTP to llama-server) → SSE stream back
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
use serde::Deserialize;
use std::convert::Infallible;
use tracing::{info, error, debug, warn};
use std::sync::Arc;

use crate::memory::Message;
use crate::memory_db::schema::Embedding;
use crate::shared_state::UnifiedAppState;
use crate::utils::{extract_content_from_bytes, estimate_tokens, truncate_to_budget, is_extraction_sentinel};
use crate::cache_management::cache_scorer::score_message_importance;
use regex::Regex;

lazy_static::lazy_static! {
    /// Matches legacy `[Attached: filename]` markers in user message text.
    static ref ATTACHED_RE: Regex = Regex::new(r"\[Attached: ([^\]]+)\]").unwrap();
    /// Matches legacy `@filename.ext` references in user message text.
    /// NOTE: must NOT be run on messages that have already had file content injected.
    static ref AT_FILE_RE: Regex = Regex::new(r"@(\S+\.\w+)").unwrap();
}

/// File attachment reference sent with the chat request.
///
/// Two sources are supported:
/// - `inline` (paperclip): a real OS file path from the Tauri file dialog.
///   Backend reads the file from disk and extracts text server-side.
/// - `local_storage` (@filename / folder icon): an `all_files_id` pointing to
///   a file saved in the user's persistent Local Storage. Backend reads from
///   the `all_files` table directly.
///
/// A 64k-token budget is applied across ALL attachments in a single request.
/// No file content bytes travel over HTTP — only file references.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatAttachment {
    pub name: String,
    /// "inline" (paperclip) or "local_storage" (@filename / folder icon)
    pub source: String,
    /// OS file path — required when source == "inline"
    #[serde(default)]
    pub file_path: Option<String>,
    /// Database ID in `all_files` table — required when source == "local_storage"
    #[serde(default)]
    pub all_files_id: Option<i64>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
}

/// Request body matching what the frontend sends
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
    /// Inline file attachments (temporary, session-scoped)
    #[serde(default)]
    pub attachments: Option<Vec<ChatAttachment>>,
}

fn default_max_tokens() -> u32 { 2000 }
fn default_temperature() -> f32 { 0.7 }
fn default_stream() -> bool { true }

/// Maximum combined token budget for all file attachments in a single request.
const MAX_ATTACHMENT_TOKENS: usize = 64_000;


/// Extract text from a single attachment. Returns an error string suitable for
/// display to the user if anything goes wrong — no silent fallbacks.
///
/// Sources:
///   - `source == "inline"` + `file_path`        → read from OS disk, extract text
///   - `source == "local_storage"` + `all_files_id` → read from all_files DB table
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

            // file_processor returns sentinel strings (not Err) on extraction failure.
            // Catch all of them so no format silently passes empty/error text to the LLM.
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

            // Read raw bytes so binary formats (DOCX, XLSX, PDF, PPTX) can be
            // properly parsed — not just lossy-decoded as text.
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

            // Catch sentinel error strings returned by file_processor on extraction failure.
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

/// Inject extracted file contents into the last user message with a 64k token budget.
/// Oversized files are truncated with a notice.
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

/// Process file attachments in messages by replacing [Attached: filename] or [@filename] markers with actual file content
/// This handles references to persistent local files stored in the database
async fn process_file_attachments(
    messages: &mut Vec<Message>,
    state: &UnifiedAppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let local_files = &state.shared_state.database_pool.local_files;

    for msg in messages.iter_mut() {
        if msg.role == "user" {
            // Snapshot the original content for regex matching — we'll accumulate
            // replacements into `updated_content` without re-matching on modified text.
            let original = msg.content.clone();
            let mut updated_content = original.clone();

            // Replace [Attached: filename] markers (legacy)
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

            // Replace @filename.ext references (legacy)
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

/// Helper to replace a file reference with actual content.
///
/// Uses `get_file_content` (raw bytes) + `extract_content_from_bytes` so binary formats
/// (DOCX, XLSX, PDF, PPTX) are correctly parsed rather than returning garbage text.
async fn replace_file_reference(
    content: &str,
    marker: &str,
    filename: &str,
    local_files: &crate::memory_db::LocalFilesStore,
) -> String {
    // Try to find the file by name in the local_files database.
    match local_files.get_file_by_name(filename) {
        Ok(file) => {
            // Read raw bytes — binary-safe for all formats (DOCX, XLSX, PDF, etc.)
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
            // Filesystem fallback for files not yet in the database (backward compatibility).
            let file_path = crate::utils::PathResolver::data_dir().join(filename);

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

/// POST /generate/stream — Main streaming chat endpoint
///
/// 1. Validates request and gets/creates session in shared memory
/// 2. Persists user message to database
/// 3. Streams LLM response back via SSE
/// 4. Persists assistant response to database after completion
pub async fn generate_stream(
    State(state): State<UnifiedAppState>,
    Json(req): Json<StreamChatRequest>,
) -> Response {
    let request_num = state.shared_state.counters.inc_total_requests();
    info!("Stream request #{} for session: {}", request_num, req.session_id);
    
    // Log attachment info
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

    // 1. Process file attachments in the messages
    let mut processed_messages = req.messages.clone();

    // 1a. FIRST: Replace legacy @filename / [Attached: filename] text references with real content.
    //     This MUST run before inject_attachment_contents so that regex patterns inside injected
    //     file blocks (e.g. `@pytest.fixture` in a Python file, `@media` in CSS, email addresses)
    //     are never incorrectly matched and corrupted.
    if let Err(e) = process_file_attachments(&mut processed_messages, &state).await {
        error!("Error processing legacy file text references: {}", e);
        // Continue — text-ref processing failure is non-fatal; structured attachments are handled below.
    }

    // 1b. THEN: Extract and inject structured attachments (paperclip inline + local_storage folder icon).
    //     Runs after legacy text-ref processing to prevent cross-contamination.
    //     Fail fast — return HTTP 422 if ANY attachment cannot be read or parsed.
    //     A 64k-token budget is shared across all attachments before injection.
    if let Some(ref attachments) = req.attachments {
        if !attachments.is_empty() {
            let mut extracted: Vec<(String, String)> = Vec::with_capacity(attachments.len());
            let mut errors: Vec<String> = Vec::new();

            for attach in attachments {
                // Fast path: check the pre-extraction cache populated by
                // `POST /attachments/preprocess` (fires when user attaches a file).
                // `remove()` evicts the entry after use — the content is only
                // needed once (for this request's context window injection).
                let cache_key = crate::api::attachment_api::attachment_cache_key(attach);
                if let Some((_, cached)) = state.shared_state.attachment_cache.remove(&cache_key) {
                    if !cached.is_stale(crate::api::attachment_api::CACHE_TTL_SECS) {
                        // Guard: preprocess may have cached a sentinel if extraction failed.
                        // Treat the cached sentinel as a cache miss so try_extract_attachment
                        // runs and surfaces a proper user-facing error via HTTP 422.
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

                // Slow path: extract now (user sent before pre-extraction finished,
                // or pre-extraction failed silently).
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

    // 1c. Persistent file context: store current attachments in DB, then re-inject
    //     ALL historical attachments from previous messages in this conversation.
    //     This gives ChatGPT/Gemini-style "the file stays in context forever" behaviour.
    {
        let db = &state.shared_state.database_pool;

        // Store current attachment references (deduped via UNIQUE constraint in DB)
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

        // Re-read all historical files that were attached in PRIOR messages of this session
        match db.session_file_contexts.get_for_session(&session_id) {
            Ok(historical) if !historical.is_empty() => {
                // Exclude files that are part of the CURRENT request (already injected above)
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

                    // Budget: 32k tokens total shared across all historical files
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
                                // Non-fatal: file may have been deleted/moved since it was attached.
                                warn!(
                                    "Could not re-read historical attachment '{}': {}",
                                    hist.file_name, e
                                );
                            }
                        }
                    }

                    if context_block.len() > 80 {
                        // Prepend as a system message (or extend existing system message)
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
            Ok(_) => {} // No historical files — nothing to inject
            Err(e) => {
                warn!("Could not retrieve session file contexts: {}", e);
            }
        }
    }

    // 2. Get or create session in shared memory (zero-cost Arc lookup)
    let session = state.shared_state.get_or_create_session(&session_id).await;

    // 3. Update in-memory session with the processed messages
    {
        if let Ok(mut session_data) = session.write() {
            session_data.last_accessed = std::time::Instant::now();
            session_data.messages = processed_messages.clone();
        }
    }

    // 4. Ensure session exists in database and persist user message
    //    CRITICAL: This must complete BEFORE title updates can happen, so we do it synchronously
    //    to avoid race conditions where title update happens before session creation completes
    //
    //    Use the ORIGINAL request messages (pre-injection) for DB storage so conversation history
    //    shows the clean user text + optional [Attached files: ...] annotation only — NOT the huge
    //    injected file content that was added to processed_messages for the LLM context window.
    let user_msg_content = req.messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone());
    if let Some(ref content) = user_msg_content {
        let db = state.shared_state.database_pool.clone();
        let sid = session_id.clone();
        let content = content.clone();
        let msg_count = processed_messages.len() as i32;
        
        // Create session synchronously to ensure it exists before streaming starts
        // This prevents race condition with title updates
        if let Err(e) = db.conversations.create_session_with_id(&sid, None) {
            // Ignore "already exists" errors - session may have been created by a previous request
            debug!("Session creation result (may already exist): {}", e);
        }
        
        // Persist user message in background (this can be async)
        tokio::spawn(async move {
            if let Err(e) = db.conversations.store_messages_batch(
                &sid,
                &[("user".to_string(), content.clone(), msg_count - 1, 0, score_message_importance("user", &content))],
            ) {
                error!("Failed to persist user message: {}", e);
            }
        });
    }

    // 4. Context Engine: Retrieve past context via semantic search when KV cache misses.
    //    Always let the retrieval planner decide — even a brand-new session can trigger
    //    cross-session search if the user asks "what did we discuss yesterday?".
    //    The planner + orchestrator handle the "nothing to search" case internally
    //    (checks has_embeddings > 0 before hitting llama-server, returns early if no past refs).
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

    // 5. Determine routing based on model source
    let max_tokens = req.max_tokens;
    let temperature = req.temperature;
    let db_for_persist = state.shared_state.database_pool.clone();
    let session_id_for_persist = session_id.clone();
    let msg_index = req.messages.len() as i32;

    // Clones for background embedding generation after stream completes
    let db_for_embed_persist = state.shared_state.database_pool.clone();
    let session_id_for_embed = session_id.clone();
    let user_msg_for_embed = user_msg_content.clone();

    {
        // Local model inference — the only inference mode.
        // First check if the runtime is ready before attempting to stream
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
                // Wrap the LLM stream to collect the full response for DB persistence
                let output_stream = async_stream::stream! {
                    let mut full_response = String::new();

                    futures_util::pin_mut!(llm_stream);

                    while let Some(item) = tokio_stream::StreamExt::next(&mut llm_stream).await {
                        match item {
                            Ok(sse_line) => {
                                // Extract content from SSE data for persistence
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

                                // Yield SSE event to client
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

                    // Persist assistant response to database after stream completes
                    if !full_response.is_empty() {
                        let importance = score_message_importance("assistant", &full_response);
                        match db_for_persist.conversations.store_messages_batch(
                            &session_id_for_persist,
                            &[("assistant".to_string(), full_response.clone(), msg_index, 0, importance)],
                        ) {
                            Ok(_stored_msgs) => {
                                debug!("Persisted assistant response ({} chars) for session {}",
                                    full_response.len(), session_id_for_persist);

                                // Background: Generate and store embeddings for the new messages
                                // This captures the vectors llama.cpp computes via /v1/embeddings
                                // enabling semantic search for future KV cache misses.
                                let llm_for_embed = llm_worker_for_embed.clone();
                                let db_for_embed = db_for_embed_persist.clone();
                                let assistant_content = full_response.clone();
                                let user_content_for_embed = user_msg_for_embed.clone();
                                let stored = _stored_msgs;

                                tokio::spawn(async move {
                                    // Collect texts + their message IDs for embedding
                                    let mut texts = Vec::new();
                                    let mut message_ids = Vec::new();

                                    // User message embedding (get ID from DB)
                                    if let Some(ref user_text) = user_content_for_embed {
                                        // The user message was stored one index before the assistant
                                        // We need its DB ID — query by session + content
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

                                    // Assistant message embedding
                                    if let Some(assistant_stored) = stored.first() {
                                        texts.push(assistant_content);
                                        message_ids.push(assistant_stored.id);
                                    }

                                    if texts.is_empty() {
                                        return;
                                    }

                                    // Call llama-server /v1/embeddings
                                    match llm_for_embed.generate_embeddings(texts).await {
                                        Ok(embeddings) => {
                                            let now = chrono::Utc::now();
                                            for (embedding_vec, msg_id) in embeddings.into_iter().zip(message_ids.iter()) {
                                                let emb = Embedding {
                                                    id: 0, // auto-assigned by DB
                                                    message_id: *msg_id,
                                                    embedding: embedding_vec,
                                                    embedding_model: "llama-server".to_string(),
                                                    generated_at: now,
                                                };
                                                if let Err(e) = db_for_embed.embeddings.store_embedding(&emb) {
                                                    debug!("Failed to store embedding for msg {}: {}", msg_id, e);
                                                }
                                            }
                                            // Mark messages as having embeddings
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
                
                // Provide clear, actionable error messages based on error type
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

