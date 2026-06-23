# ripdown task runner — run `just` to list recipes.

set shell := ["bash", "-uc"]

# Image coordinates for Docker Hub releases.
cli_image := "locci/ripdown-cli"
http_image := "locci/ripdown-http"

# Default: list available recipes.
default:
    @just --list

# ── Build ─────────────────────────────────────────────────────────────────────

# Build the whole workspace (debug).
build:
    cargo build --workspace

# Build the whole workspace (release).
build-release:
    cargo build --workspace --release

# Build just the CLI binary.
build-cli:
    cargo build -p ripdown-cli --release

# Build just the HTTP service binary.
build-http:
    cargo build -p ripdown-http --release

# ── Run ───────────────────────────────────────────────────────────────────────

# Run the CLI (pass args after `--`, e.g. `just run-cli download <url>`).
run-cli *ARGS:
    cargo run -p ripdown-cli -- {{ARGS}}

# Launch the interactive TUI.
run-tui:
    cargo run -p ripdown-cli -- tui

# Run the HTTP service (local storage on :8080).
run-http:
    cargo run -p ripdown-http

# ── Quality ───────────────────────────────────────────────────────────────────

# Format all code.
fmt:
    cargo fmt --all

# Check formatting (CI).
fmt-check:
    cargo fmt --all --check

# Lint with clippy, denying warnings.
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Type-check without producing binaries.
check:
    cargo check --workspace --all-features

# Run tests.
test:
    cargo test --workspace

# ── Install ───────────────────────────────────────────────────────────────────

# Install the `ripdown` CLI into ~/.cargo/bin.
install:
    cargo install --path crates/ripdown-cli

# ── Docker ────────────────────────────────────────────────────────────────────

# Build the CLI Docker image.
docker-build-cli tag="latest":
    docker build -f docker/cli.Dockerfile -t {{cli_image}}:{{tag}} .

# Build the HTTP Docker image.
docker-build-http tag="latest":
    docker build -f docker/http.Dockerfile -t {{http_image}}:{{tag}} .

# Bring up the HTTP service + rustfs storage stack.
docker-up:
    docker compose up --build

# Tear the stack down.
docker-down:
    docker compose down

# ── Release ───────────────────────────────────────────────────────────────────

# Tag and push a release (e.g. `just tag v0.2.0`).
tag VERSION:
    git tag -a {{VERSION}} -m "Release {{VERSION}}"
    git push origin {{VERSION}}
