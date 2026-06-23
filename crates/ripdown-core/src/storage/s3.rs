//! S3-compatible storage backend (rustfs / MinIO / AWS) built on `rust-s3`.
//!
//! The backend is deliberately thin and only touches the small surface of
//! `rust-s3` we need, so it can be swapped for another client without changing
//! the [`StorageBackend`] contract.

use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use s3::{creds::Credentials, Bucket, Region};

use super::{StorageBackend, StoredObject};

/// Seconds a presigned GET URL stays valid.
const PRESIGN_EXPIRY_SECS: u32 = 3600;

pub struct S3Storage {
    bucket: Box<Bucket>,
}

impl S3Storage {
    pub fn new(
        endpoint: String,
        region: String,
        bucket: String,
        access_key: String,
        secret_key: String,
        path_style: bool,
    ) -> Result<Self> {
        let credentials = Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
            .context("Invalid S3 credentials")?;

        let region = Region::Custom { region, endpoint };

        let mut bucket =
            Bucket::new(&bucket, region, credentials).context("Failed to open S3 bucket")?;

        if path_style {
            bucket.set_path_style();
        }

        Ok(Self { bucket })
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn put(&self, key: &str, path: &Path) -> Result<()> {
        let bytes = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read {} for upload", path.display()))?;
        self.bucket
            .put_object(format!("/{key}"), &bytes)
            .await
            .with_context(|| format!("Failed to upload {key} to S3"))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<StoredObject>> {
        let results = self
            .bucket
            .list(String::new(), None)
            .await
            .context("Failed to list S3 objects")?;

        let mut out = Vec::new();
        for page in results {
            for obj in page.contents {
                out.push(StoredObject {
                    key: obj.key,
                    size: obj.size,
                });
            }
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }

    async fn url_for(&self, key: &str) -> Result<String> {
        self.bucket
            .presign_get(format!("/{key}"), PRESIGN_EXPIRY_SECS, None)
            .await
            .context("Failed to presign S3 URL")
    }

    fn name(&self) -> &'static str {
        "s3"
    }
}
