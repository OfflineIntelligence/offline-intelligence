//! Model Downloader
//!
//! Downloads models from various sources:
//! - Hugging Face Hub

use super::{
    progress::{DownloadStatus, ProgressTracker},
    registry::ModelInfo,
    storage::{ModelStorage, ModelMetadata, HardwareRequirements},
};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{error, info};

/// Source from which to download a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadSource {
    HuggingFace { repo_id: String, filename: String },
}

/// Model downloader service
pub struct ModelDownloader {
    storage: Arc<ModelStorage>,
    progress_tracker: Arc<ProgressTracker>,
    http_client: Client,
}

impl ModelDownloader {
    pub fn new(storage: Arc<ModelStorage>) -> Self {
        let client = Client::builder()
            .user_agent("OfflineIntelligence/0.1.4")
            .timeout(std::time::Duration::from_secs(3300))  // 55 minutes for large model downloads
            .build()
            .expect("Failed to create HTTP client");

        Self {
            storage,
            progress_tracker: Arc::new(ProgressTracker::new()),
            http_client: client,
        }
    }

    /// Download a model from the specified source
    /// If `existing_download_id` is provided, uses that for progress tracking instead of creating a new one
    /// If `hf_token` is provided, it will be used for HuggingFace authentication
    pub async fn download_model(
        &self,
        model_info: ModelInfo,
        source: DownloadSource,
        existing_download_id: Option<String>,
        hf_token: Option<String>,
    ) -> Result<String> {
        info!("Starting download of model: {} from {:?}", model_info.name, source);

        // Use existing download ID if provided, otherwise create a new one
        let download_id = if let Some(id) = existing_download_id {
            info!("Using existing download ID: {}", id);
            id
        } else {
            // Start progress tracking with a new ID
            self.progress_tracker
                .start_download(
                    model_info.id.clone(),
                    model_info.name.clone(),
                    Some(model_info.size_bytes),
                )
                .await
        };

        // Update status to starting
        self.progress_tracker
            .update_progress(&download_id, 0, DownloadStatus::Starting, None)
            .await;

        // Create model directory
        let model_dir = self.storage.create_model_directory(&model_info.id)
            .context("Failed to create model directory")?;

        let result = match source {
            DownloadSource::HuggingFace { repo_id, filename } => {
                self.download_from_huggingface(&download_id, &repo_id, &filename, &model_dir, &model_info, hf_token).await
            }
        };

        match result {
            Ok(_) => {
                info!("Successfully downloaded model: {}", model_info.name);
                self.progress_tracker
                    .update_progress(&download_id, model_info.size_bytes, DownloadStatus::Completed, None)
                    .await;
                Ok(download_id)
            }
            Err(e) => {
                error!("Failed to download model {}: {}", model_info.name, e);
                self.progress_tracker
                    .update_progress(&download_id, 0, DownloadStatus::Failed, Some(e.to_string()))
                    .await;
                Err(e)
            }
        }
    }

    /// Download from Hugging Face Hub
    async fn download_from_huggingface(
        &self,
        download_id: &str,
        repo_id: &str,
        filename: &str,
        model_dir: &std::path::Path,
        model_info: &ModelInfo,
        hf_token: Option<String>,
    ) -> Result<()> {
        // Check if this is a sharded model by looking for the shard pattern
        if let Some(total_shards) = self.detect_shard_pattern(filename) {
            info!("Detected sharded model with {} parts, downloading all shards", total_shards);
            self.download_sharded_model(download_id, repo_id, filename, model_dir, total_shards, hf_token).await
        } else {
            let url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, filename);
            info!("Downloading from HuggingFace: {}", url);
            self.download_file_with_progress(download_id, &url, model_dir, filename, model_info, hf_token).await
        }
    }

    /// Detect if the filename follows a shard pattern (e.g., model-00001-of-00003.gguf)
    fn detect_shard_pattern(&self, filename: &str) -> Option<u32> {
        // Pattern: some-name-00001-of-00003.ext
        let re = regex::Regex::new(r".*-(\d{5})-of-(\d{5})\.[^.]+$").ok()?;
        if let Some(caps) = re.captures(filename) {
            if let Some(total_str) = caps.get(2) {
                if let Ok(total) = total_str.as_str().parse::<u32>() {
                    return Some(total);
                }
            }
        }
        None
    }

    /// Download all shards of a sharded model
    async fn download_sharded_model(
        &self,
        download_id: &str,
        repo_id: &str,
        first_filename: &str,
        model_dir: &std::path::Path,
        total_shards: u32,
        hf_token: Option<String>,
    ) -> Result<()> {
        // Extract the pattern from the first filename to construct other shard names
        let re = regex::Regex::new(r"(.*-)(\d{5})(-of-\d{5}\.[^.]+)$").unwrap();
        let caps = re.captures(first_filename).ok_or_else(|| {
            anyhow::anyhow!("Invalid shard filename format: {}", first_filename)
        })?;

        let prefix = &caps[1];
        let suffix = &caps[3];

        // Calculate total size for progress tracking
        let mut total_expected_size = 0u64;
        for i in 1..=total_shards {
            let shard_filename = format!("{}{:05}{}", prefix, i, suffix);
            let url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, shard_filename);
            
            // Get the size of each shard
            let response = self.http_client.head(&url).send().await?;
            if response.status().is_success() {
                if let Some(content_length) = response.content_length() {
                    total_expected_size += content_length;
                }
            }
        }

        // Update progress tracker with total size
        self.progress_tracker.update_total_bytes(download_id, total_expected_size).await;

        // Download each shard
        let mut downloaded_so_far = 0u64;
        for i in 1..=total_shards {
            let shard_filename = format!("{}{:05}{}", prefix, i, suffix);
            let url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, shard_filename);
            
            info!("Downloading shard {}/{}: {}", i, total_shards, shard_filename);

            // Download the shard file
            self.download_single_shard_with_progress(
                download_id,
                &url,
                model_dir,
                &shard_filename,
                hf_token.clone(),
                &mut downloaded_so_far,
            ).await?;
        }

        info!("Successfully downloaded all {} shards", total_shards);
        Ok(())
    }

    /// Download a single shard with progress tracking
    async fn download_single_shard_with_progress(
        &self,
        download_id: &str,
        url: &str,
        model_dir: &std::path::Path,
        filename: &str,
        hf_token: Option<String>,
        downloaded_so_far: &mut u64,
    ) -> Result<()> {
        use futures_util::StreamExt;

        // Build request with optional HF_TOKEN authentication
        let mut request = self.http_client.get(url);
        
        // Add HuggingFace token if available (provided token takes precedence over env var)
        let token_to_use = hf_token.or_else(|| self.get_hf_token());
        if let Some(hf_token) = token_to_use {
            request = request.header("Authorization", format!("Bearer {}", hf_token));
        }

        let response = request
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_msg = if status == 401 {
                // Extract repo_id from URL for the gated model link
                let repo_id = url.strip_prefix("https://huggingface.co/")
                    .and_then(|s| s.split("/resolve/").next())
                    .unwrap_or("unknown");
                
                format!(
                    "Download failed with HTTP status: 401 Unauthorized.\n\n\
                    This may be because:\n\
                    1. The model requires authentication - check your HF_TOKEN\n\
                    2. The model is gated and requires terms acceptance at huggingface.co\n\
                    3. You've hit the unauthenticated rate limit (100 req/hour)\n\
                    4. The model is private or has been removed\n\n\
                    To fix:\n\
                    - Get a token from https://huggingface.co/settings/tokens\n\
                    - Visit https://huggingface.co/{} to request access to gated models\n\
                    - Set your token in the app settings and try again\n\n\
                    REPO_ID:{}",
                    repo_id, repo_id
                )
            } else {
                format!("Download failed with HTTP status: {}", status)
            };
            return Err(anyhow::anyhow!(error_msg));
        }

        // Get size of this shard from Content-Length header
        let shard_size = response.content_length().unwrap_or(0);
        
        let mut file = tokio::fs::File::create(model_dir.join(filename)).await?;
        let start_time = std::time::Instant::now();

        // Stream the response in chunks for real-time progress
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if let Some(progress) = self.progress_tracker.get_progress(download_id).await {
                if progress.status == DownloadStatus::Cancelled {
                    // Clean up partial file
                    let _ = tokio::fs::remove_file(model_dir.join(filename)).await;
                    return Err(anyhow::anyhow!("Download cancelled by user"));
                }
                
                // Check for pause status
                if progress.status == DownloadStatus::Paused {
                    // Wait until the download is resumed
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Check every 100ms
                        if let Some(updated_progress) = self.progress_tracker.get_progress(download_id).await {
                            if updated_progress.status != DownloadStatus::Paused {
                                break; // Exit the pause loop when resumed or status changed
                            }
                        } else {
                            // If progress is no longer tracked, exit
                            break;
                        }
                    }
                    
                    // After resuming, check if we should continue or stop
                    if let Some(updated_progress) = self.progress_tracker.get_progress(download_id).await {
                        if updated_progress.status == DownloadStatus::Cancelled {
                            // Clean up partial file
                            let _ = tokio::fs::remove_file(model_dir.join(filename)).await;
                            return Err(anyhow::anyhow!("Download cancelled by user"));
                        }
                    }
                }
            }

            let chunk = chunk_result.context("Error reading download stream")?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        
        // Update cumulative progress
        *downloaded_so_far += shard_size;
        let elapsed = start_time.elapsed();
        self.progress_tracker.update_elapsed_time(download_id, elapsed).await;
        self.progress_tracker
            .update_progress(download_id, *downloaded_so_far, DownloadStatus::Downloading, None)
            .await;

        Ok(())
    }

    /// Get HuggingFace token from environment variables.
    /// Checks `HF_TOKEN` and `HUGGINGFACE_TOKEN` (either name works).
    fn get_hf_token(&self) -> Option<String> {
        std::env::var("HF_TOKEN")
            .or_else(|_| std::env::var("HUGGINGFACE_TOKEN"))
            .ok()
            .filter(|t| !t.is_empty())
    }

    /// Download a file with progress tracking using chunked streaming
    async fn download_file_with_progress(
        &self,
        download_id: &str,
        url: &str,
        model_dir: &std::path::Path,
        filename: &str,
        model_info: &ModelInfo,
        hf_token: Option<String>,
    ) -> Result<()> {
        use futures_util::StreamExt;

        // Build request with optional HF_TOKEN authentication
        let mut request = self.http_client.get(url);
        
        // Add HuggingFace token if available (provided token takes precedence over env var)
        let token_to_use = hf_token.or_else(|| self.get_hf_token());
        if let Some(hf_token) = token_to_use {
            request = request.header("Authorization", format!("Bearer {}", hf_token));
        }

        let response = request
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_msg = if status == 401 {
                // Extract repo_id from URL for the gated model link
                let repo_id = url.strip_prefix("https://huggingface.co/")
                    .and_then(|s| s.split("/resolve/").next())
                    .unwrap_or("unknown");
                
                format!(
                    "Download failed with HTTP status: 401 Unauthorized.\n\n\
                    This may be because:\n\
                    1. The model requires authentication - check your HF_TOKEN\n\
                    2. The model is gated and requires terms acceptance at huggingface.co\n\
                    3. You've hit the unauthenticated rate limit (100 req/hour)\n\
                    4. The model is private or has been removed\n\n\
                    To fix:\n\
                    - Get a token from https://huggingface.co/settings/tokens\n\
                    - Visit https://huggingface.co/{} to request access to gated models\n\
                    - Set your token in the app settings and try again\n\n\
                    REPO_ID:{}",
                    repo_id, repo_id
                )
            } else {
                format!("Download failed with HTTP status: {}", status)
            };
            return Err(anyhow::anyhow!(error_msg));
        }

        // Get total size from Content-Length header or fall back to model_info.size_bytes
        let total_size = response.content_length().unwrap_or(model_info.size_bytes);
        
        // Update progress tracker with actual size from HTTP response if available
        if total_size > 0 {
            self.progress_tracker
                .update_total_bytes(download_id, total_size)
                .await;
        }
        
        let mut file = File::create(model_dir.join(filename)).await?;
        let mut downloaded: u64 = 0;
        let start_time = std::time::Instant::now();

        self.progress_tracker
            .update_progress(download_id, 0, DownloadStatus::Downloading, None)
            .await;

        // Stream the response in chunks for real-time progress
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if let Some(progress) = self.progress_tracker.get_progress(download_id).await {
                if progress.status == DownloadStatus::Cancelled {
                    // Clean up partial file
                    let _ = tokio::fs::remove_file(model_dir.join(filename)).await;
                    return Err(anyhow::anyhow!("Download cancelled by user"));
                }
                
                // Check for pause status
                if progress.status == DownloadStatus::Paused {
                    // Wait until the download is resumed
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Check every 100ms
                        if let Some(updated_progress) = self.progress_tracker.get_progress(download_id).await {
                            if updated_progress.status != DownloadStatus::Paused {
                                break; // Exit the pause loop when resumed or status changed
                            }
                        } else {
                            // If progress is no longer tracked, exit
                            break;
                        }
                    }
                    
                    // After resuming, check if we should continue or stop
                    if let Some(updated_progress) = self.progress_tracker.get_progress(download_id).await {
                        if updated_progress.status == DownloadStatus::Cancelled {
                            // Clean up partial file
                            let _ = tokio::fs::remove_file(model_dir.join(filename)).await;
                            return Err(anyhow::anyhow!("Download cancelled by user"));
                        }
                    }
                }
            }

            let chunk = chunk_result.context("Error reading download stream")?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // Update elapsed time and progress
            let elapsed = start_time.elapsed();
            self.progress_tracker.update_elapsed_time(download_id, elapsed).await;
            self.progress_tracker
                .update_progress(download_id, downloaded, DownloadStatus::Downloading, None)
                .await;
        }

        file.flush().await?;
        Ok(())
    }

    /// Save model metadata after successful download
    pub async fn save_model_metadata(
        &self,
        model_info: &ModelInfo,
        source: &DownloadSource,
    ) -> Result<()> {
        let metadata = ModelMetadata {
            id: model_info.id.clone(),
            name: model_info.name.clone(),
            description: model_info.description.clone(),
            author: model_info.author.clone(),
            size_bytes: model_info.size_bytes,
            format: model_info.format.clone(),
            download_source: match source {
                DownloadSource::HuggingFace { .. } => "huggingface".to_string(),
            },
            download_date: chrono::Utc::now(),
            last_used: None,
            tags: model_info.tags.clone(),
            hardware_requirements: HardwareRequirements::default(),
            compatibility_notes: None,
            runtime_binaries: self.get_appropriate_runtime_binaries_for_model(&model_info.format).await, // Populate with platform-appropriate binaries based on model format
        };

        let metadata_path = self.storage.metadata_path(&model_info.id);
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        tokio::fs::write(&metadata_path, metadata_json).await?;

        Ok(())
    }

    /// Get reference to progress tracker
    pub fn progress_tracker(&self) -> &Arc<ProgressTracker> {
        &self.progress_tracker
    }

    /// Cancel an ongoing download
    pub async fn cancel_download(&self, download_id: &str) -> bool {
        self.progress_tracker.cancel_download(download_id).await
    }
    
    /// Get appropriate runtime binaries for a given model format based on the current platform
    async fn get_appropriate_runtime_binaries_for_model(&self, _model_format: &str) -> std::collections::HashMap<String, std::path::PathBuf> {
        use crate::model_runtime::platform_detector::HardwareCapabilities;
        
        let mut binaries = std::collections::HashMap::new();
        let hw_caps = HardwareCapabilities::default();
        
        // Get the platform-appropriate binary path
        if let Some(platform_binary) = hw_caps.get_runtime_binary_path() {
            // Use the platform-specific binary for this format
            let platform_name = match hw_caps.platform {
                crate::model_runtime::platform_detector::Platform::Windows => "windows",
                crate::model_runtime::platform_detector::Platform::Linux => "linux",
                crate::model_runtime::platform_detector::Platform::MacOS => "macos",
            }.to_string();
            
            binaries.insert(platform_name, platform_binary);
        }
        
        // Also add generic format mappings
        binaries.insert("default".to_string(), std::path::PathBuf::from("llama-server"));
        
        binaries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_downloader_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Arc::new(ModelStorage {
            location: super::super::storage::StorageLocation {
                app_data_dir: temp_dir.path().to_path_buf(),
                models_dir: temp_dir.path().join("models"),
                registry_dir: temp_dir.path().join("registry"),
            },
        });

        let downloader = ModelDownloader::new(storage);
        assert!(downloader.progress_tracker().get_all_downloads().await.is_empty());
        
        Ok(())
    }
}