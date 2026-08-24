//! CLI-facing download + info commands (progress bars and ANSI output).

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use ripdown_core::config::{
    binaries_present, libs_dir, libs_state, read_libs_manifest, resolve_output_dir, LibsState,
};
use ripdown_core::engine::{
    build_downloader, download_single, ensure_libraries, fetch_info_standalone,
};

use crate::cli::DownloadArgs;

/// `ripdown download` — one or more URLs with indicatif progress bars.
pub async fn run_download(args: DownloadArgs) -> Result<()> {
    let output_dir = resolve_output_dir(args.output.clone());

    println!();
    println!("  ⚡ \x1b[1;36mripdown\x1b[0m  →  {}", output_dir.display());
    println!(
        "  {} URL(s) queued | format: {:?} | jobs: {}",
        args.urls.len(),
        args.format,
        args.jobs
    );
    println!();

    // Binaries left by an older ripdown are refreshed before the first download.
    if let LibsState::Stale(reason) = libs_state() {
        println!("  \x1b[33m[libs]\x1b[0m Refreshing yt-dlp + ffmpeg — {reason}");
        println!();
    }

    // First run will auto-download yt-dlp + ffmpeg binaries — warn the user.
    if !binaries_present() {
        println!("  \x1b[33m[first run]\x1b[0m Downloading yt-dlp + ffmpeg binaries (~50 MB)…");
        println!(
            "  This only happens once and they are cached at:\n  {}",
            libs_dir().display()
        );
        println!();
    }

    let downloader = Arc::new(build_downloader(&output_dir).await?);

    let multi = MultiProgress::new();
    let bar_style =
        ProgressStyle::with_template("  {spinner:.cyan} [{bar:40.cyan/blue}] {percent}%  {msg}")?
            .progress_chars("█▓░");

    let audio_only = args.audio_only || args.format.is_audio_only();

    // Spawn downloads with bounded concurrency.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(args.jobs.max(1)));
    let mut handles = Vec::new();

    for url in &args.urls {
        let permit = semaphore.clone().acquire_owned().await?;
        let bar = multi.add(ProgressBar::new(100));
        bar.set_style(bar_style.clone());
        bar.set_message(shorten_url(url));

        let downloader = Arc::clone(&downloader);
        let url = url.clone();
        let output_dir = output_dir.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit;
            bar.set_message(format!("fetching info…  {}", shorten_url(&url)));

            let result = download_single(&downloader, &url, audio_only, &output_dir).await;

            match result {
                Ok(path) => {
                    bar.set_position(100);
                    bar.finish_with_message(format!(
                        "✓  {}",
                        PathBuf::from(&path)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or(path)
                    ));
                }
                Err(e) => {
                    bar.abandon_with_message(format!("✗  {e}"));
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    println!();
    println!(
        "  \x1b[1;32mAll done!\x1b[0m  Files saved to: {}",
        output_dir.display()
    );
    println!();

    Ok(())
}

/// `ripdown info` — fetch and print video metadata without downloading.
pub async fn run_info(url: &str) -> Result<()> {
    println!();
    println!("  \x1b[36m[ripdown info]\x1b[0m  Fetching metadata…");
    println!("  URL: {url}");
    println!();

    let info = fetch_info_standalone(url).await?;

    let uploader = info.uploader.unwrap_or_else(|| "unknown".into());
    let duration = info
        .duration_secs
        .map(|d| format!("{}:{:02}", d / 60, d % 60))
        .unwrap_or_else(|| "N/A".into());
    let view_count = info
        .view_count
        .map(|v| format_number(v as u64))
        .unwrap_or_else(|| "N/A".into());
    let upload_date = info.upload_date.unwrap_or_else(|| "N/A".into());

    println!("  \x1b[1mTitle\x1b[0m        {}", info.title);
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

/// `ripdown libs` — report on the cached binaries, optionally reinstalling them.
pub async fn run_libs(reinstall: bool) -> Result<()> {
    println!();
    println!("  ⚡ \x1b[1;36mripdown libs\x1b[0m  →  {}", libs_dir().display());

    match read_libs_manifest() {
        Some(m) => println!(
            "  installed by ripdown {} on {}",
            m.ripdown_version,
            m.installed_at.format("%Y-%m-%d %H:%M UTC")
        ),
        None if binaries_present() => {
            println!("  installed by an older ripdown (no version stamp)")
        }
        None => println!("  not installed yet"),
    }

    match libs_state() {
        LibsState::Current if !reinstall => {
            println!("  \x1b[32mup to date\x1b[0m — nothing to do");
            println!();
            return Ok(());
        }
        LibsState::Missing => println!("  \x1b[33mmissing\x1b[0m — installing…"),
        LibsState::Stale(reason) => println!("  \x1b[33mstale\x1b[0m — {reason}; reinstalling…"),
        LibsState::Current => println!("  reinstalling on request…"),
    }

    ensure_libraries(reinstall).await?;
    println!("  \x1b[32mdone\x1b[0m — yt-dlp + ffmpeg installed fresh");
    println!();
    Ok(())
}
