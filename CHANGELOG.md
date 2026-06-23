# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-23

Restructured the single-binary app into a Cargo workspace and added an HTTP
service with a bundled web UI.

### Added
- `ripdown-core` library crate: presentation-free download engine, domain models,
  shared queue, pluggable storage (`StorageBackend` trait), and config helpers.
- `ripdown-http` binary: Axum REST API (`/api/download`, `/api/queue`,
  `/api/files`) with a vanilla web UI embedded into the binary via `rust-embed`.
- Pluggable storage backends: local filesystem (default) and S3/rustfs (rust-s3),
  selectable via configuration.
- Docker images for both binaries (`docker/cli.Dockerfile`, `docker/http.Dockerfile`)
  and a `compose.yml` stack wiring `ripdown-http` to a rustfs (S3) service.
- GitHub Actions: `ci.yml` (fmt/clippy/build/test) and `release.yml` (multi-platform
  signed binaries with SHA256 + GPG, plus Docker Hub image push).
- `.justfile` task runner and `.http` request collection.
- Documentation under `docs/`: SETUP, GETTING_STARTED, CLI, API_REFERENCE,
  DOCKER, DEPLOYMENT.
- Unit tests for `FormatChoice`, `DownloadStatus`, filename sanitization, and
  platform detection.

### Changed
- Split the original single binary into `ripdown-cli` (CLI + TUI) and the new
  `ripdown-http`, both depending on `ripdown-core`.
- Moved `FormatChoice` into `ripdown-core` (now `serde`-serializable; `clap`
  derive is feature-gated).
- `ripdown-http` output directory now resolves from `--output-dir`/
  `$RIPDOWN_OUTPUT_DIR`, falling back to `~/Downloads/ripdown` for local runs.
- Scoped the project's advertised capability to **YouTube only** across the CLI
  help text, `Cargo.toml`, and README.

### Fixed
- HTTP service no longer crashes on startup when the default output directory is
  not writable (previously hard-coded to `/data/downloads`).

## [0.1.0] - 2026-06-22

### Added
- Initial working release: terminal video downloader for YouTube with a CLI
  (progress bars) and an interactive Ratatui TUI, powered by the `yt-dlp` crate.
