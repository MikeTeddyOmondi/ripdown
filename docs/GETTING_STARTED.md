# Getting Started

Two ways to use ripdown: the **CLI/TUI** (`ripdown`) and the **HTTP service**
(`ripdown-http`). Both share the same core download engine.

If you haven't built yet, see [SETUP.md](SETUP.md).

---

## Part 1 — CLI & TUI

The `ripdown` binary has three subcommands: `download`, `tui`, and `info`.

### Interactive TUI (recommended)

```bash
ripdown tui          # or: ripdown ui
```

Keys:

| Key            | Action                           |
| -------------- | -------------------------------- |
| `A` / `N`      | Add a URL (paste a YouTube link) |
| `T`            | Toggle audio-only mode           |
| `↑ ↓` / `j k`  | Navigate the queue               |
| `Enter`        | Confirm URL (in the add modal)   |
| `Esc`          | Cancel the add modal             |
| `Q` / `Ctrl-C` | Quit                             |

### Download from the command line

```bash
# Best quality
ripdown download https://www.youtube.com/watch?v=TSVHoHyErBQ

# Short alias, 1080p, custom folder
ripdown dl -f 1080p -o ~/Videos https://youtu.be/TSVHoHyErBQ

# Audio only (mp3)
ripdown dl -a https://www.youtube.com/watch?v=TSVHoHyErBQ

# Several URLs, 5 parallel jobs
ripdown dl -j 5 https://youtu.be/aaa https://youtu.be/bbb
```

### Inspect metadata (no download)

```bash
ripdown info https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### Options

```
ripdown download --help

  -o, --output <DIR>      Output directory [env: RIPDOWN_OUTPUT_DIR]
  -f, --format <FORMAT>   best | 4k | 1080p | 720p | 480p | audio [default: best]
  -a, --audio-only        Extract audio as mp3
  -p, --playlist          Download full playlist
  -j, --jobs <N>          Parallel downloads [default: 3]
      --subs <LANG>       Embed subtitles (e.g. "en")
  -v, --verbose           Show raw yt-dlp output
```

### Where do files go?

Resolution order: `--output` flag → `$RIPDOWN_OUTPUT_DIR` → `~/Downloads/ripdown`.

---

## Part 2 — HTTP service

`ripdown-http` is a single self-contained binary that serves a REST API **and**
a bundled web UI (embedded into the binary — nothing extra to deploy).

### Start it

```bash
cargo run -p ripdown-http
# or the release binary:
./target/release/ripdown-http
```

Then open **http://localhost:8080**. Paste a YouTube URL, pick a format, and
watch the queue update live; finished files appear in the Files list.

### Configuration

All flags have an environment-variable equivalent.

| Variable             | Flag              | Default                | Description                                       |
| -------------------- | ----------------- | ---------------------- | ------------------------------------------------- |
| `PORT`               | `--port`          | `8080`                 | Listen port                                       |
| `RIPDOWN_OUTPUT_DIR` | `--output-dir`    | `~/Downloads/ripdown`¹ | Working/download directory                        |
| `RIPDOWN_STORAGE`    | `--storage`       | `local`                | `local` or `s3`                                   |
| `S3_ENDPOINT`        | `--s3-endpoint`   | —                      | S3/rustfs endpoint (e.g. `http://localhost:9000`) |
| `S3_REGION`          | `--s3-region`     | `us-east-1`            | S3 region                                         |
| `S3_BUCKET`          | `--s3-bucket`     | `ripdown`              | Target bucket                                     |
| `S3_ACCESS_KEY`      | `--s3-access-key` | —                      | Access key                                        |
| `S3_SECRET_KEY`      | `--s3-secret-key` | —                      | Secret key                                        |
| `S3_PATH_STYLE`      | `--s3-path-style` | `true`                 | Path-style addressing (required by rustfs/MinIO)  |

¹ The Docker image overrides this to `/data/downloads`.

```bash
# Custom port + local dir
ripdown-http --port 9000 --output-dir ./downloads

# S3 / rustfs backend
RIPDOWN_STORAGE=s3 \
S3_ENDPOINT=http://localhost:9000 \
S3_ACCESS_KEY=ripdown S3_SECRET_KEY=ripdown-secret \
ripdown-http
```

### API endpoints

| Method | Path               | Purpose                                       |
| ------ | ------------------ | --------------------------------------------- |
| `GET`  | `/`                | Bundled web UI                                |
| `GET`  | `/health`          | Health check (`ok`)                           |
| `POST` | `/api/download`    | Queue a download                              |
| `GET`  | `/api/queue`       | Queue snapshot (polled by the UI)             |
| `GET`  | `/api/files`       | List stored files                             |
| `GET`  | `/api/files/{key}` | Download a file (local stream or S3 redirect) |

### Queue a download via API

```bash
curl -s -X POST http://localhost:8080/api/download \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ", "format": "best", "audio_only": false}'
# -> {"id":"a1b2c3d4"}
```

`format` accepts `best | 4k | 1080p | 720p | 480p | audio`. The full set of
ready-to-run requests lives in [`.http`](../.http).

### Poll the queue

```bash
curl -s http://localhost:8080/api/queue
# -> {"items":[...],"queued":0,"active":1,"done":0,"failed":0}
```

Each item's `status` is one of `queued`, `fetching_info`, `downloading`,
`merging`, `done`, or `failed`.

---

## Next steps

- Run it in containers → [DOCKER.md](DOCKER.md)
- Ship it to a server → [DEPLOYMENT.md](DEPLOYMENT.md)
