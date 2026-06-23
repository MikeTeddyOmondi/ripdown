mod routes;
mod state;
mod web;

use std::path::PathBuf;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use ripdown_core::queue::Queue;
use ripdown_core::storage::{self, StorageConfig};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::AppState;

/// ripdown HTTP service — bundled web UI + REST API for downloading from YouTube.
#[derive(Parser, Debug)]
#[command(name = "ripdown-http", version, about)]
struct Cli {
    /// Port to listen on.
    #[arg(long, env = "PORT", default_value = "8080")]
    port: u16,

    /// Directory downloads are written to before being stored.
    ///
    /// Defaults to `$RIPDOWN_OUTPUT_DIR`, falling back to `~/Downloads/ripdown`
    /// for local runs (the Docker image sets this to `/data/downloads`).
    #[arg(long, env = "RIPDOWN_OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Storage backend: `local` or `s3`.
    #[arg(long, env = "RIPDOWN_STORAGE", default_value = "local")]
    storage: String,

    // ── S3 / rustfs options (used when --storage=s3) ──
    #[arg(long, env = "S3_ENDPOINT", default_value = "")]
    s3_endpoint: String,
    #[arg(long, env = "S3_REGION", default_value = "us-east-1")]
    s3_region: String,
    #[arg(long, env = "S3_BUCKET", default_value = "ripdown")]
    s3_bucket: String,
    #[arg(long, env = "S3_ACCESS_KEY", default_value = "")]
    s3_access_key: String,
    #[arg(long, env = "S3_SECRET_KEY", default_value = "")]
    s3_secret_key: String,
    /// Use path-style addressing (required by rustfs / MinIO).
    #[arg(long, env = "S3_PATH_STYLE", default_value = "true")]
    s3_path_style: bool,
}

impl Cli {
    fn storage_config(&self, output_dir: &std::path::Path) -> Result<StorageConfig> {
        match self.storage.as_str() {
            "local" => Ok(StorageConfig::Local {
                root: output_dir.to_path_buf(),
            }),
            "s3" => Ok(StorageConfig::S3 {
                endpoint: self.s3_endpoint.clone(),
                region: self.s3_region.clone(),
                bucket: self.s3_bucket.clone(),
                access_key: self.s3_access_key.clone(),
                secret_key: self.s3_secret_key.clone(),
                path_style: self.s3_path_style,
            }),
            other => anyhow::bail!("unknown storage backend: {other} (expected `local` or `s3`)"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ripdown_http=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    // Resolve the output dir: flag/env → ~/Downloads/ripdown for local runs.
    let output_dir = ripdown_core::config::resolve_output_dir(cli.output_dir.clone());

    std::fs::create_dir_all(&output_dir).with_context(|| {
        format!("failed to create output dir {}", output_dir.display())
    })?;
    let storage = storage::from_config(cli.storage_config(&output_dir)?)?;
    tracing::info!("storage backend: {}", storage.name());
    tracing::info!("output dir: {}", output_dir.display());

    let state = AppState {
        storage,
        queue: Queue::shared(),
        output_dir,
    };

    let app = Router::new()
        .route("/", get(web::index))
        .route("/health", get(health))
        .route("/api/download", post(routes::download::create_download))
        .route("/api/queue", get(routes::download::get_queue))
        .route("/api/files", get(routes::files::list_files))
        .route("/api/files/{key}", get(routes::files::get_file))
        .route("/assets/{*path}", get(web::static_handler))
        .fallback(web::fallback)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("ripdown-http listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
