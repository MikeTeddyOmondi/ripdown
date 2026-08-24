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

# Buildx populates TARGETARCH (amd64 / arm64) for multi-arch builds.
ARG TARGETARCH

# Pin the data dir so `data_local_dir()` is deterministic regardless of $HOME.
ENV XDG_DATA_HOME=/root/.local/share
ENV RIPDOWN_LIBS=/root/.local/share/ripdown/libs \
    # Binaries are baked in below; never re-fetch them at runtime.
    RIPDOWN_SKIP_LIB_UPDATE=1

# Pre-bake yt-dlp + ffmpeg so the CLI never has to call api.github.com at
# runtime (rate-limited from shared datacenter IPs). See http.Dockerfile.
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

COPY --from=builder /app/target/release/ripdown /usr/local/bin/ripdown

ENV RIPDOWN_OUTPUT_DIR=/data/downloads
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/ripdown"]
CMD ["--help"]
