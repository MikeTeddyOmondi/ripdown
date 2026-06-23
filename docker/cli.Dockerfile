# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ripdown-core/Cargo.toml crates/ripdown-core/Cargo.toml
COPY crates/ripdown-cli/Cargo.toml crates/ripdown-cli/Cargo.toml
COPY crates/ripdown-http/Cargo.toml crates/ripdown-http/Cargo.toml
COPY crates ./crates

RUN cargo build --release -p ripdown-cli

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ripdown /usr/local/bin/ripdown

ENV RIPDOWN_OUTPUT_DIR=/data/downloads
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/ripdown"]
CMD ["--help"]
