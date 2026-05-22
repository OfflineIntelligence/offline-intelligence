//! Session file contexts store.
//!
//! Persists references to all files ever attached during a conversation session.
//! This enables "sticky" file context: once a user attaches a file, the LLM
//! sees its content for the entire conversation — matching ChatGPT, Gemini, etc.

use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;
use tracing::{debug, info};

/// A single file attachment reference stored for a session.
#[derive(Debug, Clone)]
pub struct SessionFileContext {
    pub id: i64,
    pub session_id: String,
    pub file_name: String,
    /// "inline" or "local_storage"
    pub source: String,
    /// OS file path — set when source == "inline"
    pub file_path: Option<String>,
    /// all_files table ID — set when source == "local_storage"
    pub all_files_id: Option<i64>,
    pub size_bytes: Option<i64>,
}

/// Lightweight attachment reference passed to the store. Avoids a circular
/// dependency between `memory_db` and `api::stream_api`.
pub struct AttachmentRef<'a> {
    pub name: &'a str,
    pub source: &'a str,
    pub file_path: Option<&'a str>,
    pub all_files_id: Option<i64>,
    pub size_bytes: Option<i64>,
}

pub struct SessionFileContextsStore {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl SessionFileContextsStore {
    pub fn new(pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    /// Store a set of attachment references for a session.
    /// Deduplicates by (session_id, file_name, source) — avoids duplicate rows
    /// when the same file is attached multiple times in the same session.
    pub fn store_attachments(
        &self,
        session_id: &str,
        attachments: &[AttachmentRef<'_>],
    ) -> Result<()> {
        let conn = self.pool.get()?;
        for att in attachments {
            conn.execute(
                "INSERT OR IGNORE INTO session_file_contexts
                 (session_id, file_name, source, file_path, all_files_id, size_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    session_id,
                    att.name,
                    att.source,
                    att.file_path,
                    att.all_files_id,
                    att.size_bytes,
                ],
            )?;
        }
        debug!(
            "Stored {} attachment reference(s) for session {}",
            attachments.len(),
            session_id
        );
        Ok(())
    }

    /// Return all file references stored for a session, ordered by attach time.
    pub fn get_for_session(&self, session_id: &str) -> Result<Vec<SessionFileContext>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, file_name, source, file_path, all_files_id, size_bytes
             FROM session_file_contexts
             WHERE session_id = ?1
             ORDER BY attached_at ASC",
        )?;

        let rows = stmt.query_map([session_id], |row| {
            Ok(SessionFileContext {
                id: row.get(0)?,
                session_id: row.get(1)?,
                file_name: row.get(2)?,
                source: row.get(3)?,
                file_path: row.get(4)?,
                all_files_id: row.get(5)?,
                size_bytes: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        info!(
            "Retrieved {} historical attachment(s) for session {}",
            results.len(),
            session_id
        );
        Ok(results)
    }

    /// Remove all file contexts for a session (e.g., when the conversation is deleted).
    pub fn delete_for_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            "DELETE FROM session_file_contexts WHERE session_id = ?1",
            [session_id],
        )?;
        Ok(n)
    }
}
