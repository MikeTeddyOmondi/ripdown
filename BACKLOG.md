# Backlog

Planned and proposed work, grouped the same way as the [CHANGELOG](CHANGELOG.md).
Items here are not yet implemented; they graduate to the changelog when shipped.

## [Unreleased / Planned]

### Added
- Live download progress from yt-dlp hooks (real `progress`/`speed`/`eta` in the
  queue instead of the current coarse phases).
- Persistent queue + download history (serde_json store) surfaced in the TUI and
  web UI.
- Thumbnail previews (web UI cards; sixel/kitty in the TUI).
- Playlist expansion into individual queue items.
- Subtitle embedding wired through the engine (`--subs`).
- Authentication / API keys for the HTTP service before public exposure.
- Health/readiness split and Prometheus-style metrics endpoint.

### Changed
- Stream large file downloads from local storage instead of buffering them fully
  in memory in `GET /api/files/{key}`.
- Make storage backend swappable at runtime via a richer config object (beyond
  env vars), keeping the `StorageBackend` trait generic.
- Additional platforms beyond YouTube (kept out of the engine's advertised scope
  until validated).

### Fixed
- Surface per-item yt-dlp error detail in the web UI (currently only the TUI
  renders the full failure reason).
