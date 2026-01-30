//! Minimal JavaScript bindings for the Offline Intelligence Library using N-API
use napi::{bindgen_prelude::*, JsObject, Env};
use napi_derive::napi;

/// Simple function to test the binding
#[napi]
pub fn hello_world() -> String {
    "Hello from Offline Intelligence!".to_string()
}

/// Get library version
#[napi]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// Simple module - functions are automatically exported by napi-derive