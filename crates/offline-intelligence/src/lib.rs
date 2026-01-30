// _Aud.io/offline-intelligence/crates/src/lib.rs

pub mod admin;
pub mod api;
pub mod backend_target;
pub mod config;
pub mod context_engine;
pub mod memory;
pub mod memory_db;
pub mod metrics;
pub mod resources;
pub mod cache_management;
pub mod telemetry;
pub mod utils;
pub mod shared_state;
pub mod thread_pool;
pub mod worker_threads;
pub mod thread_server;
pub mod model_runtime;

// Public API exports
pub use memory::{Message, MemoryStore, InMemoryMemoryStore};
pub use config::Config;
pub use thread_server::run_thread_server;

// API exports
pub use api::{
    memory_api::{memory_optimize, memory_stats, memory_cleanup, SessionStats, CleanupStats},
    search_api::{search as search_memory, SearchRequest, SearchResponse},
    title_api::{generate_title, GenerateTitleRequest, GenerateTitleResponse},
    conversation_api::{get_conversations, get_conversation, update_conversation_title, delete_conversation, update_conversation_pinned},
    stream_api::generate_stream,
};
