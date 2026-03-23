//! Shared state management for thread-based architecture
//!
//! This module provides the core shared memory infrastructure that enables
//! efficient communication between worker threads while maintaining thread safety.

use std::sync::{Arc, RwLock, atomic::{AtomicUsize, AtomicBool, Ordering}};
use dashmap::DashMap;
use tracing::info;

use crate::{
    config::Config,
    context_engine::ContextOrchestrator,
    memory_db::MemoryDatabase,
    cache_management::KVCacheManager,
    worker_threads::LLMWorker,
    model_management::ModelManager,
    model_runtime::RuntimeManager,
};
use crate::engine_management::EngineManager;

/// Cached result of pre-extracting a file attachment before the user hits Send.
/// Populated by `POST /attachments/preprocess`, consumed by `/generate/stream`.
#[derive(Clone)]
pub struct PreExtracted {
    pub text: String,
    pub extracted_at: std::time::Instant,
}

impl PreExtracted {
    /// Returns true when the entry has exceeded its time-to-live.
    pub fn is_stale(&self, ttl_secs: u64) -> bool {
        self.extracted_at.elapsed().as_secs() >= ttl_secs
    }
}

/// Core shared system state container
pub struct SharedSystemState {
    /// Conversation data with hierarchical locking
    pub conversations: Arc<ConversationHierarchy>,

    /// LLM runtime for direct inference
    pub llm_runtime: Arc<RwLock<Option<LLMRuntime>>>,

    /// Cache management system
    pub cache_manager: Arc<RwLock<Option<Arc<KVCacheManager>>>>,

    /// Database connection pool
    pub database_pool: Arc<MemoryDatabase>,

    /// Configuration (read-only after initialization)
    pub config: Arc<Config>,

    /// Atomic counters for performance tracking
    pub counters: Arc<AtomicCounters>,

    /// Context orchestrator for memory management (tokio RwLock for async access)
    pub context_orchestrator: Arc<tokio::sync::RwLock<Option<ContextOrchestrator>>>,

    /// LLM worker for inference operations
    pub llm_worker: Arc<LLMWorker>,

    /// Model management system
    pub model_manager: Option<Arc<ModelManager>>,

    /// Runtime management system
    pub runtime_manager: Arc<std::sync::RwLock<Option<Arc<RuntimeManager>>>>,

    /// Engine management system
    pub engine_manager: Option<Arc<EngineManager>>,

    /// Engine availability flag - true when an engine binary is ready for use
    pub engine_available: Arc<AtomicBool>,

    /// Initialization completion flag - true when all components are ready
    pub initialization_complete: Arc<AtomicBool>,
    
    /// HTTP server port - may differ from config if original port was in use
    pub http_port: Arc<RwLock<u16>>,

    /// Pre-extracted attachment text cache.
    /// Key: `"inline:{path}"` or `"local_storage:{id}"`.
    /// Populated by `POST /attachments/preprocess` while the user types;
    /// consumed (and evicted) by `/generate/stream` at Send time.
    pub attachment_cache: Arc<DashMap<String, PreExtracted>>,

    /// Limits concurrent binary-file extractions to num_cpus/2 (min 1, max 8)
    /// so the LLM server is never CPU-starved on low-spec hardware.
    pub extraction_semaphore: Arc<tokio::sync::Semaphore>,
}

/// Hierarchical conversation storage for reduced lock contention
pub struct ConversationHierarchy {
    /// Coarse-grained session-level locks
    pub sessions: DashMap<String, Arc<RwLock<SessionData>>>,

    /// Fine-grained message-level queues for hot paths
    pub message_queues: DashMap<String, Arc<crossbeam_queue::ArrayQueue<PendingMessage>>>,

    /// Lock-free counters for performance metrics
    pub counters: Arc<AtomicCounters>,
}

/// Session-level data structure
#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub messages: Vec<crate::memory::Message>,
    pub last_accessed: std::time::Instant,
    pub pinned: bool,
}

/// Pending message for asynchronous processing
#[derive(Debug, Clone)]
pub struct PendingMessage {
    pub message: crate::memory::Message,
    pub timestamp: std::time::Instant,
}

/// Atomic counters for system metrics
pub struct AtomicCounters {
    pub total_requests: AtomicUsize,
    pub active_sessions: AtomicUsize,
    pub processed_messages: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
}

impl AtomicCounters {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicUsize::new(0),
            active_sessions: AtomicUsize::new(0),
            processed_messages: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }

    pub fn inc_total_requests(&self) -> usize {
        self.total_requests.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn inc_processed_messages(&self) -> usize {
        self.processed_messages.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn inc_cache_hit(&self) -> usize {
        self.cache_hits.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn inc_cache_miss(&self) -> usize {
        self.cache_misses.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Direct LLM runtime integration
pub struct LLMRuntime {
    pub model_path: String,
    pub context_size: u32,
    pub batch_size: u32,
    pub threads: u32,
    pub gpu_layers: u32,
    // Note: Actual llama.cpp integration would go here
    // For now, we'll maintain the existing HTTP proxy approach
    // but prepare the structure for direct integration
}

impl SharedSystemState {
    pub fn new(config: Config, database: Arc<MemoryDatabase>) -> anyhow::Result<Self> {
        info!("Initializing shared system state");

        let conversations = Arc::new(ConversationHierarchy {
            sessions: DashMap::new(),
            message_queues: DashMap::new(),
            counters: Arc::new(AtomicCounters::new()),
        });

        // Extract values before moving config into Arc
        let api_port = config.api_port;
        let backend_url = config.backend_url.clone();
        
        let config = Arc::new(config);
        let counters = Arc::new(AtomicCounters::new());

        // Create LLM worker with backend URL from config
        let llm_worker = Arc::new(LLMWorker::new_with_backend(backend_url));

        // Semaphore for concurrent binary-file extractions.
        // Use half the logical cores so the LLM server is never CPU-starved.
        let max_concurrent = (num_cpus::get() / 2).max(1).min(8);
        info!("Attachment extraction semaphore: {} concurrent slots (num_cpus={})", max_concurrent, num_cpus::get());

        Ok(Self {
            conversations,
            llm_runtime: Arc::new(RwLock::new(None)),
            cache_manager: Arc::new(RwLock::new(None)),
            database_pool: database,
            config,
            counters,
            context_orchestrator: Arc::new(tokio::sync::RwLock::new(None)),
            llm_worker,
            model_manager: None,
            runtime_manager: Arc::new(std::sync::RwLock::new(None)),
            engine_manager: None,
            engine_available: Arc::new(AtomicBool::new(false)),
            initialization_complete: Arc::new(AtomicBool::new(false)),
            http_port: Arc::new(RwLock::new(api_port)),
            attachment_cache: Arc::new(DashMap::new()),
            extraction_semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent)),
        })
    }

    /// Mark initialization as complete - call this after all components are initialized
    pub fn mark_initialization_complete(&self) {
        self.initialization_complete.store(true, Ordering::SeqCst);
        info!("✅ Backend initialization marked as complete");
    }

    /// Check if initialization is complete
    pub fn is_initialization_complete(&self) -> bool {
        self.initialization_complete.load(Ordering::SeqCst)
    }

    /// Set LLM worker (replaces the default one created during initialization)
    pub fn set_llm_worker(&self, _worker: Arc<LLMWorker>) {
        // The llm_worker is now initialized in new() with the backend URL.
        // This method is kept for backward compatibility but is a no-op since
        // we initialize with the correct backend URL during construction.
        info!("LLM worker already initialized with backend URL");
    }

    /// Set runtime manager (allows setting after initialization)
    pub fn set_runtime_manager(&self, runtime_manager: Arc<RuntimeManager>) -> anyhow::Result<()> {
        // Update the runtime manager in shared state
        let mut guard = self.runtime_manager
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire runtime manager write lock: {}", e))?;
        *guard = Some(runtime_manager);
        Ok(())
    }

    /// Initialize LLM runtime with current configuration
    pub fn initialize_llm_runtime(&self) -> anyhow::Result<()> {
        let mut runtime_guard = self.llm_runtime.try_write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire LLM runtime write lock"))?;

        let runtime = LLMRuntime {
            model_path: self.config.model_path.clone(),
            context_size: self.config.ctx_size,
            batch_size: self.config.batch_size,
            threads: self.config.threads,
            gpu_layers: self.config.gpu_layers,
        };

        *runtime_guard = Some(runtime);
        info!("LLM runtime initialized");
        Ok(())
    }

    /// Get or create session data with proper locking
    pub async fn get_or_create_session(&self, session_id: &str) -> Arc<RwLock<SessionData>> {
        // Fast path: try to get existing session
        if let Some(session) = self.conversations.sessions.get(session_id) {
            return session.clone();
        }

        // Slow path: create new session
        let new_session = Arc::new(RwLock::new(SessionData {
            session_id: session_id.to_string(),
            messages: Vec::new(),
            last_accessed: std::time::Instant::now(),
            pinned: false,
        }));

        self.conversations.sessions.insert(session_id.to_string(), new_session.clone());
        self.counters.active_sessions.fetch_add(1, Ordering::Relaxed);

        new_session
    }

    /// Queue message for asynchronous processing
    pub fn queue_message(&self, session_id: &str, message: crate::memory::Message) -> bool {
        let queue = self.conversations.message_queues
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(crossbeam_queue::ArrayQueue::new(1000)));

        queue.push(PendingMessage {
            message,
            timestamp: std::time::Instant::now(),
        }).is_ok()
    }

    /// Process queued messages for a session
    pub async fn process_queued_messages(&self, session_id: &str) -> Vec<PendingMessage> {
        let mut messages = Vec::new();

        if let Some(queue) = self.conversations.message_queues.get(session_id) {
            while let Some(msg) = queue.pop() {
                messages.push(msg);
            }
        }

        messages
    }
}

impl ConversationHierarchy {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            message_queues: DashMap::new(),
            counters: Arc::new(AtomicCounters::new()),
        }
    }
}

/// Unified application state for all API handlers.
/// This is the single state type used by the Axum router, providing access
/// to all subsystems through shared memory (Arc) rather than network hops.
#[derive(Clone)]
pub struct UnifiedAppState {
    pub shared_state: Arc<SharedSystemState>,
    pub context_orchestrator: Arc<tokio::sync::RwLock<Option<ContextOrchestrator>>>,
    pub llm_worker: Arc<LLMWorker>,
    pub auth_state: Option<Arc<crate::api::auth_api::AuthState>>,
    /// Shared HTTP client — one TLS pool reused across all outbound requests
    /// (OpenRouter, HuggingFace, etc.) instead of creating a new client per call.
    pub http_client: reqwest::Client,
}

impl UnifiedAppState {
    pub fn new(shared_state: Arc<SharedSystemState>) -> Self {
        let context_orchestrator = shared_state.context_orchestrator.clone();
        let llm_worker = shared_state.llm_worker.clone();
        // Build a single shared client with a generous timeout for LLM streaming.
        // reqwest::Client is cheaply Clone (just bumps an Arc ref-count internally).
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            shared_state,
            context_orchestrator,
            llm_worker,
            auth_state: None,
            http_client,
        }
    }

    /// Get API key from all sources in priority order:
    /// 1. Database stored keys (persisted)
    /// 2. Environment variables
    /// 3. Config file
    /// 
    /// This ensures ultimate synchronicity - keys stored in DB are available
    /// to both backend and frontend immediately after saving.
    pub async fn get_openrouter_api_key(&self) -> Option<String> {
        // 1. First check database (highest priority - persisted)
        if let Ok(Some(key)) = self.shared_state.database_pool.api_keys.get_key_plaintext(&crate::memory_db::ApiKeyType::OpenRouter) {
            if !key.is_empty() {
                info!("Using OpenRouter API key from database");
                return Some(key);
            }
        }
        
        // 2. Check environment variable
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            if !key.is_empty() {
                info!("Using OpenRouter API key from environment variable");
                return Some(key);
            }
        }
        
        // 3. Check config
        let config = &self.shared_state.config;
        if !config.openrouter_api_key.is_empty() {
            info!("Using OpenRouter API key from config");
            return Some(config.openrouter_api_key.clone());
        }
        
        None
    }

    /// Get HuggingFace token from all sources in priority order:
    /// 1. Database stored keys (persisted)
    /// 2. Environment variables
    /// 3. Config file
    pub async fn get_huggingface_token(&self) -> Option<String> {
        // 1. First check database (highest priority - persisted)
        if let Ok(Some(token)) = self.shared_state.database_pool.api_keys.get_key_plaintext(&crate::memory_db::ApiKeyType::HuggingFace) {
            if !token.is_empty() {
                info!("Using HuggingFace token from database");
                return Some(token);
            }
        }
        
        // 2. Check environment variable
        // HUGGINGFACE_TOKEN or HF_TOKEN are common env var names
        if let Ok(token) = std::env::var("HUGGINGFACE_TOKEN").or_else(|_| std::env::var("HF_TOKEN")) {
            if !token.is_empty() {
                info!("Using HuggingFace token from environment variable");
                return Some(token);
            }
        }
        
        None
    }
}

// Re-exports for convenience
pub use self::SharedSystemState as SharedState;
