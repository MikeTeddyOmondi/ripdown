# Setup

Get a working ripdown development environment from a fresh clone.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `just` | latest | Optional task runner — `cargo install just` |
| Docker | 24+ | Optional, for the container workflow ([DOCKER.md](DOCKER.md)) |
| Internet access | — | First download fetches the yt-dlp + ffmpeg binaries (~50 MB), cached permanently |

> **No system `ffmpeg`/`yt-dlp` needed.** The `yt-dlp` crate downloads the
> correct binaries for your platform on first run and caches them under
> `<data_local>/ripdown/libs` (e.g. `~/Library/Application Support/ripdown/libs`
> on macOS, `~/.local/share/ripdown/libs` on Linux).

## Clone & build

```bash
git clone <repo>
cd ripdown
cargo build --workspace
```

This produces two binaries:

- `target/debug/ripdown` — the CLI + TUI
- `target/debug/ripdown-http` — the HTTP service

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
├── .http                      # ready-to-run API requests
└── .github/workflows/         # CI + Docker release
```

`ripdown-core` holds all presentation-free logic. The CLI/TUI and the HTTP
service are thin front-ends that depend on it.

## Feature flags (`ripdown-core`)

| Feature | Default | Effect |
|---|---|---|
| `clap` | off | Derives `clap::ValueEnum` on `FormatChoice` (enabled by `ripdown-cli`) |
| `s3` | off | Compiles the S3/rustfs storage backend (enabled by `ripdown-http`) |

## Common tasks

```bash
just            # list all recipes
just build      # cargo build --workspace
just lint       # cargo clippy --workspace --all-targets --all-features -- -D warnings
just fmt        # cargo fmt --all
just test       # cargo test --workspace
just check      # cargo check --workspace --all-features
```

Without `just`, run the underlying `cargo` commands directly.

## Editor / API testing

The repo ships an [`.http`](../.http) file with runnable `curl` requests for
every endpoint, grouped by `### <heading>`.

## Verify your setup

```bash
cargo test --workspace          # unit tests should pass
cargo run -p ripdown-cli -- --help
cargo run -p ripdown-http       # then open http://localhost:8080
```

Next: [GETTING_STARTED.md](GETTING_STARTED.md).
