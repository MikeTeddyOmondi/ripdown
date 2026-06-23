# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1-bookworm AS builder

WORKDIR /app

# Cache dependencies first.
COPY Cargo.toml Cargo.lock ./
COPY crates/ripdown-core/Cargo.toml crates/ripdown-core/Cargo.toml
COPY crates/ripdown-cli/Cargo.toml crates/ripdown-cli/Cargo.toml
COPY crates/ripdown-http/Cargo.toml crates/ripdown-http/Cargo.toml

# Now the sources (web/ is embedded into the binary at compile time).
COPY crates ./crates

RUN cargo build --release -p ripdown-http

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

# ca-certificates for TLS to YouTube / S3. The yt-dlp crate fetches the yt-dlp +
# ffmpeg binaries itself on first run, so no system ffmpeg is required.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ripdown-http /usr/local/bin/ripdown-http

ENV RIPDOWN_OUTPUT_DIR=/data/downloads \
    PORT=8080
VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/ripdown-http"]
