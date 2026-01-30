
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::{info, warn};
#[derive(Clone)]
pub struct BackendTarget {
    inner: Arc<ArcSwap<String>>,
}
impl BackendTarget {
    pub fn new(initial: String) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(initial))),
        }
    }
    pub async fn set(&self, new_target: String) {
        let current = self.inner.load();


        if current.is_empty() {
            info!("ðŸ”„ Setting initial backend target to: {}", new_target);
            self.inner.store(Arc::new(new_target));
        }

        else if **current != new_target {
            info!("ðŸ”„ Switching backend target from {} â†’ {}", **current, new_target);
            self.inner.store(Arc::new(new_target));
        } else {
            warn!("backend target set() called, but no change (still {})", new_target);
        }
    }
    pub async fn get(&self) -> String {
        (**self.inner.load()).clone()
    }
    /
    pub async fn is_initialized(&self) -> bool {
        !self.inner.load().is_empty()
    }
    /
    /
    pub async fn generate_url(&self) -> String {
        let base = self.get().await;
        if base.is_empty() {
            warn!("Backend target not initialized yet, returning empty URL");
        }
        format!("{}/", base)
    }
    /
    pub async fn health_url(&self) -> String {
        let base = self.get().await;
        format!("{}/health", base)
    }
    /
    pub async fn chat_completions_url(&self) -> String {
        let base = self.get().await;
        format!("{}/v1/chat/completions", base)
    }
    /
    /
    pub async fn direct_generation_url(&self) -> String {
        let base = self.get().await;
        if base.is_empty() {
            warn!("Backend target not initialized yet, returning empty URL");
        }
        format!("{}/", base)
    }
}
