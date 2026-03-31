
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tracing::{debug, info};
use uuid::Uuid;

mod duration_as_secs_f64 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_f64(duration.as_secs_f64())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de> {
        let secs = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

mod option_duration_as_secs_f64 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        match duration {
            Some(d) => serializer.serialize_some(&d.as_secs_f64()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where D: Deserializer<'de> {
        let opt = Option::<f64>::deserialize(deserializer)?;
        Ok(opt.map(Duration::from_secs_f64))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub download_id: String,
    pub model_id: String,
    pub model_name: String,
    pub status: DownloadStatus,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f32,
    pub speed_bps: f64, 
    #[serde(with = "duration_as_secs_f64")]
    pub elapsed_time: std::time::Duration,
    #[serde(with = "option_duration_as_secs_f64")]
    pub estimated_time_remaining: Option<std::time::Duration>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Starting,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, DownloadStatus::Queued | DownloadStatus::Starting | DownloadStatus::Downloading)
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled)
    }
}

pub struct ProgressTracker {
    downloads: Arc<RwLock<HashMap<String, DownloadProgress>>>,
    watchers: Arc<RwLock<HashMap<String, watch::Sender<DownloadProgress>>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            downloads: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_download(
        &self,
        model_id: String,
        model_name: String,
        total_bytes: Option<u64>,
    ) -> String {
        let download_id = Uuid::new_v4().to_string();
        let progress = DownloadProgress {
            download_id: download_id.clone(),
            model_id,
            model_name,
            status: DownloadStatus::Queued,
            bytes_downloaded: 0,
            total_bytes,
            percentage: 0.0,
            speed_bps: 0.0,
            elapsed_time: std::time::Duration::from_secs(0),
            estimated_time_remaining: None,
            error_message: None,
        };

        {
            let mut downloads = self.downloads.write().await;
            downloads.insert(download_id.clone(), progress);
        }

        info!("Started tracking download: {}", download_id);
        download_id
    }

    pub async fn update_progress(
        &self,
        download_id: &str,
        bytes_downloaded: u64,
        status: DownloadStatus,
        error_message: Option<String>,
    ) {
        let mut downloads = self.downloads.write().await;
        
        if let Some(progress) = downloads.get_mut(download_id) {
            let old_bytes = progress.bytes_downloaded;
            progress.bytes_downloaded = bytes_downloaded;
            progress.status = status;
            progress.error_message = error_message;

            if let Some(total) = progress.total_bytes {
                if total > 0 {
                    progress.percentage = (bytes_downloaded as f32 / total as f32) * 100.0;
                }
            }

            if bytes_downloaded > old_bytes && progress.elapsed_time.as_secs() > 0 {
                let time_elapsed_secs = progress.elapsed_time.as_secs_f64();
                progress.speed_bps = bytes_downloaded as f64 / time_elapsed_secs;

                if let Some(total) = progress.total_bytes {
                    
                    let remaining_bytes = total.saturating_sub(bytes_downloaded);
                    if progress.speed_bps > 0.0 {
                        let eta_secs = remaining_bytes as f64 / progress.speed_bps;
                        progress.estimated_time_remaining = Some(std::time::Duration::from_secs_f64(eta_secs));
                    }
                }
            }

            self.notify_watchers(download_id, progress.clone()).await;
            
            debug!("Updated progress for {}: {:.1}%", download_id, progress.percentage);
        }
    }

    pub async fn update_elapsed_time(&self, download_id: &str, elapsed: std::time::Duration) {
        let mut downloads = self.downloads.write().await;
        if let Some(progress) = downloads.get_mut(download_id) {
            progress.elapsed_time = elapsed;
        }
    }

    pub async fn update_total_bytes(&self, download_id: &str, total_bytes: u64) {
        let mut downloads = self.downloads.write().await;
        if let Some(progress) = downloads.get_mut(download_id) {
            
            if progress.total_bytes.is_none() || progress.total_bytes != Some(total_bytes) {
                progress.total_bytes = Some(total_bytes);
                
                if total_bytes > 0 {
                    progress.percentage = (progress.bytes_downloaded as f32 / total_bytes as f32) * 100.0;
                    
                    if progress.speed_bps > 0.0 {
                        let remaining_bytes = total_bytes.saturating_sub(progress.bytes_downloaded);
                        let eta_secs = remaining_bytes as f64 / progress.speed_bps;
                        progress.estimated_time_remaining = Some(std::time::Duration::from_secs_f64(eta_secs));
                    }
                }
                
                self.notify_watchers(download_id, progress.clone()).await;
                debug!("Updated total_bytes for {}: {} bytes", download_id, total_bytes);
            }
        }
    }

    pub async fn get_progress(&self, download_id: &str) -> Option<DownloadProgress> {
        let downloads = self.downloads.read().await;
        downloads.get(download_id).cloned()
    }

    pub async fn get_active_downloads(&self) -> Vec<DownloadProgress> {
        let downloads = self.downloads.read().await;
        downloads.values()
            .filter(|p| p.status.is_active())
            .cloned()
            .collect()
    }

    pub async fn get_all_downloads(&self) -> Vec<DownloadProgress> {
        let downloads = self.downloads.read().await;
        downloads.values().cloned().collect()
    }

    pub async fn subscribe(&self, download_id: &str) -> Option<watch::Receiver<DownloadProgress>> {
        let mut watchers = self.watchers.write().await;
        let (tx, rx) = watch::channel(DownloadProgress {
            download_id: download_id.to_string(),
            model_id: String::new(),
            model_name: String::new(),
            status: DownloadStatus::Queued,
            bytes_downloaded: 0,
            total_bytes: None,
            percentage: 0.0,
            speed_bps: 0.0,
            elapsed_time: std::time::Duration::from_secs(0),
            estimated_time_remaining: None,
            error_message: None,
        });
        
        watchers.insert(download_id.to_string(), tx);
        Some(rx)
    }

    async fn notify_watchers(&self, download_id: &str, progress: DownloadProgress) {
        let watchers = self.watchers.read().await;
        if let Some(tx) = watchers.get(download_id) {
            let _ = tx.send(progress);
        }
    }

    pub async fn remove_download(&self, download_id: &str) {
        {
            let mut downloads = self.downloads.write().await;
            downloads.remove(download_id);
        }
        {
            let mut watchers = self.watchers.write().await;
            watchers.remove(download_id);
        }
        info!("Removed download tracking: {}", download_id);
    }

    pub async fn cancel_download(&self, download_id: &str) -> bool {
        let mut downloads = self.downloads.write().await;
        if let Some(progress) = downloads.get_mut(download_id) {
            if progress.status.is_active() {
                progress.status = DownloadStatus::Cancelled;
                self.notify_watchers(download_id, progress.clone()).await;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub async fn get_statistics(&self) -> DownloadStatistics {
        let downloads = self.downloads.read().await;
        let mut stats = DownloadStatistics::default();
        
        for progress in downloads.values() {
            match progress.status {
                DownloadStatus::Queued => stats.queued += 1,
                DownloadStatus::Starting => stats.starting += 1,
                DownloadStatus::Downloading => {
                    stats.downloading += 1;
                    stats.total_speed_bps += progress.speed_bps;
                },
                DownloadStatus::Paused => stats.paused += 1,
                DownloadStatus::Completed => stats.completed += 1,
                DownloadStatus::Failed => stats.failed += 1,
                DownloadStatus::Cancelled => stats.cancelled += 1,
            }
            
            if let Some(total) = progress.total_bytes {
                stats.total_data_bytes += total;
            }
            stats.downloaded_bytes += progress.bytes_downloaded;
        }
        
        stats
    }
}

#[derive(Debug, Default)]
pub struct DownloadStatistics {
    pub queued: usize,
    pub starting: usize,
    pub downloading: usize,
    pub paused: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total_speed_bps: f64,
    pub downloaded_bytes: u64,
    pub total_data_bytes: u64,
}

impl DownloadStatistics {
    pub fn active_downloads(&self) -> usize {
        self.queued + self.starting + self.downloading + self.paused
    }

    pub fn finished_downloads(&self) -> usize {
        self.completed + self.failed + self.cancelled
    }

    pub fn overall_percentage(&self) -> f32 {
        if self.total_data_bytes > 0 {
            (self.downloaded_bytes as f32 / self.total_data_bytes as f32) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_tracking() {
        let tracker = ProgressTracker::new();
        
        let download_id = tracker.start_download(
            "test-model".to_string(),
            "Test Model".to_string(),
            Some(1000)
        ).await;

        tracker.update_progress(&download_id, 500, DownloadStatus::Downloading, None).await;
        
        let progress = tracker.get_progress(&download_id).await.unwrap();
        assert_eq!(progress.bytes_downloaded, 500);
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.status, DownloadStatus::Downloading);
    }

    #[tokio::test]
    async fn test_subscription() {
        let tracker = ProgressTracker::new();
        
        let download_id = tracker.start_download(
            "test-model".to_string(),
            "Test Model".to_string(),
            Some(1000)
        ).await;

        let mut receiver = tracker.subscribe(&download_id).await.unwrap();
        
        tracker.update_progress(&download_id, 250, DownloadStatus::Downloading, None).await;
        
        let progress = receiver.borrow_and_update().clone();
        assert_eq!(progress.bytes_downloaded, 250);
    }
}
