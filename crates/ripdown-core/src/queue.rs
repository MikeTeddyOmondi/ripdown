//! In-memory, shared download queue used by both the TUI and the HTTP service.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::{DownloadItem, DownloadStatus};

pub type SharedQueue = Arc<RwLock<Queue>>;

#[derive(Debug, Default)]
pub struct Queue {
    pub items: Vec<DownloadItem>,
}

impl Queue {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn shared() -> SharedQueue {
        Arc::new(RwLock::new(Queue::new()))
    }

    pub fn push(&mut self, item: DownloadItem) {
        self.items.push(item);
    }

    pub fn update_status(&mut self, id: &str, status: DownloadStatus) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = status;
        }
    }

    pub fn update_title(&mut self, id: &str, title: String, platform: Option<String>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.title = Some(title);
            if let Some(p) = platform {
                item.platform = Some(p);
            }
        }
    }

    pub fn queued_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, DownloadStatus::Queued))
            .count()
    }

    pub fn active_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    DownloadStatus::FetchingInfo
                        | DownloadStatus::Downloading { .. }
                        | DownloadStatus::Merging
                )
            })
            .count()
    }

    pub fn done_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, DownloadStatus::Done { .. }))
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, DownloadStatus::Failed { .. }))
            .count()
    }
}
