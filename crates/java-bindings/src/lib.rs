use jni::JNIEnv;
use jni::objects::{JClass, JString, JValue};
use jni::sys::{jstring, jlong, jobject};
use std::sync::Arc;
use tokio::runtime::Runtime;
/
pub struct JavaMessage {
    pub role: String,
    pub content: String,
}
/
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
/
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
/
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_optimizeContext(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    session_id: JString,
    _messages: jobject,
    user_query: JString,
) -> jobject {
    let _instance = unsafe { &*(ptr as *const OfflineIntelligenceJNI) };


    let _session_id: String = env.get_string(&session_id)
        .expect("Couldn't get session_id string!")
        .into();

    let _user_query_opt: Option<String> = if user_query.is_null() {
        None
    } else {
        Some(env.get_string(&user_query)
            .expect("Couldn't get user_query string!")
            .into())
    };


    let result_class = env.find_class("com/offlineintelligence/OptimizationResult")
        .expect("Couldn't find OptimizationResult class");

    let result_object = env.new_object(result_class, "()V", &[])
        .expect("Couldn't create OptimizationResult object");


    let result_ref = result_object.as_ref();
    env.set_field(result_ref, "originalCount", "I", JValue::Int(0))
        .expect("Couldn't set originalCount field");
    env.set_field(result_ref, "optimizedCount", "I", JValue::Int(0))
        .expect("Couldn't set optimizedCount field");
    env.set_field(result_ref, "compressionRatio", "F", JValue::Float(0.0))
        .expect("Couldn't set compressionRatio field");

    result_object.as_raw()
}
/
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_search(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    query: JString,
    session_id: JString,
    _limit: i32,
) -> jobject {
    let _instance = unsafe { &*(ptr as *const OfflineIntelligenceJNI) };

    let _query: String = env.get_string(&query)
        .expect("Couldn't get query string!")
        .into();

    let _session_id_opt: Option<String> = if session_id.is_null() {
        None
    } else {
        Some(env.get_string(&session_id)
            .expect("Couldn't get session_id string!")
            .into())
    };


    let result_class = env.find_class("com/offlineintelligence/SearchResult")
        .expect("Couldn't find SearchResult class");

    let result_object = env.new_object(result_class, "()V", &[])
        .expect("Couldn't create SearchResult object");


    let result_ref = result_object.as_ref();
    env.set_field(result_ref, "total", "I", JValue::Int(0))
        .expect("Couldn't set total field");
    let keyword_str = env.new_string("keyword")
        .expect("Couldn't create string");
    env.set_field(result_ref, "searchType", "Ljava/lang/String;",
        JValue::Object(keyword_str.as_ref()))
        .expect("Couldn't set searchType field");

    result_object.as_raw()
}
/
#[no_mangle]
pub extern "system" fn Java_com_offlineintelligence_OfflineIntelligence_generateTitle(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    _messages: jobject,
) -> jstring {
    let _instance = unsafe { &*(ptr as *const OfflineIntelligenceJNI) };

    let title = env.new_string("Generated Title")
        .expect("Couldn't create title string");

    title.as_raw()
}
/
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

