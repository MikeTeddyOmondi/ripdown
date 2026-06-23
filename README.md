```
  ██████╗ ██╗██████╗ ██████╗  ██████╗ ██╗    ██╗███╗   ██╗
  ██╔══██╗██║██╔══██╗██╔══██╗██╔═══██╗██║    ██║████╗  ██║
  ██████╔╝██║██████╔╝██║  ██║██║   ██║██║ █╗ ██║██╔██╗ ██║
  ██╔══██╗██║██╔═══╝ ██║  ██║██║   ██║██║███╗██║██║╚██╗██║
  ██║  ██║██║██║     ██████╔╝╚██████╔╝╚███╔███╔╝██║ ╚████║
  ╚═╝  ╚═╝╚═╝╚═╝     ╚═════╝  ╚═════╝  ╚══╝╚══╝ ╚═╝  ╚═══╝
```

# ⚡ ripdown

A blazing-fast video downloader for **YouTube** — built in Rust.

Ships three ways to use it:

- **CLI** (`ripdown`) — classic one-liner with progress bars
- **TUI** (`ripdown tui`) — interactive terminal UI with a live download queue
- **HTTP service** (`ripdown-http`) — a tiny Axum server with a bundled web UI
  that stores downloads to local disk or any S3-compatible storage (rustfs / MinIO / AWS)

---

## Features

| Feature | Detail |
|---|---|
| YouTube | Download videos & playlists from YouTube |
| Auto-installs deps | yt-dlp + ffmpeg downloaded automatically on first run |
| TUI | Ratatui-powered interactive queue with live status |
| Web UI | Self-contained, single-binary web app (matches the TUI look) |
| Pluggable storage | Local filesystem (default) or S3/rustfs — swappable via a trait |
| Format picker | best / 4k / 1080p / 720p / 480p / audio-only |
| Parallel downloads | Configurable concurrency (`-j N`) |
| Metadata inspect | `ripdown info <url>` |

> Other platforms may come later; today ripdown targets YouTube.

---

## Workspace layout

```
ripdown/
├── Cargo.toml                 # workspace + shared dependencies
├── crates/
│   ├── ripdown-core/          # library: engine, models, queue, storage, config
│   ├── ripdown-cli/           # binary `ripdown` — CLI + TUI
│   └── ripdown-http/          # binary `ripdown-http` — Axum API + bundled web UI
├── docker/                    # Dockerfiles for both binaries
├── compose.yml                # ripdown-http + rustfs (S3) stack
├── .justfile                  # task runner
└── .github/workflows/         # CI + Docker release
```

`ripdown-core` holds all presentation-free logic. The CLI/TUI and the HTTP
service are thin front-ends that depend on it — see
[`StorageBackend`](crates/ripdown-core/src/storage/mod.rs) for the pluggable
storage trait.

### Crates

| Crate | Kind | Description |
|---|---|---|
| [`ripdown-core`](crates/ripdown-core) | library | Presentation-free heart: download [engine](crates/ripdown-core/src/engine.rs), domain [models](crates/ripdown-core/src/models.rs), shared [queue](crates/ripdown-core/src/queue.rs), pluggable [storage](crates/ripdown-core/src/storage/mod.rs) (local + S3), and [config](crates/ripdown-core/src/config.rs). Feature flags: `clap`, `s3`. |
| [`ripdown-cli`](crates/ripdown-cli) | binary `ripdown` | CLI (`download` / `info`) with progress bars + the interactive Ratatui [TUI](crates/ripdown-cli/src/tui.rs). |
| [`ripdown-http`](crates/ripdown-http) | binary `ripdown-http` | Axum REST API + a vanilla [web UI](crates/ripdown-http/web) embedded into the binary via `rust-embed`. |

---

## Documentation

Full guides live in [`docs/`](docs/):

| Doc | What it covers |
|---|---|
| [SETUP.md](docs/SETUP.md) | Prerequisites, building from source, workspace layout, `just` tasks |
| [GETTING_STARTED.md](docs/GETTING_STARTED.md) | Using the CLI/TUI and the HTTP service (config, endpoints, examples) |
| [CLI.md](docs/CLI.md) | Full `ripdown` CLI/TUI reference — commands, flags, keybindings |
| [API_REFERENCE.md](docs/API_REFERENCE.md) | `ripdown-http` REST API — endpoints, request/response schemas |
| [DOCKER.md](docs/DOCKER.md) | Building/running both images and the `compose.yml` + rustfs stack |
| [DEPLOYMENT.md](docs/DEPLOYMENT.md) | CI/CD, tag-driven releases, production run (Docker + systemd), storage backends |

---

## Installation

### Prerequisites
- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- [`just`](https://github.com/casey/just) (optional, for task recipes)
- Internet access (first run downloads yt-dlp + ffmpeg ~50 MB, cached permanently)

### Build
```bash
git clone <repo>
cd ripdown
cargo build --release
# Binaries: ./target/release/ripdown  and  ./target/release/ripdown-http
```

### Install the CLI globally
```bash
cargo install --path crates/ripdown-cli
```

---

## CLI usage

### 🖥️ TUI (recommended)
```bash
ripdown tui   # or: ripdown ui
```

Inside the TUI:
- **A** — add a URL (paste any YouTube link)
- **T** — toggle audio-only mode
- **↑ ↓ / j k** — navigate queue
- **Q** — quit

### 📥 Download
```bash
# Best quality
ripdown download https://www.youtube.com/watch?v=TSVHoHyErBQ

# 1080p to a specific folder
ripdown dl -f 1080p -o ~/Videos https://youtu.be/...

# Audio only (mp3)
ripdown dl -a https://www.youtube.com/watch?v=...

# Multiple URLs, 5 parallel jobs
ripdown dl -j 5 https://youtu.be/... https://youtu.be/...
```

### 🔍 Inspect metadata
```bash
ripdown info https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### All options
```
ripdown download --help

Options:
  -o, --output <DIR>      Output directory [env: RIPDOWN_OUTPUT_DIR]
  -f, --format <FORMAT>   best | 4k | 1080p | 720p | 480p | audio [default: best]
  -a, --audio-only        Extract audio as mp3
  -p, --playlist          Download full playlist
  -j, --jobs <N>          Parallel downloads [default: 3]
      --subs <LANG>       Embed subtitles (e.g. "en")
  -v, --verbose           Show raw yt-dlp output
```

---

## HTTP service

A single self-contained binary that serves a REST API **and** the bundled web UI
(embedded into the binary with `rust-embed` — no separate static files to deploy).

```bash
just run-http             # http://localhost:8080
# or
cargo run -p ripdown-http
```

### Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET`  | `/` | Bundled web UI |
| `GET`  | `/health` | Health check |
| `POST` | `/api/download` | Queue a download `{ url, format, audio_only }` |
| `GET`  | `/api/queue` | Queue snapshot (polled by the UI) |
| `GET`  | `/api/files` | List stored files |
| `GET`  | `/api/files/{key}` | Download a file (local stream or S3 redirect) |

### Configuration (env vars / flags)

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | Listen port |
| `RIPDOWN_OUTPUT_DIR` | `/data/downloads` | Working/download directory |
| `RIPDOWN_STORAGE` | `local` | `local` or `s3` |
| `S3_ENDPOINT` | — | S3/rustfs endpoint (e.g. `http://rustfs:9000`) |
| `S3_REGION` | `us-east-1` | S3 region |
| `S3_BUCKET` | `ripdown` | Target bucket |
| `S3_ACCESS_KEY` / `S3_SECRET_KEY` | — | Credentials |
| `S3_PATH_STYLE` | `true` | Path-style addressing (required by rustfs/MinIO) |

---

## Docker

Images are published to Docker Hub on each release:
`locci/ripdown-cli` and `locci/ripdown-http`.

```bash
# Bring up the HTTP service + rustfs (S3) storage
just docker-up        # or: docker compose up --build

# Build images locally
just docker-build-http
just docker-build-cli
```

> **Note:** the `yt-dlp` crate downloads the yt-dlp + ffmpeg binaries on first
> run into a cache directory. The container needs network access and a writable
> volume (mounted at `/data`) for this one-time bootstrap.

---

## Task runner (`just`)

```bash
just                  # list recipes
just build            # build the workspace (debug)
just build-release    # build the workspace (release)
just run-tui          # run the TUI
just run-http         # run the HTTP service
just lint             # cargo clippy -D warnings
just fmt              # cargo fmt
just test             # cargo test
just docker-up        # docker compose up
```

---

## Crate stack

| Crate | Purpose |
|---|---|
| `clap` 4 | CLI parsing (derive) |
| `yt-dlp` 2.7 | Async yt-dlp + ffmpeg wrapper (auto-downloads binaries) |
| `ratatui` 0.30 | Terminal UI widgets |
| `crossterm` 0.29 | Cross-platform terminal control |
| `axum` 0.8 | HTTP service |
| `rust-s3` 0.35 | S3-compatible storage client |
| `rust-embed` 8 | Bundle the web UI into the binary |
| `tokio` 1 | Async runtime |
| `tracing` | Structured logging |

### Why `yt-dlp`?
The `yt-dlp` crate wraps the actively-maintained Python **yt-dlp** binary and
auto-downloads the correct yt-dlp + ffmpeg builds for your platform — zero manual
setup, and resilient to YouTube changes.

---

## Roadmap

- [ ] Persistent queue (serde_json history file)
- [ ] Download history / library browser
- [ ] Live progress percentage from yt-dlp hooks
- [ ] Additional platforms beyond YouTube
```
