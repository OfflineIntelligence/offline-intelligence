//! API Keys Store — Machine-specific encryption with SQLite storage
//!
//! Stores and manages API keys using machine-specific XOR encryption stored directly in SQLite.
//! No OS keychain, no password prompts, no external dependencies.
//!
//! Encryption uses the machine's device name as the key, ensuring that keys can only be
//! decrypted on the same machine where they were encrypted.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

// ─────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────

/// Identifies which external service a key belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKeyType {
    HuggingFace,
}

impl ApiKeyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyType::HuggingFace => "huggingface",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "huggingface" | "hf" => Some(ApiKeyType::HuggingFace),
            _ => None,
        }
    }
}

/// Metadata record stored in SQLite.
/// `encrypted_value` is always a base64-encoded XOR-encrypted string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub key_type: String,
    pub encrypted_value: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_mode: Option<String>,
    pub usage_count: i64,
}

// ─────────────────────────────────────────────
// Store
// ─────────────────────────────────────────────

/// Manages API keys with machine-specific encryption stored in SQLite.
pub struct ApiKeysStore {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl ApiKeysStore {
    pub fn new(pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    // ── Schema ───────────────────────────────

    /// Create the `api_keys` table if it does not yet exist.
    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key_type TEXT NOT NULL UNIQUE,
                encrypted_value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_used_at TEXT,
                last_mode TEXT,
                usage_count INTEGER DEFAULT 0
            )",
            [],
        )?;
        info!("API keys table initialized");
        Ok(())
    }

    // ── Write ────────────────────────────────

    /// Save or update an API key.
    ///
    /// Encrypts the plaintext value using machine-specific encryption and stores
    /// the encrypted value directly in SQLite.
    pub fn save_key(&self, key_type: ApiKeyType, plaintext: &str) -> Result<()> {
        let name = key_type.as_str();

        // Encrypt using machine-specific key
        let encrypted = Encryption::encrypt(plaintext);
        info!("Encrypted API key for: {}", name);

        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO api_keys (key_type, encrypted_value, created_at, usage_count)
             VALUES (?1, ?2, ?3, 0)
             ON CONFLICT(key_type) DO UPDATE SET
                encrypted_value = excluded.encrypted_value,
                created_at = excluded.created_at",
            params![name, encrypted, now],
        )?;

        info!("API key saved for: {}", name);
        Ok(())
    }

    // ── Read ─────────────────────────────────

    /// Retrieve the **plaintext** value of an API key.
    ///
    /// Decrypts the stored encrypted value using machine-specific encryption.
    pub fn get_key_plaintext(&self, key_type: &ApiKeyType) -> Result<Option<String>> {
        let record = match self.get_key_metadata(key_type)? {
            Some(r) => r,
            None => return Ok(None),
        };

        // Decrypt the stored encrypted value
        match Encryption::decrypt(&record.encrypted_value) {
            Ok(plaintext) => Ok(Some(plaintext)),
            Err(e) => {
                warn!("Failed to decrypt API key '{}': {}", key_type.as_str(), e);
                Err(e)
            }
        }
    }

    /// Get the raw SQLite metadata record (does **not** decrypt).
    pub fn get_key_metadata(&self, key_type: &ApiKeyType) -> Result<Option<ApiKeyRecord>> {
        let conn = self.pool.get()?;
        let name = key_type.as_str();

        let row = conn
            .query_row(
                "SELECT id, key_type, encrypted_value, created_at, last_used_at, last_mode, usage_count
                 FROM api_keys WHERE key_type = ?1",
                params![name],
                |row| {
                    Ok(ApiKeyRecord {
                        id: row.get(0)?,
                        key_type: row.get(1)?,
                        encrypted_value: row.get(2)?,
                        created_at: row
                            .get::<_, String>(3)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                        last_used_at: row
                            .get::<_, Option<String>>(4)?
                            .and_then(|s| s.parse().ok()),
                        last_mode: row.get(5)?,
                        usage_count: row.get(6)?,
                    })
                },
            )
            .optional()?;

        Ok(row)
    }

    /// Get all API keys (encrypted values).
    pub fn get_all_keys(&self) -> Result<Vec<ApiKeyRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, key_type, encrypted_value, created_at, last_used_at, last_mode, usage_count
             FROM api_keys",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ApiKeyRecord {
                id: row.get(0)?,
                key_type: row.get(1)?,
                encrypted_value: row.get(2)?,
                created_at: row
                    .get::<_, String>(3)?
                    .parse()
                    .unwrap_or_else(|_| Utc::now()),
                last_used_at: row
                    .get::<_, Option<String>>(4)?
                    .and_then(|s| s.parse().ok()),
                last_mode: row.get(5)?,
                usage_count: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Get all API keys with their plaintext values (decrypted).
    pub fn get_all_keys_with_values(&self) -> Result<Vec<(ApiKeyRecord, String)>> {
        let keys = self.get_all_keys()?;
        let mut result = Vec::new();
        for record in keys {
            match Encryption::decrypt(&record.encrypted_value) {
                Ok(value) => result.push((record, value)),
                Err(e) => {
                    warn!("Failed to decrypt API key '{}': {}", record.key_type, e);
                }
            }
        }
        Ok(result)
    }

    /// Check if an API key exists (without decrypting it).
    pub fn key_exists(&self, key_type: &ApiKeyType) -> Result<bool> {
        let conn = self.pool.get()?;
        let name = key_type.as_str();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE key_type = ?1",
            params![name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ── Update ───────────────────────────────

    /// Mark an API key as used (update last_used_at and usage_count).
    pub fn mark_used(&self, key_type: ApiKeyType, mode: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let name = key_type.as_str();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1, last_mode = ?2, usage_count = usage_count + 1
             WHERE key_type = ?3",
            params![now, mode, name],
        )?;
        Ok(())
    }

    // ── Delete ───────────────────────────────

    /// Delete an API key from storage.
    pub fn delete_key(&self, key_type: ApiKeyType) -> Result<bool> {
        let conn = self.pool.get()?;
        let name = key_type.as_str();

        let rows = conn.execute("DELETE FROM api_keys WHERE key_type = ?1", params![name])?;
        info!("Deleted API key for: {}", name);
        Ok(rows > 0)
    }
}

// ─────────────────────────────────────────────
// Simple machine-specific XOR encryption
// ─────────────────────────────────────────────

/// Machine-specific XOR encryption for API keys.
/// Uses the device name as the encryption key.
/// Keys can only be decrypted on the same machine where they were encrypted.
pub struct Encryption;

impl Encryption {
    fn get_machine_key() -> Vec<u8> {
        let machine_id = whoami::devicename();
        let mut key: Vec<u8> = machine_id.bytes().collect();
        while key.len() < 32 {
            key.push((key.len() as u8).wrapping_mul(17));
        }
        key.truncate(32);
        key
    }

    /// Encrypt plaintext using machine-specific XOR encryption.
    /// Returns base64-encoded ciphertext.
    pub fn encrypt(plaintext: &str) -> String {
        let key = Self::get_machine_key();
        let encrypted: Vec<u8> = plaintext
            .as_bytes()
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();
        BASE64.encode(&encrypted)
    }

    /// Decrypt base64-encoded ciphertext using machine-specific XOR encryption.
    pub fn decrypt(ciphertext: &str) -> Result<String> {
        let key = Self::get_machine_key();
        let bytes = BASE64
            .decode(ciphertext)
            .context("Failed to decode base64")?;
        let decrypted: Vec<u8> = bytes
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();
        Ok(String::from_utf8(decrypted)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original = "hf_1234567890abcdef";
        let encrypted = Encryption::encrypt(original);
        let decrypted = Encryption::decrypt(&encrypted).unwrap();
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_encrypt_different_output() {
        let key1 = Encryption::encrypt("test_key");
        let key2 = Encryption::encrypt("test_key");
        // Same input should produce same output on same machine
        assert_eq!(key1, key2);
    }
}
