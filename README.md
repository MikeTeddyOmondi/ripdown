# ⚡ ripdown

A blazing-fast terminal video downloader for **YouTube** — built in Rust.

Ships two modes:
- **CLI mode** — classic one-liner with progress bars
- **TUI mode** — interactive terminal UI with a live download queue

---

## Features

| Feature | Detail |
|---|---|
| 1800+ sites | Backed by yt-dlp — YouTube, X, IG, TikTok, Twitch, Vimeo, … |
| Auto-installs deps | yt-dlp + ffmpeg downloaded automatically on first run |
| TUI | Ratatui-powered interactive queue with live status |
| Format picker | best / 4k / 1080p / 720p / 480p / audio-only |
| Parallel downloads | Configurable concurrency (`-j N`) |
| Playlist support | `--playlist` flag |
| Subtitle embed | `--subs en` (or any language code) |
| Metadata inspect | `ripdown info <url>` |

---

## Installation

### Prerequisites
- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Internet access (first run downloads yt-dlp + ffmpeg ~50 MB, cached permanently)

### Build
```bash
git clone <repo>
cd ripdown
cargo build --release
# Binary: ./target/release/ripdown
```

### Install globally
```bash
cargo install --path .
```

---

## Usage

### 🖥️ TUI (recommended)
```bash
ripdown tui
# or alias:
ripdown ui
```

Inside the TUI:
- **A** — add a URL (paste any supported link)
- **T** — toggle audio-only mode
- **↑ ↓ / j k** — navigate queue
- **Q** — quit

### 📥 CLI download
```bash
# Download best quality
ripdown download https://www.youtube.com/watch?v=TSVHoHyErBQ

# 1080p to a specific folder
ripdown dl -f 1080p -o ~/Videos https://youtu.be/...

# Audio only (mp3)
ripdown dl -a https://www.youtube.com/watch?v=...

# Multiple URLs, 5 parallel jobs
ripdown dl -j 5 https://youtu.be/... https://x.com/...

# Entire playlist
ripdown dl --playlist https://www.youtube.com/playlist?list=...
```

### 🔍 Inspect metadata
```bash
ripdown info https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### All options
```
ripdown download --help

Options:
  -o, --output <DIR>      Output directory [env: RIPDOWN_OUTPUT_DIR]
  -f, --format <FORMAT>   best | 4k | 1080p | 720p | 480p | audio [default: best]
  -a, --audio-only        Extract audio as mp3
  -p, --playlist          Download full playlist
  -j, --jobs <N>          Parallel downloads [default: 3]
      --subs <LANG>       Embed subtitles (e.g. "en")
  -v, --verbose           Show raw yt-dlp output
```

---

## Project Structure

```
ripdown/
├── src/
│   ├── main.rs        # Entry point, Clap command routing
│   ├── cli.rs         # All Clap CLI definitions (derive API)
│   ├── types.rs       # DownloadItem, DownloadStatus
│   ├── queue.rs       # Shared async queue (Arc<RwLock<Queue>>)
│   ├── downloader.rs  # yt-dlp wrapper, progress, info fetch
│   └── tui.rs         # Full Ratatui TUI — layout, widgets, input
└── Cargo.toml
```

---

## Crate Stack

| Crate | Version | Purpose |
|---|---|---|
| `clap` | 4 | CLI parsing with derive macros |
| `yt-dlp` | 0.4 | Async yt-dlp + ffmpeg wrapper (auto-downloads binaries) |
| `ratatui` | 0.29 | Terminal UI widgets |
| `crossterm` | 0.28 | Cross-platform terminal control |
| `tokio` | 1 | Async runtime |
| `indicatif` | 0.17 | CLI progress bars |
| `dirs` | 5 | Platform-aware directories |
| `anyhow` | 1 | Error handling |
| `chrono` | 0.4 | Timestamps |
| `uuid` | 1 | Download item IDs |

### Why `yt-dlp` crate?
- Pure Rust wrappers (`rustube`) break constantly due to YouTube API changes.
- The `yt-dlp` crate wraps the **Python yt-dlp** binary which is actively maintained by a massive community and supports 1800+ sites.
- The crate auto-downloads and caches the correct yt-dlp + ffmpeg binaries for your platform — zero manual setup.

---

## Adding new platforms

No code changes needed. yt-dlp already handles YouTube, X/Twitter, Instagram, TikTok, Facebook, Reddit, Twitch, Vimeo, SoundCloud, Bandcamp, and 1800+ more. Just paste any URL into ripdown.

---

## Roadmap

- [ ] Persistent queue (serde_json history file)
- [ ] Download history / library browser in TUI
- [ ] Thumbnail preview (sixel / kitty protocol)
- [ ] Locci Cloud integration — upload to RustFS after download
- [ ] M-Pesa premium format unlock (1080p+ gated)
- [ ] `ripdown serve` — local web UI for remote queuing
