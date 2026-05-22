//! HuggingFace Gated Model Access Checker
//!
//! Uses a HEAD request to determine whether the current HF token can download
//! a specific model file — the most direct proxy for "access approved".
//!
//! HTTP status meanings:
//!   200  → file reachable → access granted
//!   401  → no/bad token   → not authenticated
//!   403  → token valid but model not approved yet
//!   404  → file not found (repo or filename wrong)
//!   other → treat as unknown/error

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Result of a gated-access check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HfAccessStatus {
    /// Token has access — download can proceed
    Accessible,
    /// Token is valid but the user has NOT been approved by the repo owner yet
    NotApproved,
    /// No token provided or token is invalid / expired
    Unauthorized,
    /// The repo or file was not found — repo_id / filename may be wrong
    NotFound,
    /// Network or unexpected HTTP error
    Error(String),
}

/// Check whether the caller's HF token can download `filename` from `repo_id`.
///
/// # Arguments
/// * `repo_id`   – HuggingFace repo, e.g. `"meta-llama/Meta-Llama-3-8B-Instruct"`
/// * `filename`  – A file that lives in the repo, e.g. `"model.gguf"`
/// * `hf_token`  – Optional Bearer token; `None` tests unauthenticated access
pub async fn check_hf_gated_access(
    repo_id: &str,
    filename: &str,
    hf_token: Option<&str>,
) -> HfAccessStatus {
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename
    );

    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none()) // don't follow redirects
        .build()
    {
        Ok(c) => c,
        Err(e) => return HfAccessStatus::Error(format!("Failed to build HTTP client: {}", e)),
    };

    let mut req = client.head(&url);
    if let Some(token) = hf_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    match req.send().await {
        Ok(resp) => match resp.status().as_u16() {
            200 | 302 | 301 => HfAccessStatus::Accessible, // redirect = CDN redirect, still accessible
            401 => HfAccessStatus::Unauthorized,
            403 => HfAccessStatus::NotApproved,
            404 => HfAccessStatus::NotFound,
            other => HfAccessStatus::Error(format!("Unexpected HTTP {}", other)),
        },
        Err(e) => HfAccessStatus::Error(format!("Network error: {}", e)),
    }
}
