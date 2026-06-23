# API Reference

REST API exposed by the `ripdown-http` service. Base URL defaults to
`http://localhost:8080`. All request/response bodies are JSON.

Ready-to-run examples for every endpoint live in [`.http`](../.http).

## Conventions

- Content type for bodies is `application/json`.
- A permissive CORS layer is enabled.
- Errors return a plain-text body with a `4xx`/`5xx` status code.

---

## `GET /health`

Liveness check.

**Response** `200 OK` — `text/plain`

```
ok
```

---

## `GET /`

Serves the bundled web UI (single-page app). Static assets are served under
`/assets/*`. Unmatched routes fall back to the SPA `index.html`.

---

## `POST /api/download`

Queue a new download. The work runs in the background; poll
[`GET /api/queue`](#get-apiqueue) for progress.

**Request body**

| Field        | Type    | Required | Default | Description                                              |
| ------------ | ------- | -------- | ------- | -------------------------------------------------------- |
| `url`        | string  | yes      | —       | YouTube video or playlist URL                            |
| `format`     | string  | no       | `best`  | One of `best`, `4k`, `1080p`, `720p`, `480p`, `audio`    |
| `audio_only` | boolean | no       | `false` | Force audio-only (mp3). Implied when `format` is `audio` |

```json
{
  "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
  "format": "1080p",
  "audio_only": false
}
```

**Response** `200 OK`

```json
{ "id": "a1b2c3d4" }
```

The `id` is the queue item's short identifier; use it to track the item in the
queue snapshot.

```bash
curl -X POST http://localhost:8080/api/download \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ", "format": "best", "audio_only": false}'
```

---

## `GET /api/queue`

Snapshot of the in-memory queue. The web UI polls this roughly once per second.

**Response** `200 OK`

```json
{
  "items": [
    {
      "id": "a1b2c3d4",
      "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
      "title": "Rick Astley - Never Gonna Give You Up",
      "platform": "YouTube",
      "thumbnail": null,
      "duration_secs": null,
      "format": "Best",
      "audio_only": false,
      "output_dir": "/data/downloads",
      "status": {
        "state": "downloading",
        "progress": 42.0,
        "speed": "…",
        "eta": "…"
      },
      "added_at": "2026-06-23T01:00:00Z"
    }
  ],
  "queued": 0,
  "active": 1,
  "done": 0,
  "failed": 0
}
```

### `DownloadItem` fields

| Field           | Type           | Description                                       |
| --------------- | -------------- | ------------------------------------------------- |
| `id`            | string         | Short item id                                     |
| `url`           | string         | Source URL                                        |
| `title`         | string \| null | Resolved video title (populated after info fetch) |
| `platform`      | string \| null | Detected platform (e.g. `YouTube`)                |
| `thumbnail`     | string \| null | Reserved (currently `null`)                       |
| `duration_secs` | number \| null | Reserved (currently `null`)                       |
| `format`        | string         | Requested format label                            |
| `audio_only`    | boolean        | Whether audio-only was requested                  |
| `output_dir`    | string         | Working directory for the download                |
| `status`        | object         | See [status object](#status-object)               |
| `added_at`      | string         | RFC 3339 timestamp                                |

### Counters

| Field    | Counts items whose status is              |
| -------- | ----------------------------------------- |
| `queued` | `queued`                                  |
| `active` | `fetching_info`, `downloading`, `merging` |
| `done`   | `done`                                    |
| `failed` | `failed`                                  |

### Status object

The `status` field is a tagged union keyed by `state`:

| `state`         | Extra fields                                          | Meaning                                   |
| --------------- | ----------------------------------------------------- | ----------------------------------------- |
| `queued`        | —                                                     | Waiting to start                          |
| `fetching_info` | —                                                     | Resolving metadata                        |
| `downloading`   | `progress` (number), `speed` (string), `eta` (string) | In progress                               |
| `merging`       | —                                                     | Muxing audio + video                      |
| `done`          | `path` (string)                                       | Finished; `path` is the stored object key |
| `failed`        | `reason` (string)                                     | Failed with an error message              |

```json
{ "state": "done", "path": "Never Gonna Give You Up [dQw4w9WgXcQ].mp4" }
```

---

## `GET /api/files`

List objects in the configured storage backend.

**Response** `200 OK`

```json
[{ "key": "Never Gonna Give You Up [dQw4w9WgXcQ].mp4", "size": 18475920 }]
```

| Field  | Type   | Description           |
| ------ | ------ | --------------------- |
| `key`  | string | Object key / filename |
| `size` | number | Size in bytes         |

---

## `GET /api/files/{key}`

Fetch a stored object.

- **Local storage** — streams the file directly with `Content-Type` (guessed from
  the extension) and a `Content-Disposition: attachment` header.
- **S3 / rustfs storage** — responds with `307 Temporary Redirect` to a
  presigned URL (default 1-hour expiry), so the client downloads straight from
  object storage.

Keys containing `..`, `/`, or `\` are rejected with `400 Bad Request`. Unknown
keys return `404 Not Found`.

```bash
curl -L http://localhost:8080/api/files/"Never Gonna Give You Up [dQw4w9WgXcQ].mp4" -o video.mp4
```

---

## Status codes

| Code                        | When                                    |
| --------------------------- | --------------------------------------- |
| `200 OK`                    | Successful request                      |
| `307 Temporary Redirect`    | `GET /api/files/{key}` on an S3 backend |
| `400 Bad Request`           | Invalid file key                        |
| `404 Not Found`             | File key not found                      |
| `500 Internal Server Error` | Storage listing/resolution failure      |

See [GETTING_STARTED.md](GETTING_STARTED.md#part-2--http-service) for service
configuration and [DEPLOYMENT.md](DEPLOYMENT.md) for production usage.
