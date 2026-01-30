use napi::{bindgen_prelude::*, JsObject, Env};
use napi_derive::napi;
/
#[napi]
pub fn hello_world() -> String {
    "Hello from Offline Intelligence!".to_string()
}
/
#[napi]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
// Simple module - functions are automatically exported by napi-derive

