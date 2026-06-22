use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Queued,
    FetchingInfo,
    Downloading { progress: f32, speed: String, eta: String },
    Merging,
    Done { path: String },
    Failed { reason: String },
}

impl DownloadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            DownloadStatus::Queued => "QUEUED",
            DownloadStatus::FetchingInfo => "FETCHING",
            DownloadStatus::Downloading { .. } => "DOWNLOADING",
            DownloadStatus::Merging => "MERGING",
            DownloadStatus::Done { .. } => "DONE",
            DownloadStatus::Failed { .. } => "FAILED",
        }
    }

    pub fn progress_pct(&self) -> f64 {
        match self {
            DownloadStatus::Queued => 0.0,
            DownloadStatus::FetchingInfo => 5.0,
            DownloadStatus::Downloading { progress, .. } => *progress as f64,
            DownloadStatus::Merging => 98.0,
            DownloadStatus::Done { .. } => 100.0,
            DownloadStatus::Failed { .. } => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub platform: Option<String>,
    pub thumbnail: Option<String>,
    pub duration_secs: Option<u64>,
    pub format: String,
    pub audio_only: bool,
    pub output_dir: String,
    pub status: DownloadStatus,
    pub added_at: DateTime<Utc>,
}

impl DownloadItem {
    pub fn new(url: String, format: String, audio_only: bool, output_dir: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            url,
            title: None,
            platform: None,
            thumbnail: None,
            duration_secs: None,
            format,
            audio_only,
            output_dir,
            status: DownloadStatus::Queued,
            added_at: Utc::now(),
        }
    }

}
