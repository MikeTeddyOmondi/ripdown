//! The download engine: a thin, presentation-free wrapper around yt-dlp.
//!
//! Front-ends (CLI progress bars, TUI queue, HTTP service) observe progress via
//! the [`DownloadStatus`] callback passed to [`download_with_progress`] — no
//! `println!`/`indicatif` lives here.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use yt_dlp::{client::deps::Libraries, Downloader};

use crate::config::{clear_libs, libs_dir, libs_state, write_libs_manifest, LibsState};
use crate::models::DownloadStatus;

/// Lightweight, serializable view of a video's metadata.
#[derive(Debug, Clone)]
pub struct VideoMeta {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<i64>,
    pub view_count: Option<i64>,
    pub upload_date: Option<String>,
    pub description: Option<String>,
}

/// Ensure the yt-dlp + ffmpeg binaries are installed and owned by this ripdown
/// build, returning the resolved [`Libraries`].
///
/// Binaries left behind by an older ripdown (or older than the max-age window)
/// are wiped and reinstalled from scratch — a partial upgrade of a cached libs
/// dir is exactly how yt-dlp/ffmpeg mismatches happen. Pass `force` to
/// reinstall unconditionally (`ripdown libs --reinstall`).
///
/// A healthy, current install is a no-op: `install_dependencies` queries
/// `api.github.com`, which is rate-limited (HTTP 403) from shared datacenter
/// IPs like Render's, so deployments with pre-baked binaries (which also set
/// `RIPDOWN_SKIP_LIB_UPDATE`) never touch the GitHub API at runtime.
pub async fn ensure_libraries(force: bool) -> Result<Libraries> {
    let state = libs_state();

    if force || matches!(state, LibsState::Stale(_)) {
        if let LibsState::Stale(reason) = &state {
            tracing::info!("reinstalling yt-dlp + ffmpeg binaries: {reason}");
        }
        clear_libs().context("Failed to remove the stale yt-dlp / ffmpeg binaries")?;
    }

    let libs = libs_dir();
    let libraries = Libraries::new(libs.join("yt-dlp"), libs.join("ffmpeg"));

    if !force && state == LibsState::Current {
        return Ok(libraries);
    }

    let libraries = libraries
        .install_dependencies()
        .await
        .context("Failed to install yt-dlp / ffmpeg binaries (check network access)")?;

    // Stamp the install so a later ripdown can tell it owns these binaries.
    if let Err(e) = write_libs_manifest() {
        tracing::warn!("failed to write the libs version stamp: {e}");
    }

    Ok(libraries)
}

/// Build a configured yt-dlp [`Downloader`], installing or refreshing the
/// yt-dlp + ffmpeg binaries as needed (a no-op once cached and current).
pub async fn build_downloader(output_dir: &Path) -> Result<Downloader> {
    let libraries = ensure_libraries(false).await?;

    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    Downloader::builder(libraries, output_dir.to_path_buf())
        .build()
        .await
        .context("Failed to initialise yt-dlp downloader")
}

/// Download a single URL, returning the absolute path of the saved file.
///
/// If a file with the expected name already exists it is treated as a cache hit.
pub async fn download_single(
    downloader: &Downloader,
    url: &str,
    audio_only: bool,
    output_dir: &Path,
) -> Result<String> {
    let info = downloader
        .fetch_video_infos(url)
        .await
        .context("Failed to fetch video info")?;

    let ext = if audio_only { "mp3" } else { "mp4" };
    let safe_title = sanitize_filename(&info.title);
    let filename = format!("{} [{}].{}", safe_title, info.id, ext);
    let expected_path = output_dir.join(&filename);

    if expected_path.exists() {
        return Ok(expected_path.to_string_lossy().into_owned());
    }

    let path = if audio_only {
        downloader
            .download_audio_stream(&info, &filename)
            .await
            .context("Failed to download audio")?
    } else {
        downloader
            .download_video(&info, &filename)
            .await
            .context("Failed to download video")?
    };

    Ok(path.to_string_lossy().into_owned())
}

/// Download a URL, reporting lifecycle transitions through `on_progress`.
///
/// This is the shared entry point for the TUI queue and HTTP service. The
/// downloader is built lazily so callers only pay the binary-bootstrap cost when
/// a download actually runs.
pub async fn download_with_progress<F>(
    url: &str,
    audio_only: bool,
    output_dir: &Path,
    mut on_progress: F,
) -> Result<String>
where
    F: FnMut(DownloadStatus),
{
    on_progress(DownloadStatus::FetchingInfo);

    let downloader = build_downloader(output_dir).await?;

    if let Ok(meta) = fetch_info(&downloader, url).await {
        on_progress(DownloadStatus::Downloading {
            progress: 10.0,
            speed: "…".into(),
            eta: "…".into(),
        });
        // Surface the resolved title to observers that also poll the queue.
        let _ = meta;
    }

    match download_single(&downloader, url, audio_only, output_dir).await {
        Ok(path) => {
            on_progress(DownloadStatus::Done { path: path.clone() });
            Ok(path)
        }
        Err(e) => {
            on_progress(DownloadStatus::Failed {
                reason: format!("{e:#}"),
            });
            Err(e)
        }
    }
}

/// Fetch metadata for a URL using an existing downloader.
pub async fn fetch_info(downloader: &Downloader, url: &str) -> Result<VideoMeta> {
    let info = downloader
        .fetch_video_infos(url)
        .await
        .context("Failed to fetch video info")?;

    Ok(VideoMeta {
        id: info.id,
        title: info.title,
        uploader: info.uploader,
        duration_secs: info.duration,
        view_count: info.view_count,
        upload_date: info.upload_date.map(|ts| ts.to_string()),
        description: info.description,
    })
}

/// Convenience: build a throwaway downloader and fetch metadata for a URL.
pub async fn fetch_info_standalone(url: &str) -> Result<VideoMeta> {
    let tmp = std::env::temp_dir().join("ripdown-info");
    let downloader = build_downloader(&tmp).await?;
    fetch_info(&downloader, url).await
}

/// Fetch just the title + uploader (used by the TUI queue rows).
pub async fn fetch_title(url: &str) -> Result<(String, String)> {
    let meta = fetch_info_standalone(url).await?;
    let uploader = meta.uploader.unwrap_or_else(|| "Unknown".into());
    Ok((meta.title, uploader))
}

/// Build a downloader and download a single URL in one call (used by the CLI).
pub async fn download_url(url: &str, audio_only: bool, output_dir: &Path) -> Result<String> {
    let downloader = build_downloader(output_dir).await?;
    download_single(&downloader, url, audio_only, output_dir).await
}

/// Replace filesystem-hostile characters and cap the length of a filename stem.
pub fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .chars()
        .take(120)
        .collect()
}

/// Best-effort platform label derived from a URL host.
pub fn detect_platform(url: &str) -> &'static str {
    if url.contains("youtube") || url.contains("youtu.be") {
        "YouTube"
    } else {
        "…"
    }
}

/// Expose the canonical output-dir resolver from [`crate::config`].
pub fn resolve_output_dir(arg: Option<PathBuf>) -> PathBuf {
    crate::config::resolve_output_dir(arg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_illegal_chars() {
        assert_eq!(sanitize_filename("a/b:c*?"), "a_b_c__");
    }

    #[test]
    fn detect_platform_recognises_youtube() {
        assert_eq!(detect_platform("https://youtu.be/abc"), "YouTube");
        assert_eq!(
            detect_platform("https://www.youtube.com/watch?v=x"),
            "YouTube"
        );
        assert_eq!(detect_platform("https://example.com/x"), "…");
    }
}
