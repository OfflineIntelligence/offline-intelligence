//! Pre-extraction endpoint for file attachments.
//!
//! `POST /attachments/preprocess` fires background extraction tasks the moment
//! a user attaches a file — so by the time they hit Send the content is already
//! cached and injection is zero-overhead.
//!
//! Hardware-adaptive concurrency:
//! - Plain-text formats (code, JSON, CSV, Markdown, …) bypass the semaphore —
//!   they are just an `fs::read` + UTF-8 decode, essentially free.
//! - Binary formats (PDF, DOCX, XLSX, PPTX, …) acquire a semaphore permit first.
//!   The semaphore is sized to `num_cpus / 2` (min 1, max 8) so the LLM server
//!   is never starved of CPU/RAM on low-spec hardware.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::api::stream_api::ChatAttachment;
use crate::shared_state::{PreExtracted, UnifiedAppState};
use crate::utils::{extract_content_from_bytes, is_extraction_sentinel};

/// TTL for cached pre-extracted entries (30 minutes).
/// The background eviction task checks every 5 minutes.
pub const CACHE_TTL_SECS: u64 = 1_800;

/// Build a stable, unique cache key for an attachment.
pub fn attachment_cache_key(attach: &ChatAttachment) -> String {
    match attach.source.as_str() {
        "inline" => format!(
            "inline:{}",
            attach.file_path.as_deref().unwrap_or(&attach.name)
        ),
        "local_storage" => format!("local_storage:{}", attach.all_files_id.unwrap_or(0)),
        other => format!("{}:{}", other, attach.name),
    }
}

/// Returns `true` for formats that are pure text — just an `fs::read` + UTF-8
/// decode, no binary parser, no meaningful CPU/RAM pressure.
fn is_plain_text_format(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "txt" | "md" | "markdown"
            | "json" | "jsonl" | "csv" | "tsv"
            | "yaml" | "yml" | "toml" | "xml" | "html" | "htm"
            | "css" | "scss" | "sass" | "less"
            | "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "mjs" | "cjs"
            | "java" | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp"
            | "cs" | "go" | "php" | "rb" | "swift" | "kt" | "scala"
            | "sql" | "sh" | "bash" | "zsh" | "fish" | "bat" | "ps1" | "psm1"
            | "env" | "log" | "ini" | "cfg" | "conf" | "properties"
            | "dockerfile" | "makefile" | "gitignore" | "gitattributes"
            | "editorconfig" | "eslintrc" | "prettierrc"
    )
}

#[derive(Debug, Deserialize)]
pub struct PreprocessRequest {
    pub attachments: Vec<ChatAttachment>,
}

#[derive(Debug, Serialize)]
pub struct PreprocessResponse {
    /// Files whose extraction was queued in the background.
    pub queued: usize,
    /// Files already in a fresh (non-stale) cache entry — no work needed.
    pub already_cached: usize,
}

/// `POST /attachments/preprocess`
///
/// Accepts a list of file attachments and immediately spawns background
/// extraction tasks — one per file. Returns before any extraction completes.
///
/// The next `/generate/stream` call for those same files will find the results
/// in `shared_state.attachment_cache` and skip re-extraction entirely.
pub async fn preprocess_attachments(
    State(state): State<UnifiedAppState>,
    Json(req): Json<PreprocessRequest>,
) -> (StatusCode, Json<PreprocessResponse>) {
    if req.attachments.is_empty() {
        return (
            StatusCode::OK,
            Json(PreprocessResponse {
                queued: 0,
                already_cached: 0,
            }),
        );
    }

    let mut queued = 0usize;
    let mut already_cached = 0usize;

    for attach in req.attachments {
        let key = attachment_cache_key(&attach);

        // Skip if already cached and still fresh.
        if let Some(entry) = state.shared_state.attachment_cache.get(&key) {
            if !entry.is_stale(CACHE_TTL_SECS) {
                already_cached += 1;
                continue;
            }
            // Stale entry — fall through and re-queue.
        }

        queued += 1;
        let state_clone = state.clone();
        let attach_clone = attach.clone();
        let key_clone = key.clone();
        let is_plain = is_plain_text_format(&attach.name);

        tokio::spawn(async move {
            // Binary files acquire a semaphore permit before starting so that at
            // most N heavy extractions run concurrently (N = num_cpus / 2, max 8).
            // Plain-text files bypass the semaphore — they are I/O + decode only.
            let _permit = if is_plain {
                None
            } else {
                match state_clone
                    .shared_state
                    .extraction_semaphore
                    .acquire()
                    .await
                {
                    Ok(p) => Some(p),
                    Err(_) => {
                        warn!(
                            "Extraction semaphore closed while pre-processing '{}'",
                            attach_clone.name
                        );
                        return;
                    }
                }
            };

            match extract_for_cache(&attach_clone, &state_clone).await {
                Ok(text) => {
                    info!(
                        "Pre-extracted '{}' ({} chars) → stored in cache",
                        attach_clone.name,
                        text.len()
                    );
                    state_clone.shared_state.attachment_cache.insert(
                        key_clone,
                        PreExtracted {
                            text,
                            extracted_at: std::time::Instant::now(),
                        },
                    );
                }
                Err(e) => {
                    // Silently discard — the error surfaces again at Send time
                    // via `try_extract_attachment`, which has proper user messages.
                    warn!("Pre-extraction failed for '{}': {}", attach_clone.name, e);
                }
            }
            // `_permit` is dropped here, releasing the semaphore slot.
        });
    }

    info!(
        "Attachment preprocess: {} queued, {} already cached",
        queued, already_cached
    );

    (
        StatusCode::OK,
        Json(PreprocessResponse {
            queued,
            already_cached,
        }),
    )
}

/// Internal extraction helper for the preprocess path.
/// Mirrors `try_extract_attachment` but returns `anyhow::Result<String>`
/// (just the text) instead of the user-facing `Result<(String, String), String>`.
async fn extract_for_cache(
    attach: &ChatAttachment,
    state: &UnifiedAppState,
) -> anyhow::Result<String> {
    match attach.source.as_str() {
        "inline" => {
            let path = attach
                .file_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("no file_path on inline attachment"))?;
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|e| anyhow::anyhow!("fs::read '{}': {}", path, e))?;
            let text = extract_content_from_bytes(&bytes, &attach.name)
                .await
                .map_err(|e| anyhow::anyhow!("extract '{}': {}", attach.name, e))?;
            // Do not cache sentinel strings — let try_extract_attachment surface the
            // proper user-facing error at Send time via HTTP 422.
            if is_extraction_sentinel(&text) {
                return Err(anyhow::anyhow!("extraction failed for '{}' — result is sentinel", attach.name));
            }
            Ok(text)
        }

        "local_storage" => {
            let id = attach
                .all_files_id
                .ok_or_else(|| anyhow::anyhow!("no all_files_id on local_storage attachment"))?;
            let bytes = state
                .shared_state
                .database_pool
                .all_files
                .get_file_bytes(id)
                .map_err(|e| anyhow::anyhow!("db get_file_bytes id={}: {}", id, e))?;
            let text = extract_content_from_bytes(&bytes, &attach.name)
                .await
                .map_err(|e| anyhow::anyhow!("extract '{}': {}", attach.name, e))?;
            // Do not cache sentinel strings.
            if is_extraction_sentinel(&text) {
                return Err(anyhow::anyhow!("extraction failed for '{}' — result is sentinel", attach.name));
            }
            Ok(text)
        }

        other => Err(anyhow::anyhow!("unknown attachment source '{}'", other)),
    }
}
