# Deployment

How ripdown is built, released, and run in production. The deployable artifact is
the **`ripdown-http`** service (the CLI is a local tool).

---

## CI/CD overview

Two GitHub Actions workflows drive everything:

| Workflow | Trigger | Does |
|---|---|---|
| [`ci.yml`](../.github/workflows/ci.yml) | push to `main`, PRs | `fmt --check`, `clippy -D warnings`, build, test |
| [`release.yml`](../.github/workflows/release.yml) | push tag `v*` | Build release binaries + publish GitHub Release; build & push Docker images |

### Required repository secrets

| Secret | Used by | Purpose |
|---|---|---|
| `DOCKER_USERNAME` | `release.yml` (docker job) | Docker Hub login |
| `DOCKER_TOKEN` | `release.yml` (docker job) | Docker Hub access token |
| `RELEASE_GPG_KEY` | `release.yml` (release job) | ASCII-armored GPG **private** key used to sign binaries |
| `RELEASE_GPG_PASSPHRASE` | `release.yml` (release job) | Passphrase for the GPG key |

`GITHUB_TOKEN` (built-in, no setup) is used to publish the GitHub Release.

> `ci.yml` needs **no secrets** — it only runs fmt/clippy/build/test.

### Generating the GPG signing key

```bash
# Generate a key (choose RSA 4096, set a passphrase)
gpg --full-generate-key

# Find the key id, then export the private key + note the passphrase
gpg --list-secret-keys --keyid-format=long
gpg --armor --export-secret-keys <KEY_ID> | pbcopy   # → paste into RELEASE_GPG_KEY

# Export the PUBLIC key for users to verify downloads
gpg --armor --export <KEY_ID> > ripdown-public.asc
```

Add `RELEASE_GPG_KEY` (the armored private key) and `RELEASE_GPG_PASSPHRASE` to
the repo's **Settings → Secrets and variables → Actions**.

The matching **public** key is committed to the repo as
[`ripdown-public.asc`](../ripdown-public.asc) (fingerprint
`FAC9A6D3 1A96C8ED 160EE72B A0C7B97B 7D1110F2`) so anyone can verify downloads.

### Verifying a release

Each release ships a per-binary detached signature (`<binary>.asc`) and a
`checksums.sha256`. To verify:

```bash
# 1. Import the public key (once)
gpg --import ripdown-public.asc

# 2. Verify the checksum
sha256sum -c checksums.sha256

# 3. Verify the signature for a binary
gpg --verify ripdown-linux-x64.asc ripdown-linux-x64
```

---

## Cutting a release

Releases are tag-driven. The current working state is frozen at `v0.1.0`.

```bash
# 1. Make sure CI is green on main
# 2. Tag and push
just tag v0.2.0
# equivalent to:
#   git tag -a v0.2.0 -m "Release v0.2.0"
#   git push origin v0.2.0
```

Pushing the tag triggers `release.yml`, which:

1. Builds release binaries (`ripdown`, `ripdown-http`) and attaches them to a
   GitHub Release with auto-generated notes.
2. Builds multi-arch (`linux/amd64`, `linux/arm64`) Docker images and pushes:
   - `locci/ripdown-cli:{version}` and `:latest`
   - `locci/ripdown-http:{version}` and `:latest`

Image tags are derived from the git tag via `docker/metadata-action`
(`type=semver`), so `v0.2.0` → image tag `0.2.0` + `latest`.

---

## Running in production

### Option A — Docker image (recommended)

```bash
docker run -d --name ripdown-http \
  --restart unless-stopped \
  -p 8080:8080 \
  -v ripdown-data:/data \
  -e RIPDOWN_STORAGE=s3 \
  -e S3_ENDPOINT=https://s3.example.com \
  -e S3_BUCKET=ripdown \
  -e S3_ACCESS_KEY=... \
  -e S3_SECRET_KEY=... \
  -e S3_PATH_STYLE=true \
  locci/ripdown-http:latest
```

See [DOCKER.md](DOCKER.md) for the full image reference and the `compose.yml`
stack (service + rustfs).

### Option B — Bare binary (systemd)

Build a release binary (`cargo build --release -p ripdown-http`) and run it under
a service manager:

```ini
# /etc/systemd/system/ripdown-http.service
[Unit]
Description=ripdown HTTP service
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/ripdown-http
Environment=PORT=8080
Environment=RIPDOWN_OUTPUT_DIR=/var/lib/ripdown/downloads
Environment=RIPDOWN_STORAGE=s3
Environment=S3_ENDPOINT=https://s3.example.com
Environment=S3_BUCKET=ripdown
Environment=S3_ACCESS_KEY=...
Environment=S3_SECRET_KEY=...
Environment=S3_PATH_STYLE=true
# Structured log level (see Logging below)
Environment=RUST_LOG=ripdown_http=info,tower_http=info
Restart=on-failure
StateDirectory=ripdown
DynamicUser=yes

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now ripdown-http
```

---

## Configuration reference

All settings are flags with environment fallbacks — see the table in
[GETTING_STARTED.md](GETTING_STARTED.md#configuration). The production-relevant
ones:

- `PORT` — listen port (default `8080`).
- `RIPDOWN_OUTPUT_DIR` — working dir for in-flight downloads (must be writable).
- `RIPDOWN_STORAGE` — `local` or `s3`.
- `S3_*` — endpoint, bucket, credentials, and `S3_PATH_STYLE=true` for
  rustfs/MinIO-style servers.

---

## Storage backends

The storage layer is a pluggable trait
([`StorageBackend`](../crates/ripdown-core/src/storage/mod.rs)):

- **`local`** — files stay on disk under `RIPDOWN_OUTPUT_DIR`; the service
  streams them directly from `/api/files/{key}`.
- **`s3`** — finished files are uploaded to the bucket; `/api/files/{key}`
  returns a **presigned redirect** (default 1-hour expiry), so clients fetch
  bytes straight from object storage.

Works with any S3-compatible server (rustfs, MinIO, AWS S3). Set `S3_PATH_STYLE`
appropriately for your provider.

---

## Operational notes

- **First-run bootstrap:** the `yt-dlp` crate downloads yt-dlp + ffmpeg into the
  data dir on first use. Ensure outbound network access and a persistent
  `/data` volume so this isn't repeated on every restart.
- **Health checks:** `GET /health` returns `ok` — wire it to your load balancer
  or orchestrator readiness/liveness probe.
- **Logging:** structured via `tracing`. Control verbosity with `RUST_LOG`
  (default `ripdown_http=info,tower_http=info`).
- **CORS:** the service enables a permissive CORS layer; front it with a reverse
  proxy (and TLS) if exposing publicly.
- **Resources:** transcoding/merging is CPU- and disk-intensive; size the host
  accordingly and give `/data` enough space for concurrent downloads.

---

## Pre-release checklist

```bash
just fmt-check    # formatting
just lint         # clippy -D warnings
just test         # unit tests
just build-release
```

These mirror what `ci.yml` enforces, so a green local run means a green tag.
