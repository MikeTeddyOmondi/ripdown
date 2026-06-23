//! Shared application state for the Axum service.

use std::path::PathBuf;
use std::sync::Arc;

use ripdown_core::storage::StorageBackend;
use ripdown_core::SharedQueue;

/// Cloneable handle to all shared service state.
#[derive(Clone)]
pub struct AppState {
    /// Pluggable storage backend (local FS by default, S3/rustfs when configured).
    pub storage: Arc<dyn StorageBackend>,
    /// In-memory download queue (also surfaced to the web UI).
    pub queue: SharedQueue,
    /// Working directory where downloads are written before being stored.
    pub output_dir: PathBuf,
}
