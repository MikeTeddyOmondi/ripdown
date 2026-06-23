//! Serves the bundled vanilla web app from the binary (rust-embed).

use axum::{
    extract::Path,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/"]
struct Assets;

/// Serve `index.html` for the app root.
pub async fn index() -> Response {
    serve("index.html")
}

/// Serve a named static asset, falling back to `index.html` (SPA behaviour).
pub async fn static_handler(Path(path): Path<String>) -> Response {
    if Assets::get(&path).is_some() {
        serve(&path)
    } else {
        serve("index.html")
    }
}

/// Fallback for unmatched routes.
pub async fn fallback(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if Assets::get(path).is_some() {
        serve(path)
    } else {
        serve("index.html")
    }
}

fn serve(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}
