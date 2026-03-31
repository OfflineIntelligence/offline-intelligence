
use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct SessionFileContext {
    pub id: i64,
    pub session_id: String,
    pub file_name: String,
    
    pub source: String,
    
    pub file_path: Option<String>,
    
    pub all_files_id: Option<i64>,
    pub size_bytes: Option<i64>,
}

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

    pub fn delete_for_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            "DELETE FROM session_file_contexts WHERE session_id = ?1",
            [session_id],
        )?;
        Ok(n)
    }
}
