//! `POST /api/download` and `GET /api/queue`.

use std::path::Path;

use axum::{extract::State, Json};
use ripdown_core::engine::{detect_platform, download_with_progress, fetch_title};
use ripdown_core::models::{DownloadItem, DownloadStatus, FormatChoice};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub url: String,
    #[serde(default)]
    pub format: FormatChoice,
    #[serde(default)]
    pub audio_only: bool,
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct QueueSnapshot {
    pub items: Vec<DownloadItem>,
    pub queued: usize,
    pub active: usize,
    pub done: usize,
    pub failed: usize,
}

/// Queue a download and run it in the background, updating the shared queue and
/// persisting the finished file through the configured storage backend.
pub async fn create_download(
    State(state): State<AppState>,
    Json(req): Json<DownloadRequest>,
) -> Json<DownloadResponse> {
    let audio_only = req.audio_only || req.format.is_audio_only();
    let item = DownloadItem::new(
        req.url.clone(),
        format!("{:?}", req.format),
        audio_only,
        state.output_dir.to_string_lossy().into_owned(),
    );
    let id = item.id.clone();
    let platform = detect_platform(&req.url).to_string();

    {
        let mut q = state.queue.write().await;
        q.push(item);
    }

    let queue = state.queue.clone();
    let storage = state.storage.clone();
    let output_dir = state.output_dir.clone();
    let url = req.url.clone();
    let task_id = id.clone();

    tokio::spawn(async move {
        // Best-effort title enrichment for the UI.
        if let Ok((title, _uploader)) = fetch_title(&url).await {
            let mut q = queue.write().await;
            q.update_title(&task_id, title, Some(platform.clone()));
        }

        let progress_queue = queue.clone();
        let progress_id = task_id.clone();
        let result = download_with_progress(&url, audio_only, &output_dir, move |status| {
            // Mirror lifecycle transitions into the shared queue. `try_write`
            // keeps the hot path non-blocking; the terminal Done/Failed states
            // are re-applied below to guarantee they land.
            if let Ok(mut q) = progress_queue.try_write() {
                q.update_status(&progress_id, status);
            }
        })
        .await;

        match result {
            Ok(path) => {
                let key = Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| task_id.clone());

                if let Err(e) = storage.put(&key, Path::new(&path)).await {
                    tracing::error!("storage put failed for {key}: {e:#}");
                    let mut q = queue.write().await;
                    q.update_status(
                        &task_id,
                        DownloadStatus::Failed {
                            reason: format!("stored locally but upload failed: {e:#}"),
                        },
                    );
                    return;
                }

                let mut q = queue.write().await;
                q.update_status(&task_id, DownloadStatus::Done { path: key });
            }
            Err(e) => {
                let mut q = queue.write().await;
                q.update_status(
                    &task_id,
                    DownloadStatus::Failed {
                        reason: format!("{e:#}"),
                    },
                );
            }
        }
    });

    Json(DownloadResponse { id })
}

/// Snapshot of the queue for the polling web UI.
pub async fn get_queue(State(state): State<AppState>) -> Json<QueueSnapshot> {
    let q = state.queue.read().await;
    Json(QueueSnapshot {
        items: q.items.clone(),
        queued: q.queued_count(),
        active: q.active_count(),
        done: q.done_count(),
        failed: q.failed_count(),
    })
}
