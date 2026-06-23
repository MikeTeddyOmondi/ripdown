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

# Buildx populates TARGETARCH (amd64 / arm64) for multi-arch builds.
ARG TARGETARCH

# Pin the data dir so `data_local_dir()` is deterministic regardless of $HOME.
ENV XDG_DATA_HOME=/root/.local/share
ENV RIPDOWN_LIBS=/root/.local/share/ripdown/libs

# Pre-bake yt-dlp + ffmpeg into the image. This avoids the runtime call to
# api.github.com that the yt-dlp crate makes to resolve the latest release —
# that endpoint is rate-limited (HTTP 403) from shared datacenter IPs like
# Render's. ffmpeg comes from apt; yt-dlp is pulled from the release CDN
# (github.com/.../releases/latest/download, which is NOT the rate-limited API).
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl ffmpeg \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p "$RIPDOWN_LIBS" \
    && case "$TARGETARCH" in \
         amd64) YTDLP_ASSET=yt-dlp_linux ;; \
         arm64) YTDLP_ASSET=yt-dlp_linux_aarch64 ;; \
         *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
       esac \
    && curl -fsSL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/${YTDLP_ASSET}" \
         -o "$RIPDOWN_LIBS/yt-dlp" \
    && chmod +x "$RIPDOWN_LIBS/yt-dlp" \
    && ln -sf /usr/bin/ffmpeg  "$RIPDOWN_LIBS/ffmpeg" \
    && ln -sf /usr/bin/ffprobe "$RIPDOWN_LIBS/ffprobe"

COPY --from=builder /app/target/release/ripdown-http /usr/local/bin/ripdown-http

ENV RIPDOWN_OUTPUT_DIR=/data/downloads \
    PORT=8080
VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/ripdown-http"]
