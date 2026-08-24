# CLI Reference

Complete reference for the `ripdown` binary (the `ripdown-cli` crate). For the
HTTP service, see [API_REFERENCE.md](API_REFERENCE.md).

```
ripdown <COMMAND>
```

| Command                         | Alias | Purpose                            |
| ------------------------------- | ----- | ---------------------------------- |
| [`download`](#ripdown-download) | `dl`  | Download one or more URLs          |
| [`tui`](#ripdown-tui)           | `ui`  | Launch the interactive terminal UI |
| [`info`](#ripdown-info)         | —     | Print metadata without downloading |

Global flags: `--help`, `--version`.

---

## `ripdown download`

Download one or more YouTube URLs.

```
ripdown download [OPTIONS] <URL>...
ripdown dl [OPTIONS] <URL>...
```

### Arguments

| Argument   | Description                                                     |
| ---------- | --------------------------------------------------------------- |
| `<URL>...` | One or more YouTube video/playlist URLs (at least one required) |

### Options

| Flag                    | Env                  | Default               | Description                                    |
| ----------------------- | -------------------- | --------------------- | ---------------------------------------------- |
| `-o, --output <DIR>`    | `RIPDOWN_OUTPUT_DIR` | `~/Downloads/ripdown` | Output directory                               |
| `-f, --format <FORMAT>` | —                    | `best`                | `best`, `4k`, `1080p`, `720p`, `480p`, `audio` |
| `-a, --audio-only`      | —                    | off                   | Extract audio as mp3                           |
| `-p, --playlist`        | —                    | off                   | Download the full playlist                     |
| `-j, --jobs <N>`        | —                    | `3`                   | Maximum parallel downloads                     |
| `--subs <LANG>`         | —                    | —                     | Subtitle language code to embed (e.g. `en`)    |
| `-v, --verbose`         | —                    | off                   | Print raw yt-dlp output                        |

### Format values

| Value   | Resolves to                                          |
| ------- | ---------------------------------------------------- |
| `best`  | Best available muxed quality (default)               |
| `4k`    | Up to 2160p                                          |
| `1080p` | Up to 1080p                                          |
| `720p`  | Up to 720p                                           |
| `480p`  | Up to 480p                                           |
| `audio` | Best audio only (mp3) — equivalent to `--audio-only` |

### Examples

```bash
# Best quality
ripdown download https://www.youtube.com/watch?v=TSVHoHyErBQ

# 1080p into a specific folder (alias form)
ripdown dl -f 1080p -o ~/Videos https://youtu.be/TSVHoHyErBQ

# Audio only (mp3)
ripdown dl -a https://www.youtube.com/watch?v=TSVHoHyErBQ

# Several URLs, 5 parallel jobs
ripdown dl -j 5 https://youtu.be/aaa https://youtu.be/bbb

# Set the output dir via environment
RIPDOWN_OUTPUT_DIR=~/Media ripdown dl https://youtu.be/aaa
```

### Output location

Resolution order: `--output` flag → `$RIPDOWN_OUTPUT_DIR` → `~/Downloads/ripdown`.
Files are named `"<title> [<id>].<ext>"` (`.mp4` for video, `.mp3` for audio). An
already-present file with the expected name is treated as a cache hit and skipped.

---

## `ripdown tui`

Launch the interactive terminal UI — a live download queue.

```
ripdown tui
ripdown ui
```

### Keybindings

| Key            | Action                                           |
| -------------- | ------------------------------------------------ |
| `A` / `N`      | Open the "add URL" modal                         |
| `T`            | Toggle audio-only mode (applies to the next add) |
| `↑` / `k`      | Move selection up                                |
| `↓` / `j`      | Move selection down                              |
| `Enter`        | Confirm the URL in the add modal                 |
| `Esc`          | Cancel the add modal                             |
| `Q` / `Ctrl-C` | Quit                                             |

The header, queue table, per-item detail panel, and live stats bar mirror the
service's web UI styling. Downloads run in the background while you keep adding
more.

---

## `ripdown info`

Fetch and print metadata for a URL without downloading.

```
ripdown info <URL>
```

### Arguments

| Argument | Description              |
| -------- | ------------------------ |
| `<URL>`  | The video URL to inspect |

### Example

```bash
ripdown info https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

Prints title, uploader, duration, view count, upload date, and a truncated
description.

---

## `ripdown libs`

Inspect the cached yt-dlp + ffmpeg binaries, or reinstall them from scratch.

```bash
ripdown libs              # show which ripdown version installed them
ripdown libs --reinstall  # wipe <data_local>/ripdown/libs and install afresh
```

The binaries are stamped with the installing ripdown version in
`<data_local>/ripdown/libs/.ripdown-libs.json`. When that stamp is missing
(binaries left by an older ripdown), names a different version, or is older
than the max-age window, the next `download` / `tui` / `info` run wipes the
libs directory and reinstalls automatically.

## Environment variables

| Variable                     | Used by           | Description                                                        |
| ---------------------------- | ----------------- | ------------------------------------------------------------------ |
| `RIPDOWN_OUTPUT_DIR`         | `download`, `tui` | Default output directory                                           |
| `RIPDOWN_LIBS_MAX_AGE_DAYS`  | all               | Days before cached binaries are refreshed (default 30; `0` = never) |
| `RIPDOWN_SKIP_LIB_UPDATE`    | all               | Set to `1` to never re-fetch binaries (used by the Docker images)  |

## First run

On first download the `yt-dlp` crate fetches the yt-dlp + ffmpeg binaries
(~50 MB) into `<data_local>/ripdown/libs` and caches them. This needs network
access and only happens once, until the install goes stale (see
[`ripdown libs`](#ripdown-libs)).

---

See also: [GETTING_STARTED.md](GETTING_STARTED.md) · [SETUP.md](SETUP.md).
