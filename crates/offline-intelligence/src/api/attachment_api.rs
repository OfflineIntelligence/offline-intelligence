
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::api::stream_api::ChatAttachment;
use crate::shared_state::{PreExtracted, UnifiedAppState};
use crate::utils::{extract_content_from_bytes, is_extraction_sentinel};

pub const CACHE_TTL_SECS: u64 = 1_800;

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
    
    pub queued: usize,
    
    pub already_cached: usize,
}

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

        if let Some(entry) = state.shared_state.attachment_cache.get(&key) {
            if !entry.is_stale(CACHE_TTL_SECS) {
                already_cached += 1;
                continue;
            }
            
        }

        queued += 1;
        let state_clone = state.clone();
        let attach_clone = attach.clone();
        let key_clone = key.clone();
        let is_plain = is_plain_text_format(&attach.name);

        tokio::spawn(async move {
            
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
                    
                    warn!("Pre-extraction failed for '{}': {}", attach_clone.name, e);
                }
            }
            
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
            
            if is_extraction_sentinel(&text) {
                return Err(anyhow::anyhow!("extraction failed for '{}' — result is sentinel", attach.name));
            }
            Ok(text)
        }

        other => Err(anyhow::anyhow!("unknown attachment source '{}'", other)),
    }
}
