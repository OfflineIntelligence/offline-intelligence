
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HfAccessStatus {
    
    Accessible,
    
    NotApproved,
    
    Unauthorized,
    
    NotFound,
    
    Error(String),
}

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
        .redirect(reqwest::redirect::Policy::none()) 
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
            200 | 302 | 301 => HfAccessStatus::Accessible, 
            401 => HfAccessStatus::Unauthorized,
            403 => HfAccessStatus::NotApproved,
            404 => HfAccessStatus::NotFound,
            other => HfAccessStatus::Error(format!("Unexpected HTTP {}", other)),
        },
        Err(e) => HfAccessStatus::Error(format!("Network error: {}", e)),
    }
}
