use rusqlite::{Connection, Result, OptionalExtension};
use tracing::{info, warn, error};
use std::path::Path;
use crate::memory_db::schema;
/
pub struct MigrationManager<'a> {
    conn: &'a mut Connection,
}
impl<'a> MigrationManager<'a> {
    /
    pub fn new(conn: &'a mut Connection) -> Self {
        Self { conn }
    }

    /
    pub fn initialize_database(&mut self) -> Result<()> {
        info!("Initializing memory database schema...");


        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;


        let current_version: i32 = self.conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        info!("Current database schema version: {}", current_version);


        self.apply_migrations(current_version)?;

        Ok(())
    }

    /
    fn apply_migrations(&mut self, current_version: i32) -> Result<()> {
        let migrations = get_migrations();

        for (version, migration_sql) in migrations.iter() {
            if *version > current_version {
                info!("Applying migration {}...", version);


                let tx = self.conn.transaction()?;


                if let Err(e) = tx.execute_batch(migration_sql) {
                    error!("Failed to apply migration {}: {}", version, e);
                    return Err(e);
                }


                tx.execute(
                    "INSERT INTO schema_version (version) VALUES (?)",
                    [version],
                )?;


                tx.commit()?;

                info!("Migration {} applied successfully", version);
            }
        }

        Ok(())
    }

    /
    pub fn create_connection(db_path: &Path) -> Result<Connection> {

        let mut conn = Connection::open(db_path)?;


        conn.execute_batch("
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -2000; -- 2MB cache
        ")?;


        let mut migrator = MigrationManager::new(&mut conn);
        migrator.initialize_database()?;

        Ok(conn)
    }

    /
    pub fn cleanup_old_data(&mut self, older_than_days: i32) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
        let cutoff_str = cutoff.to_rfc3339();


        let deleted = self.conn.execute(
            "DELETE FROM sessions WHERE last_accessed < ?1",
            [&cutoff_str],
        )?;

        info!("Cleaned up {} old sessions", deleted);


        if deleted > 0 {
            self.conn.execute_batch("VACUUM")?;
            info!("Database vacuum completed");
        }

        Ok(deleted)
    }

    /
    pub fn get_current_version(&self) -> Result<i32> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .or_else(|_| Ok(0))
    }

    /
    pub fn has_migration_applied(&self, version: i32) -> Result<bool> {
        self.conn
            .query_row(
                "SELECT 1 FROM schema_version WHERE version = ?",
                [version],
                |_| Ok(1),
            )
            .optional()
            .map(|result| result.is_some())
    }
}
/
fn get_migrations() -> Vec<(i32, &'static str)> {
    vec![
        (1, include_str!("migrations/001_initial.sql")),
        (2, include_str!("migrations/002_add_embeddings.sql")),
        (3, include_str!("migrations/003_add_kv_snapshots.sql")),
    ]
}
/
/
pub fn get_database_stats(conn: &Connection) -> Result<schema::DatabaseStats> {

    fn get_table_count(conn: &Connection, table_name: &str) -> Result<i64> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {}", table_name), [], |row| row.get(0))
            .or_else(|e| {
                warn!("Failed to get count from table {}: {}", table_name, e);
                Ok(0)
            })
    }

    let total_sessions = get_table_count(conn, "sessions")?;
    let total_messages = get_table_count(conn, "messages")?;
    let total_summaries = get_table_count(conn, "summaries")?;
    let total_details = get_table_count(conn, "details")?;
    let total_embeddings = get_table_count(conn, "embeddings")?;


    let database_size_bytes: i64 = conn
        .query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(schema::DatabaseStats {
        total_sessions,
        total_messages,
        total_summaries,
        total_details,
        total_embeddings,
        database_size_bytes,
    })
}
/
/
pub fn get_database_stats_from_path(db_path: &Path) -> Result<schema::DatabaseStats> {
    let conn = Connection::open(db_path)?;
    get_database_stats(&conn)
}
/
pub fn run_maintenance(conn: &mut Connection) -> Result<()> {
    info!("Running database maintenance...");


    conn.execute_batch("ANALYZE")?;


    conn.execute_batch("PRAGMA incremental_vacuum(100)")?;


    conn.execute_batch("PRAGMA integrity_check")?;

    info!("Database maintenance completed");
    Ok(())
}

