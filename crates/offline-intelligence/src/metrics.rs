// _Aud.CLI/_Server/metrics.rs

use prometheus::{Encoder, TextEncoder, Registry, IntCounterVec, IntGauge, Histogram, HistogramVec};
use lazy_static::lazy_static;
use std::sync::Mutex;
use axum::response::IntoResponse;
use axum::http::StatusCode;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref REQ_COUNTER: Mutex<IntCounterVec> = Mutex::new(
        IntCounterVec::new(
            prometheus::opts!("requests_total", "Total requests per route"),
            &["route", "status"]
        ).unwrap()
    );
    static ref ACTIVE_SESSIONS: Mutex<IntGauge> = Mutex::new(
        IntGauge::new("active_sessions", "Active streaming sessions").unwrap()
    );
    static ref QUEUE_DEPTH: Mutex<IntGauge> = Mutex::new(
        IntGauge::new("queue_depth", "Number of requests waiting in queue").unwrap()
    );
    static ref RESPONSE_TIME: Mutex<HistogramVec> = Mutex::new(
        HistogramVec::new(
            prometheus::HistogramOpts::new(
                "response_time_seconds",
                "Response time by endpoint"
            ),
            &["endpoint"]
        ).unwrap()
    );
    static ref CONTEXT_OPTIMIZATION_TIME: Mutex<Histogram> = Mutex::new(
        Histogram::with_opts(prometheus::HistogramOpts::new(
            "context_optimization_time_seconds",
            "Time spent in context optimization"
        )).unwrap()
    );
    static ref BACKEND_LATENCY: Mutex<Histogram> = Mutex::new(
        Histogram::with_opts(prometheus::HistogramOpts::new(
            "backend_latency_seconds",
            "Latency to backend service"
        )).unwrap()
    );
}

pub fn init_metrics() {
    REGISTRY.register(Box::new(REQ_COUNTER.lock().unwrap().clone())).ok();
    REGISTRY.register(Box::new(ACTIVE_SESSIONS.lock().unwrap().clone())).ok();
    REGISTRY.register(Box::new(QUEUE_DEPTH.lock().unwrap().clone())).ok();
    REGISTRY.register(Box::new(RESPONSE_TIME.lock().unwrap().clone())).ok();
    REGISTRY.register(Box::new(CONTEXT_OPTIMIZATION_TIME.lock().unwrap().clone())).ok();
    REGISTRY.register(Box::new(BACKEND_LATENCY.lock().unwrap().clone())).ok();
}

// Add this missing function
pub fn inc_request(route: &str, status: &str) {
    REQ_COUNTER.lock().unwrap().with_label_values(&[route, status]).inc();
}

pub fn inc_sessions() {
    ACTIVE_SESSIONS.lock().unwrap().inc();
}

pub fn dec_sessions() {
    ACTIVE_SESSIONS.lock().unwrap().dec();
}

pub fn inc_queue() {
    QUEUE_DEPTH.lock().unwrap().inc();
}

pub fn dec_queue() {
    QUEUE_DEPTH.lock().unwrap().dec();
}

pub fn observe_queue_wait(duration: f64) {
    // Placeholder for queue wait time
}

pub fn observe_response_time(endpoint: &str, duration: f64) {
    RESPONSE_TIME.lock().unwrap().with_label_values(&[endpoint]).observe(duration);
}

pub fn observe_context_optimization(duration: f64) {
    CONTEXT_OPTIMIZATION_TIME.lock().unwrap().observe(duration);
}

pub fn observe_backend_latency(duration: f64) {
    BACKEND_LATENCY.lock().unwrap().observe(duration);
}

pub async fn get_metrics() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        buffer,
    )
}
