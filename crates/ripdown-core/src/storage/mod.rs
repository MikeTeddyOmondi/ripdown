//! Pluggable storage backends for downloaded files.
//!
//! [`StorageBackend`] is intentionally minimal and object-safe so the HTTP
//! service can hold an `Arc<dyn StorageBackend>` and swap implementations
//! (local filesystem by default, S3/rustfs when configured) purely via config.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

mod local;
pub use local::LocalStorage;

#[cfg(feature = "s3")]
mod s3;
#[cfg(feature = "s3")]
pub use s3::S3Storage;

/// A stored object as listed by a backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    pub key: String,
    pub size: u64,
}

/// Where a downloaded file should be persisted.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Persist a local file under `key`.
    async fn put(&self, key: &str, path: &Path) -> Result<()>;

    /// List stored objects.
    async fn list(&self) -> Result<Vec<StoredObject>>;

    /// Resolve a client-facing URL (or local path) for `key`.
    async fn url_for(&self, key: &str) -> Result<String>;

    /// Human-readable backend name, for logging.
    fn name(&self) -> &'static str;
}

/// Storage selection resolved from configuration / environment.
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Persist into a local directory.
    Local { root: std::path::PathBuf },
    /// Persist into an S3-compatible bucket (rustfs / MinIO / AWS).
    #[cfg(feature = "s3")]
    S3 {
        endpoint: String,
        region: String,
        bucket: String,
        access_key: String,
        secret_key: String,
        /// Path-style addressing (required by rustfs / MinIO).
        path_style: bool,
    },
}

/// Build a boxed backend from configuration.
pub fn from_config(config: StorageConfig) -> Result<Arc<dyn StorageBackend>> {
    match config {
        StorageConfig::Local { root } => Ok(Arc::new(LocalStorage::new(root)?)),
        #[cfg(feature = "s3")]
        StorageConfig::S3 {
            endpoint,
            region,
            bucket,
            access_key,
            secret_key,
            path_style,
        } => Ok(Arc::new(S3Storage::new(
            endpoint, region, bucket, access_key, secret_key, path_style,
        )?)),
    }
}
