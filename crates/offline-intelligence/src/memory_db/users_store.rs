
use anyhow::Result;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub email_verified: bool,
    pub verification_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub google_id: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct UsersStore {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl Clone for UsersStore {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
        }
    }
}

impl UsersStore {
    pub fn new(pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                email_verified INTEGER DEFAULT 0,
                verification_token TEXT,
                created_at TEXT NOT NULL,
                verified_at TEXT
            )",
            [],
        )?;

        let _ = conn.execute("ALTER TABLE users ADD COLUMN google_id TEXT", []);
        let _ = conn.execute("ALTER TABLE users ADD COLUMN avatar_url TEXT", []);

        info!("Users table initialized");
        Ok(())
    }

    pub fn create_user(
        &self,
        email: &str,
        name: &str,
        password_hash: &str,
    ) -> Result<i64> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (email, name, password_hash, email_verified, verification_token, created_at)
             VALUES (?1, ?2, ?3, 1, '', ?4)",
            params![email, name, password_hash, now],
        )?;

        let id = conn.last_insert_rowid();
        info!("User created with id: {}", id);
        Ok(id)
    }

    pub fn upsert_google_user(
        &self,
        email: &str,
        name: &str,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> Result<(User, bool)> {
        let conn = self.pool.get()?;

        let mut is_new_user = false;

        let by_google_id = conn
            .query_row(
                "SELECT id FROM users WHERE google_id = ?1",
                params![google_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if let Some(user_id) = by_google_id {
            
            conn.execute(
                "UPDATE users SET name = ?1, avatar_url = ?2 WHERE id = ?3",
                params![name, avatar_url, user_id],
            )?;
        } else {
            
            let by_email = conn
                .query_row(
                    "SELECT id FROM users WHERE email = ?1",
                    params![email],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;

            if let Some(user_id) = by_email {
                
                conn.execute(
                    "UPDATE users SET google_id = ?1, avatar_url = ?2, email_verified = 1 WHERE id = ?3",
                    params![google_id, avatar_url, user_id],
                )?;
            } else {
                
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO users (email, name, password_hash, email_verified, google_id, avatar_url, created_at)
                     VALUES (?1, ?2, 'google-oauth-user', 1, ?3, ?4, ?5)",
                    params![email, name, google_id, avatar_url, now],
                )?;
                is_new_user = true;
            }
        }

        let user = conn.query_row(
            "SELECT id, email, name, password_hash, email_verified, verification_token,
                    created_at, verified_at, google_id, avatar_url
             FROM users WHERE email = ?1",
            params![email],
            |row| {
                let created_str: String = row.get(6)?;
                let verified_str: Option<String> = row.get(7)?;
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    name: row.get(2)?,
                    password_hash: row.get(3)?,
                    email_verified: row.get::<_, i32>(4)? != 0,
                    verification_token: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    verified_at: verified_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    google_id: row.get(8)?,
                    avatar_url: row.get(9)?,
                })
            },
        )?;

        info!("Google user upserted: {} (new={})", email, is_new_user);
        Ok((user, is_new_user))
    }

    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let conn = self.pool.get()?;

        let result = conn
            .query_row(
                "SELECT id, email, name, password_hash, email_verified, verification_token,
                        created_at, verified_at, google_id, avatar_url
                 FROM users WHERE email = ?1",
                params![email],
                |row| {
                    let created_str: String = row.get(6)?;
                    let verified_str: Option<String> = row.get(7)?;

                    Ok(User {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        name: row.get(2)?,
                        password_hash: row.get(3)?,
                        email_verified: row.get::<_, i32>(4)? != 0,
                        verification_token: row.get(5)?,
                        created_at: DateTime::parse_from_rfc3339(&created_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        verified_at: verified_str.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                        google_id: row.get(8)?,
                        avatar_url: row.get(9)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
        let conn = self.pool.get()?;

        let result = conn
            .query_row(
                "SELECT id, email, name, password_hash, email_verified, verification_token,
                        created_at, verified_at, google_id, avatar_url
                 FROM users WHERE id = ?1",
                params![id],
                |row| {
                    let created_str: String = row.get(6)?;
                    let verified_str: Option<String> = row.get(7)?;

                    Ok(User {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        name: row.get(2)?,
                        password_hash: row.get(3)?,
                        email_verified: row.get::<_, i32>(4)? != 0,
                        verification_token: row.get(5)?,
                        created_at: DateTime::parse_from_rfc3339(&created_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        verified_at: verified_str.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                        google_id: row.get(8)?,
                        avatar_url: row.get(9)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn verify_email(&self, token: &str) -> Result<Option<User>> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();

        let rows_affected = conn.execute(
            "UPDATE users SET email_verified = 1, verified_at = ?1, verification_token = NULL
             WHERE verification_token = ?2 AND email_verified = 0",
            params![now, token],
        )?;

        if rows_affected > 0 {
            let user = conn
                .query_row(
                    "SELECT id, email, name, password_hash, email_verified, verification_token,
                            created_at, verified_at, google_id, avatar_url
                     FROM users WHERE verification_token IS NULL AND email_verified = 1
                     ORDER BY id DESC LIMIT 1",
                    [],
                    |row| {
                        let created_str: String = row.get(6)?;
                        let verified_str: Option<String> = row.get(7)?;

                        Ok(User {
                            id: row.get(0)?,
                            email: row.get(1)?,
                            name: row.get(2)?,
                            password_hash: row.get(3)?,
                            email_verified: true,
                            verification_token: None,
                            created_at: DateTime::parse_from_rfc3339(&created_str)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now()),
                            verified_at: verified_str.and_then(|s| {
                                DateTime::parse_from_rfc3339(&s)
                                    .ok()
                                    .map(|dt| dt.with_timezone(&Utc))
                            }),
                            google_id: row.get(8)?,
                            avatar_url: row.get(9)?,
                        })
                    },
                )
                .optional()?;

            Ok(user)
        } else {
            Ok(None)
        }
    }

    pub fn email_exists(&self, email: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE email = ?1",
            params![email],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}
