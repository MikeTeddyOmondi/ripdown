use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use yt_dlp::{
    client::deps::Libraries,
    Downloader,
};

use crate::cli::{DownloadArgs, FormatChoice};

/// Resolve the output directory: CLI arg → env → ~/Downloads/ripdown
pub fn resolve_output_dir(arg: Option<PathBuf>) -> PathBuf {
    arg.or_else(|| {
        std::env::var("RIPDOWN_OUTPUT_DIR").ok().map(PathBuf::from)
    })
    .unwrap_or_else(|| {
        dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ripdown")
    })
}

/// Build a configured yt-dlp Downloader, auto-downloading binaries if needed.
pub async fn build_downloader(output_dir: &PathBuf) -> Result<Downloader> {
    // By default, yt-dlp crate auto-downloads yt-dlp + ffmpeg into a libs/ dir
    let libs_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ripdown")
        .join("libs");

    let libraries = Libraries::new(
        libs_dir.join("yt-dlp"),
        libs_dir.join("ffmpeg"),
    );

    // Download yt-dlp + ffmpeg binaries on first run (no-op if already present)
    let libraries = libraries
        .install_dependencies()
        .await
        .context("Failed to install yt-dlp / ffmpeg binaries (check network access)")?;

    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let downloader = Downloader::builder(libraries, output_dir.clone())
        .build()
        .await
        .context("Failed to initialise yt-dlp downloader")?;

    Ok(downloader)
}

/// CLI download command: one or more URLs with indicatif progress bars.
pub async fn run_download(args: DownloadArgs) -> Result<()> {
    let output_dir = resolve_output_dir(args.output.clone());

    println!();
    println!("  ⚡ \x1b[1;36mripdown\x1b[0m  →  {}", output_dir.display());
    println!("  {} URL(s) queued | format: {:?} | jobs: {}",
        args.urls.len(), args.format, args.jobs);
    println!();

    // First run will auto-download yt-dlp + ffmpeg binaries — warn user
    let libs_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ripdown")
        .join("libs");

    if !libs_dir.join("yt-dlp").exists() || !libs_dir.join("ffmpeg").exists() {
        println!("  \x1b[33m[first run]\x1b[0m Downloading yt-dlp + ffmpeg binaries (~50 MB)…");
        println!("  This only happens once and they are cached at:\n  {}", libs_dir.display());
        println!();
    }

    let downloader = Arc::new(build_downloader(&output_dir).await?);

    let multi = MultiProgress::new();
    let bar_style = ProgressStyle::with_template(
        "  {spinner:.cyan} [{bar:40.cyan/blue}] {percent}%  {msg}",
    )?
    .progress_chars("█▓░");

    // Determine format string
    let fmt = if args.audio_only {
        FormatChoice::Audio.to_ytdlp_format()
    } else {
        args.format.to_ytdlp_format()
    };

    // Spawn downloads with bounded concurrency
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(args.jobs));
    let mut handles = Vec::new();

    for url in &args.urls {
        let permit = semaphore.clone().acquire_owned().await?;
        let bar = multi.add(ProgressBar::new(100));
        bar.set_style(bar_style.clone());
        bar.set_message(shorten_url(url));

        let downloader = Arc::clone(&downloader);
        let url = url.clone();
        let fmt = fmt.to_string();
        let audio_only = args.audio_only;
        let output_dir = output_dir.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit;
            bar.set_message(format!("fetching info…  {}", shorten_url(&url)));

            let result = download_single(&downloader, &url, &fmt, audio_only, &output_dir).await;

            match result {
                Ok(path) => {
                    bar.set_position(100);
                    bar.finish_with_message(format!(
                        "✓  {}",
                        PathBuf::from(&path).file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or(path)
                    ));
                }
                Err(e) => {
                    bar.abandon_with_message(format!("✗  {}", e));
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    println!();
    println!("  \x1b[1;32mAll done!\x1b[0m  Files saved to: {}", output_dir.display());
    println!();

    Ok(())
}

async fn download_single(
    downloader: &Downloader,
    url: &str,
    _format: &str,
    audio_only: bool,
    output_dir: &PathBuf,
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

    if audio_only {
        let path = downloader
            .download_audio_stream(&info, &filename)
            .await
            .context("Failed to download audio")?;
        Ok(path.to_string_lossy().into_owned())
    } else {
        let path = downloader
            .download_video(&info, &filename)
            .await
            .context("Failed to download video")?;
        Ok(path.to_string_lossy().into_owned())
    }
}

/// Fetch and print video metadata without downloading.
pub async fn run_info(url: &str) -> Result<()> {
    let tmp = std::env::temp_dir().join("ripdown-info");
    std::fs::create_dir_all(&tmp)?;

    println!();
    println!("  \x1b[36m[ripdown info]\x1b[0m  Fetching metadata…");
    println!("  URL: {url}");
    println!();

    let downloader = build_downloader(&tmp).await?;
    let info = downloader
        .fetch_video_infos(url)
        .await
        .context("Failed to fetch video info")?;

    // let title = info.title.unwrap_or_else(|| "(no title)".into());
    let title = info.title;
    let uploader = info.uploader.unwrap_or_else(|| "unknown".into());
    let duration = info.duration
        .map(|d| format!("{}:{:02}", d / 60, d % 60))
        .unwrap_or_else(|| "N/A".into());
    let view_count = info.view_count
        .map(|v| format_number(v as u64))
        .unwrap_or_else(|| "N/A".into());
    let upload_date = info.upload_date
        .map(|ts| ts.to_string())
        .unwrap_or_else(|| "N/A".into());

    println!("  \x1b[1mTitle\x1b[0m        {title}");
    println!("  \x1b[1mUploader\x1b[0m     {uploader}");
    println!("  \x1b[1mDuration\x1b[0m     {duration}");
    println!("  \x1b[1mViews\x1b[0m        {view_count}");
    println!("  \x1b[1mUploaded\x1b[0m     {upload_date}");

    if let Some(desc) = info.description {
        let snippet: String = desc.chars().take(200).collect();
        println!();
        println!("  \x1b[1mDescription\x1b[0m");
        println!("  {}", snippet.replace('\n', "\n  "));
        if desc.len() > 200 {
            println!("  \x1b[2m… (truncated)\x1b[0m");
        }
    }

    println!();
    Ok(())
}

fn sanitize_filename(s: &str) -> String {
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

fn shorten_url(url: &str) -> String {
    if url.len() > 55 {
        format!("{}…", &url[..52])
    } else {
        url.to_string()
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

/// Expose a simpler download function for TUI use
pub async fn download_url_for_tui(
    url: &str,
    format: &str,
    audio_only: bool,
    output_dir: &PathBuf,
) -> Result<String> {
    let downloader = Arc::new(build_downloader(output_dir).await?);
    download_single(&downloader, url, format, audio_only, output_dir).await
}

/// Fetch title for TUI display
pub async fn fetch_title_for_tui(url: &str) -> Result<(String, String)> {
    let tmp = std::env::temp_dir().join("ripdown-tui-info");
    std::fs::create_dir_all(&tmp)?;
    let downloader = build_downloader(&tmp).await?;
    let info = downloader.fetch_video_infos(url).await?;
    // let title = info.title.unwrap_or_else(|| shorten_url(url));
    let title = info.title;
    let uploader = info.uploader.unwrap_or_else(|| "Unknown".into());
    Ok((title, uploader))
}
