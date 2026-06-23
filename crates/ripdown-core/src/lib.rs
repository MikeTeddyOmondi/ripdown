//! `ripdown-core` — the reusable heart of ripdown.
//!
//! Contains the domain models, the shared download queue, the presentation-free
//! download engine, and pluggable storage backends. Front-ends (`ripdown-cli`
//! and `ripdown-http`) depend on this crate and add their own I/O layer.

pub mod config;
pub mod engine;
pub mod models;
pub mod queue;
pub mod storage;

pub use engine::{download_url, download_with_progress, fetch_info, fetch_title, VideoMeta};
pub use models::{DownloadItem, DownloadStatus, FormatChoice};
pub use queue::{Queue, SharedQueue};
pub use storage::{from_config, StorageBackend, StorageConfig, StoredObject};
