
use std::sync::Arc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tracing::debug;

use crate::memory_db::schema::SessionSummary;

pub struct SessionSummariesStore {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl SessionSummariesStore {
    pub fn new(pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    pub fn upsert(
        &self,
        session_id: &str,
        summary_text: &str,
        token_count: i32,
        total_message_count: i32,
    ) -> anyhow::Result<()> {
        let conn = self.pool.get()?;

        let existing_clear_count: i32 = conn
            .query_row(
                "SELECT clear_count FROM session_summaries WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "INSERT OR REPLACE INTO session_summaries
             (session_id, summary_text, token_count, total_message_count, clear_count, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)",
            rusqlite::params![
                session_id,
                summary_text,
                token_count,
                total_message_count,
                existing_clear_count + 1,
            ],
        )?;

        debug!(
            "Upserted cumulative summary for session {} (clear #{}, {} tokens)",
            session_id,
            existing_clear_count + 1,
            token_count
        );
        Ok(())
    }

    pub fn get(&self, session_id: &str) -> anyhow::Result<Option<SessionSummary>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT session_id, summary_text, token_count, total_message_count,
                    clear_count, last_updated
             FROM session_summaries
             WHERE session_id = ?1",
        )?;

        let mut rows = stmt.query([session_id])?;
        if let Some(row) = rows.next()? {
            let last_updated_str: String = row.get(5)?;
            let last_updated = chrono::DateTime::parse_from_rfc3339(&last_updated_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse timestamp: {}", e))?
                .with_timezone(&chrono::Utc);

            return Ok(Some(SessionSummary {
                session_id: row.get(0)?,
                summary_text: row.get(1)?,
                token_count: row.get(2)?,
                total_message_count: row.get(3)?,
                clear_count: row.get(4)?,
                last_updated,
            }));
        }

        Ok(None)
    }

    pub fn delete_for_session(&self, session_id: &str) -> anyhow::Result<usize> {
        let conn = self.pool.get()?;
        let deleted = conn.execute(
            "DELETE FROM session_summaries WHERE session_id = ?1",
            [session_id],
        )?;
        Ok(deleted)
    }
}
