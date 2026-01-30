//! Java bindings for the Offline Intelligence Library using JNI
use jni::JNIEnv;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jstring, jlong, jobject, jboolean};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Message class wrapper
pub struct JavaMessage {
    pub role: String,
    pub content: String,
}

/// Main library interface
pub struct OfflineIntelligenceJNI {
    rt: Arc<Runtime>,
}

impl OfflineIntelligenceJNI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let rt = Runtime::new()?;
        Ok(OfflineIntelligenceJNI {
            rt: Arc::new(rt),
        })
    }
}

/// JNI function to create new OfflineIntelligence instance
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_newInstance(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    match OfflineIntelligenceJNI::new() {
        Ok(instance) => Box::into_raw(Box::new(instance)) as jlong,
        Err(_) => 0,
    }
}

/// JNI function to optimize context
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_optimizeContext(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    session_id: JString,
    messages: jobject,
    user_query: JString,
) -> jobject {
    let instance = unsafe { &*(ptr as *const OfflineIntelligenceJNI) };
    
    // Convert Java strings to Rust
    let session_id: String = env.get_string(&session_id)
        .expect("Couldn't get session_id string!")
        .into();
    
    let user_query_opt = if user_query.is_null() {
        None
    } else {
        Some(env.get_string(&user_query)
            .expect("Couldn't get user_query string!")
            .into())
    };
    
    // Create result object
    let result_class = env.find_class("com/offlineintelligence/OptimizationResult")
        .expect("Couldn't find OptimizationResult class");
    
    let result_object = env.new_object(result_class, "()V", &[])
        .expect("Couldn't create OptimizationResult object");
    
    // Set fields (placeholder values)
    env.set_field(result_object, "originalCount", "I", JValue::Int(0))
        .expect("Couldn't set originalCount field");
    env.set_field(result_object, "optimizedCount", "I", JValue::Int(0))
        .expect("Couldn't set optimizedCount field");
    env.set_field(result_object, "compressionRatio", "F", JValue::Float(0.0))
        .expect("Couldn't set compressionRatio field");
    
    result_object
}

/// JNI function to search memory
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_search(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    query: JString,
    session_id: JString,
    limit: i32,
) -> jobject {
    let instance = unsafe { &*(ptr as *const OfflineIntelligenceJNI) };
    
    let query: String = env.get_string(&query)
        .expect("Couldn't get query string!")
        .into();
    
    let session_id_opt = if session_id.is_null() {
        None
    } else {
        Some(env.get_string(&session_id)
            .expect("Couldn't get session_id string!")
            .into())
    };
    
    // Create result object
    let result_class = env.find_class("com/offlineintelligence/SearchResult")
        .expect("Couldn't find SearchResult class");
    
    let result_object = env.new_object(result_class, "()V", &[])
        .expect("Couldn't create SearchResult object");
    
    // Set fields (placeholder values)
    env.set_field(result_object, "total", "I", JValue::Int(0))
        .expect("Couldn't set total field");
    env.set_field(result_object, "searchType", "Ljava/lang/String;", 
        JValue::Object(*env.new_string("keyword")
            .expect("Couldn't create string")))
        .expect("Couldn't set searchType field");
    
    result_object
}

/// JNI function to generate title
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_generateTitle(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    messages: jobject,
) -> jstring {
    let instance = unsafe { &*(ptr as *const OfflineIntelligenceJNI) };
    
    let title = env.new_string("Generated Title")
        .expect("Couldn't create title string");
    
    title.into_inner()
}

/// JNI function to dispose instance
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_dispose(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    if ptr != 0 {
        unsafe {
            let _instance = Box::from_raw(ptr as *mut OfflineIntelligenceJNI);
        }
    }
}