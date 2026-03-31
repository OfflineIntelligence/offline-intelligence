
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

#[derive(Clone)]
pub struct PreExtracted {
    pub text: String,
    pub extracted_at: std::time::Instant,
}

impl PreExtracted {
    
    pub fn is_stale(&self, ttl_secs: u64) -> bool {
        self.extracted_at.elapsed().as_secs() >= ttl_secs
    }
}

pub struct SharedSystemState {
    
    pub conversations: Arc<ConversationHierarchy>,

    pub llm_runtime: Arc<RwLock<Option<LLMRuntime>>>,

    pub cache_manager: Arc<RwLock<Option<Arc<KVCacheManager>>>>,

    pub database_pool: Arc<MemoryDatabase>,

    pub config: Arc<Config>,

    pub counters: Arc<AtomicCounters>,

    pub context_orchestrator: Arc<tokio::sync::RwLock<Option<ContextOrchestrator>>>,

    pub llm_worker: Arc<LLMWorker>,

    pub model_manager: Option<Arc<ModelManager>>,

    pub runtime_manager: Arc<std::sync::RwLock<Option<Arc<RuntimeManager>>>>,

    pub engine_manager: Option<Arc<EngineManager>>,

    pub engine_available: Arc<AtomicBool>,

    pub initialization_complete: Arc<AtomicBool>,
    
    pub http_port: Arc<RwLock<u16>>,

    pub attachment_cache: Arc<DashMap<String, PreExtracted>>,

    pub extraction_semaphore: Arc<tokio::sync::Semaphore>,
}

pub struct ConversationHierarchy {
    
    pub sessions: DashMap<String, Arc<RwLock<SessionData>>>,

    pub message_queues: DashMap<String, Arc<crossbeam_queue::ArrayQueue<PendingMessage>>>,

    pub counters: Arc<AtomicCounters>,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub messages: Vec<crate::memory::Message>,
    pub last_accessed: std::time::Instant,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct PendingMessage {
    pub message: crate::memory::Message,
    pub timestamp: std::time::Instant,
}

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

pub struct LLMRuntime {
    pub model_path: String,
    pub context_size: u32,
    pub batch_size: u32,
    pub threads: u32,
    pub gpu_layers: u32,
    
}

impl SharedSystemState {
    pub fn new(config: Config, database: Arc<MemoryDatabase>) -> anyhow::Result<Self> {
        info!("Initializing shared system state");

        let conversations = Arc::new(ConversationHierarchy {
            sessions: DashMap::new(),
            message_queues: DashMap::new(),
            counters: Arc::new(AtomicCounters::new()),
        });

        let api_port = config.api_port;
        let backend_url = config.backend_url.clone();
        
        let config = Arc::new(config);
        let counters = Arc::new(AtomicCounters::new());

        let llm_worker = Arc::new(LLMWorker::new_with_backend(backend_url));

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

    pub fn mark_initialization_complete(&self) {
        self.initialization_complete.store(true, Ordering::SeqCst);
        info!("✅ Backend initialization marked as complete");
    }

    pub fn is_initialization_complete(&self) -> bool {
        self.initialization_complete.load(Ordering::SeqCst)
    }

    pub fn set_llm_worker(&self, _worker: Arc<LLMWorker>) {
        
        info!("LLM worker already initialized with backend URL");
    }

    pub fn set_runtime_manager(&self, runtime_manager: Arc<RuntimeManager>) -> anyhow::Result<()> {
        
        let mut guard = self.runtime_manager
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire runtime manager write lock: {}", e))?;
        *guard = Some(runtime_manager);
        Ok(())
    }

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

    pub async fn get_or_create_session(&self, session_id: &str) -> Arc<RwLock<SessionData>> {
        
        if let Some(session) = self.conversations.sessions.get(session_id) {
            return session.clone();
        }

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

    pub fn queue_message(&self, session_id: &str, message: crate::memory::Message) -> bool {
        let queue = self.conversations.message_queues
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(crossbeam_queue::ArrayQueue::new(1000)));

        queue.push(PendingMessage {
            message,
            timestamp: std::time::Instant::now(),
        }).is_ok()
    }

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

#[derive(Clone)]
pub struct UnifiedAppState {
    pub shared_state: Arc<SharedSystemState>,
    pub context_orchestrator: Arc<tokio::sync::RwLock<Option<ContextOrchestrator>>>,
    pub llm_worker: Arc<LLMWorker>,
    pub auth_state: Option<Arc<crate::api::auth_api::AuthState>>,
    
    pub http_client: reqwest::Client,
}

impl UnifiedAppState {
    pub fn new(shared_state: Arc<SharedSystemState>) -> Self {
        let context_orchestrator = shared_state.context_orchestrator.clone();
        let llm_worker = shared_state.llm_worker.clone();
        
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

    pub async fn get_openrouter_api_key(&self) -> Option<String> {
        
        if let Ok(Some(key)) = self.shared_state.database_pool.api_keys.get_key_plaintext(&crate::memory_db::ApiKeyType::OpenRouter) {
            if !key.is_empty() {
                info!("Using OpenRouter API key from database");
                return Some(key);
            }
        }
        
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            if !key.is_empty() {
                info!("Using OpenRouter API key from environment variable");
                return Some(key);
            }
        }
        
        let config = &self.shared_state.config;
        if !config.openrouter_api_key.is_empty() {
            info!("Using OpenRouter API key from config");
            return Some(config.openrouter_api_key.clone());
        }
        
        None
    }

    pub async fn get_huggingface_token(&self) -> Option<String> {
        
        if let Ok(Some(token)) = self.shared_state.database_pool.api_keys.get_key_plaintext(&crate::memory_db::ApiKeyType::HuggingFace) {
            if !token.is_empty() {
                info!("Using HuggingFace token from database");
                return Some(token);
            }
        }
        
        if let Ok(token) = std::env::var("HUGGINGFACE_TOKEN").or_else(|_| std::env::var("HF_TOKEN")) {
            if !token.is_empty() {
                info!("Using HuggingFace token from environment variable");
                return Some(token);
            }
        }
        
        None
    }
}

pub use self::SharedSystemState as SharedState;
