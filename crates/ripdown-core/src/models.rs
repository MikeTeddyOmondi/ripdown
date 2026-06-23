//! Domain models shared across the CLI, TUI and HTTP service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lifecycle of a single download.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DownloadStatus {
    Queued,
    FetchingInfo,
    Downloading {
        progress: f32,
        speed: String,
        eta: String,
    },
    Merging,
    Done {
        path: String,
    },
    Failed {
        reason: String,
    },
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

/// A queued or in-flight download item.
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

/// Quality/format preference. This is domain logic (it maps to a yt-dlp format
/// string), so it lives in core and is shared by every front-end.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum FormatChoice {
    /// Best available quality (default).
    #[default]
    Best,
    /// 4K / 2160p.
    #[serde(rename = "4k")]
    #[cfg_attr(feature = "clap", value(name = "4k"))]
    FourK,
    /// 1080p Full HD.
    #[serde(rename = "1080p")]
    #[cfg_attr(feature = "clap", value(name = "1080p"))]
    Fhd,
    /// 720p HD.
    #[serde(rename = "720p")]
    #[cfg_attr(feature = "clap", value(name = "720p"))]
    Hd,
    /// 480p SD.
    #[serde(rename = "480p")]
    #[cfg_attr(feature = "clap", value(name = "480p"))]
    Sd,
    /// Audio only (mp3).
    Audio,
}

impl FormatChoice {
    /// Convert to a yt-dlp format string.
    pub fn to_ytdlp_format(&self) -> &'static str {
        match self {
            FormatChoice::Best => "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
            FormatChoice::FourK => {
                "bestvideo[height<=2160][ext=mp4]+bestaudio[ext=m4a]/best[height<=2160]"
            }
            FormatChoice::Fhd => {
                "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/best[height<=1080]"
            }
            FormatChoice::Hd => {
                "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/best[height<=720]"
            }
            FormatChoice::Sd => {
                "bestvideo[height<=480][ext=mp4]+bestaudio[ext=m4a]/best[height<=480]"
            }
            FormatChoice::Audio => "bestaudio[ext=m4a]/bestaudio",
        }
    }

    /// Whether this choice implies audio-only extraction.
    pub fn is_audio_only(&self) -> bool {
        matches!(self, FormatChoice::Audio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_default_is_best() {
        assert_eq!(FormatChoice::default(), FormatChoice::Best);
    }

    #[test]
    fn audio_choice_is_audio_only() {
        assert!(FormatChoice::Audio.is_audio_only());
        assert!(!FormatChoice::Best.is_audio_only());
    }

    #[test]
    fn status_progress_is_monotonic() {
        assert_eq!(DownloadStatus::Queued.progress_pct(), 0.0);
        let done = DownloadStatus::Done { path: "x".into() };
        assert_eq!(done.progress_pct(), 100.0);
    }

    #[test]
    fn format_roundtrips_through_json() {
        let json = serde_json::to_string(&FormatChoice::Fhd).unwrap();
        assert_eq!(json, "\"1080p\"");
        let back: FormatChoice = serde_json::from_str(&json).unwrap();
        assert_eq!(back, FormatChoice::Fhd);
    }
}
