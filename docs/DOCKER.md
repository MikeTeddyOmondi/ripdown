# Docker

ripdown ships two images, built from [`docker/`](../docker/):

| Image | Dockerfile | Entry |
|---|---|---|
| `locci/ripdown-http` | `docker/http.Dockerfile` | The HTTP service + bundled web UI |
| `locci/ripdown-cli`  | `docker/cli.Dockerfile`  | The `ripdown` CLI |

Both are multi-stage (`rust:1-bookworm` builder → `debian:bookworm-slim`
runtime). The runtime only needs `ca-certificates`; the `yt-dlp` crate fetches
yt-dlp + ffmpeg itself at runtime.

> **Runtime requirements:** the container needs **outbound network access** (to
> download the yt-dlp/ffmpeg binaries on first run and to reach YouTube) and a
> **writable volume at `/data`** so those binaries and downloads persist across
> restarts. The image sets `RIPDOWN_OUTPUT_DIR=/data/downloads`.

---

## Quick start — full stack with rustfs

[`compose.yml`](../compose.yml) brings up the HTTP service backed by
**rustfs** (S3-compatible storage):

```bash
docker compose up --build
# or:
just docker-up
```

Open **http://localhost:8080**. Downloads are stored in the `ripdown` bucket on
rustfs (S3 backend). Stop with `docker compose down` (`just docker-down`).

The compose file wires the service to rustfs:

```yaml
RIPDOWN_STORAGE: s3
S3_ENDPOINT: http://rustfs:9000
S3_BUCKET: ripdown
S3_ACCESS_KEY: ripdown
S3_SECRET_KEY: ripdown-secret
S3_PATH_STYLE: "true"
```

To keep files on a local volume instead of S3, set `RIPDOWN_STORAGE: local`
(downloads then live under the `ripdown-data` volume at `/data/downloads`).

---

## Build images manually

```bash
# HTTP service
docker build -f docker/http.Dockerfile -t locci/ripdown-http:latest .
just docker-build-http            # equivalent

# CLI
docker build -f docker/cli.Dockerfile -t locci/ripdown-cli:latest .
just docker-build-cli             # equivalent
```

---

## Run the HTTP image standalone

### Local storage (files on a host volume)

```bash
docker run --rm -p 8080:8080 \
  -v ripdown-data:/data \
  locci/ripdown-http:latest
```

### S3 / rustfs storage

```bash
docker run --rm -p 8080:8080 \
  -v ripdown-data:/data \
  -e RIPDOWN_STORAGE=s3 \
  -e S3_ENDPOINT=http://my-rustfs:9000 \
  -e S3_BUCKET=ripdown \
  -e S3_ACCESS_KEY=... \
  -e S3_SECRET_KEY=... \
  -e S3_PATH_STYLE=true \
  locci/ripdown-http:latest
```

---

## Run the CLI image

The CLI entrypoint is `ripdown`; pass subcommands as arguments. Mount a host
directory to keep your downloads.

```bash
# Show help (default CMD)
docker run --rm locci/ripdown-cli:latest

# Download into ./out on the host
docker run --rm \
  -v "$PWD/out:/data/downloads" \
  locci/ripdown-cli:latest \
  download https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

> The interactive TUI (`ripdown tui`) needs a TTY: add `-it` if you want to try
> it in a container, though native use is recommended for the TUI.

---

## Images on Docker Hub

Releases publish multi-arch (`linux/amd64`, `linux/arm64`) images tagged with the
semver version and `latest`:

```bash
docker pull locci/ripdown-http:latest
docker pull locci/ripdown-cli:latest
```

See [DEPLOYMENT.md](DEPLOYMENT.md) for how the release pipeline builds and pushes
these, and how to run the service in production.
