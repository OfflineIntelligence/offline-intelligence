//! Admin API endpoints
//! 
//! This module provides administrative functionality for system management.
//! Currently a placeholder for future implementation.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::{Arc, atomic::Ordering};
use serde::{Deserialize, Serialize};

use crate::shared_state::SharedState;

/// System health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Database statistics response
#[derive(Debug, Serialize)]
pub struct DbStatsResponse {
    pub total_sessions: usize,
    pub total_messages: usize,
    pub total_summaries: usize,
    pub database_size_bytes: u64,
}

/// Maintenance request
#[derive(Debug, Deserialize)]
pub struct MaintenanceRequest {
    pub operation: String,
    pub parameters: Option<serde_json::Value>,
}


// Store application start time as seconds since epoch
static START_TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn init_start_time() {
    let now: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();
    START_TIME.store(now, Ordering::SeqCst);
}

fn get_uptime_seconds() -> u64 {
    let start = START_TIME.load(Ordering::SeqCst);
    if start == 0 {
        // Initialize on first call
        init_start_time();
        return 0;
    }
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();
    
    now.saturating_sub(start)
}

/// Health check endpoint
pub async fn health(
    State(_shared_state): State<Arc<SharedState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // Initialize start time if not already set
    if START_TIME.load(Ordering::SeqCst) == 0 {
        init_start_time();
    }
    
    Ok((
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: get_uptime_seconds(),
        }),
    ))
}

/// Database statistics endpoint
pub async fn db_stats(
    State(shared_state): State<Arc<SharedState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // Access the database to get actual statistics
    let total_sessions = shared_state.conversations.counters.active_sessions.load(Ordering::Relaxed);
    
    let total_messages = shared_state.conversations.counters.processed_messages.load(Ordering::Relaxed);
    
    // Get cache statistics
    let cache_hits = shared_state.conversations.counters.cache_hits.load(Ordering::Relaxed);
    let cache_misses = shared_state.conversations.counters.cache_misses.load(Ordering::Relaxed);
    let total_summaries = cache_hits + cache_misses; // Approximation
    
    // Get database file size from the database path
    let database_size_bytes = match std::fs::metadata("data/conversations.db") {
        Ok(metadata) => metadata.len(),
        Err(_) => match std::fs::metadata("conversations.db") {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        }
    };
    
    Ok((
        StatusCode::OK,
        Json(DbStatsResponse {
            total_sessions,
            total_messages,
            total_summaries,
            database_size_bytes,
        }),
    ))
}

/// Maintenance endpoint - performs system maintenance operations
pub async fn maintenance(
    State(shared_state): State<Arc<SharedState>>,
    Json(payload): Json<MaintenanceRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    match payload.operation.as_str() {
        "cleanup_expired_sessions" => {
            // Remove sessions from the in-memory DashMap that haven't been accessed
            // in the last 30 minutes. Database records are kept intact.
            const EXPIRY_SECS: u64 = 30 * 60;
            let initial_count = shared_state.conversations.sessions.len();

            shared_state.conversations.sessions.retain(|_, session| {
                session.read()
                    .map(|data| data.last_accessed.elapsed().as_secs() <= EXPIRY_SECS)
                    .unwrap_or(true) // keep if lock poisoned
            });

            let removed = initial_count.saturating_sub(shared_state.conversations.sessions.len());
            shared_state.counters.active_sessions.fetch_sub(
                removed.min(shared_state.counters.active_sessions.load(std::sync::atomic::Ordering::Relaxed)),
                std::sync::atomic::Ordering::Relaxed,
            );

            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Expired sessions cleanup completed",
                    "operation": "cleanup_expired_sessions",
                    "initial_session_count": initial_count,
                    "removed_count": removed,
                    "status": "completed"
                })),
            ))
        },
        "optimize_database" => {
            // Run PRAGMA optimize + WAL checkpoint to reclaim disk space and
            // update query planner statistics.
            match shared_state.database_pool.optimize() {
                Ok(()) => Ok((
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "message": "Database optimization completed (PRAGMA optimize + WAL checkpoint)",
                        "operation": "optimize_database",
                        "status": "completed"
                    })),
                )),
                Err(e) => Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "message": format!("Database optimization failed: {}", e),
                        "operation": "optimize_database",
                        "status": "error"
                    })),
                )),
            }
        },
        "clear_inactive_sessions" => {
            // More aggressive version: remove sessions idle for more than 5 minutes.
            const INACTIVE_SECS: u64 = 5 * 60;
            let initial_count = shared_state.conversations.sessions.len();

            shared_state.conversations.sessions.retain(|_, session| {
                session.read()
                    .map(|data| data.last_accessed.elapsed().as_secs() <= INACTIVE_SECS)
                    .unwrap_or(true)
            });

            let removed = initial_count.saturating_sub(shared_state.conversations.sessions.len());
            shared_state.counters.active_sessions.fetch_sub(
                removed.min(shared_state.counters.active_sessions.load(std::sync::atomic::Ordering::Relaxed)),
                std::sync::atomic::Ordering::Relaxed,
            );

            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Inactive sessions cleared",
                    "operation": "clear_inactive_sessions",
                    "initial_session_count": initial_count,
                    "removed_count": removed,
                    "status": "completed"
                })),
            ))
        },
        _ => {
            Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "message": format!("Unknown maintenance operation: {}", payload.operation),
                    "supported_operations": ["cleanup_expired_sessions", "optimize_database", "clear_inactive_sessions"],
                    "status": "error"
                })),
            ))
        }
    }
}
