//! `GET /api/files` and `GET /api/files/{key}`.

use axum::{
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use ripdown_core::storage::StoredObject;

use crate::state::AppState;

/// List stored objects.
pub async fn list_files(State(state): State<AppState>) -> Response {
    match state.storage.list().await {
        Ok(objects) => Json::<Vec<StoredObject>>(objects).into_response(),
        Err(e) => {
            tracing::error!("list failed: {e:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to list files").into_response()
        }
    }
}

/// Serve a stored object.
///
/// For local storage we stream the file directly from the output directory; for
/// remote backends we redirect to the presigned URL.
pub async fn get_file(State(state): State<AppState>, AxumPath(key): AxumPath<String>) -> Response {
    // Guard against path traversal.
    if key.contains("..") || key.contains('/') || key.contains('\\') {
        return (StatusCode::BAD_REQUEST, "invalid key").into_response();
    }

    if state.storage.name() == "local" {
        let path = state.output_dir.join(&key);
        return match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let mime = mime_guess::from_path(&path).first_or_octet_stream();
                (
                    [
                        (header::CONTENT_TYPE, mime.to_string()),
                        (
                            header::CONTENT_DISPOSITION,
                            format!("attachment; filename=\"{key}\""),
                        ),
                    ],
                    bytes,
                )
                    .into_response()
            }
            Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
        };
    }

    match state.storage.url_for(&key).await {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(e) => {
            tracing::error!("url_for failed: {e:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to resolve file").into_response()
        }
    }
}
