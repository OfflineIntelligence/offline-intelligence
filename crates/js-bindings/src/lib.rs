//! JavaScript bindings for the Offline Intelligence Library using N-API
use napi::{bindgen_prelude::*, JsObject, Env};
use napi_derive::napi;
use serde_json;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Message structure for JavaScript
#[napi(object)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Configuration wrapper for JavaScript
#[napi]
pub struct Config {
    inner: offline_intelligence::Config,
}

#[napi]
impl Config {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        match offline_intelligence::Config::from_env() {
            Ok(config) => Ok(Config { inner: config }),
            Err(e) => Err(Error::new(
                Status::GenericFailure,
                format!("Failed to load config: {}", e)
            )),
        }
    }
    
    #[napi(getter)]
    pub fn model_path(&self) -> String {
        self.inner.model_path.clone()
    }
    
    #[napi(getter)]
    pub fn ctx_size(&self) -> u32 {
        self.inner.ctx_size
    }
    
    #[napi(getter)]
    pub fn batch_size(&self) -> u32 {
        self.inner.batch_size
    }
}

/// Main library interface
#[napi]
pub struct OfflineIntelligence {
    rt: Arc<Runtime>,
}

#[napi]
impl OfflineIntelligence {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        let rt = Runtime::new()
            .map_err(|e| Error::new(
                Status::GenericFailure,
                format!("Failed to create async runtime: {}", e)
            ))?;
        
        Ok(OfflineIntelligence {
            rt: Arc::new(rt),
        })
    }
    
    /// Optimize conversation context
    #[napi]
    pub fn optimize_context(&self, session_id: String, messages: Vec<Message>, user_query: Option<String>) -> Result<JsObject> {
        // Convert JavaScript messages to Rust messages
        let rust_messages: Vec<offline_intelligence::Message> = messages
            .into_iter()
            .map(|m| offline_intelligence::Message {
                role: m.role,
                content: m.content,
            })
            .collect();
        
        // Placeholder implementation - would need proper async handling
        Ok(JsObject::new())
    }
    
    /// Search memory
    #[napi]
    pub fn search(&self, query: String, session_id: Option<String>, limit: Option<i32>) -> Result<JsObject> {
        // Placeholder implementation
        Ok(JsObject::new())
    }
    
    /// Generate title for conversation
    #[napi]
    pub fn generate_title(&self, messages: Vec<Message>) -> Result<String> {
        // Placeholder implementation
        Ok("Generated Title".to_string())
    }
}

/// Module initialization
#[napi]
pub fn init_module(env: Env, exports: &mut JsObject) -> Result<()> {
    exports.set_named_property("version", env.create_string_from_std(env!("CARGO_PKG_VERSION").to_string())?)?;
    exports.set_named_property("author", env.create_string_from_std("Offline Intelligence Team".to_string())?)?;
    Ok(())
}