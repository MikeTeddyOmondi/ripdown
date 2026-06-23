//! Local-filesystem storage backend (the default).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{StorageBackend, StoredObject};

/// Stores downloaded files in a local directory and serves them by relative key.
#[derive(Debug, Clone)]
pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("Failed to create storage root {}", root.display()))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn put(&self, key: &str, path: &Path) -> Result<()> {
        let dest = self.root.join(key);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        // If the source already lives under the root, this is a no-op move.
        if path == dest {
            return Ok(());
        }
        tokio::fs::copy(path, &dest)
            .await
            .with_context(|| format!("Failed to store {key} locally"))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<StoredObject>> {
        let mut out = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.root)
            .await
            .with_context(|| format!("Failed to read storage root {}", self.root.display()))?;
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_file() {
                out.push(StoredObject {
                    key: entry.file_name().to_string_lossy().into_owned(),
                    size: meta.len(),
                });
            }
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }

    async fn url_for(&self, key: &str) -> Result<String> {
        // The HTTP service streams local files itself; expose the API route.
        Ok(format!("/api/files/{key}"))
    }

    fn name(&self) -> &'static str {
        "local"
    }
}
