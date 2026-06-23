//! Filesystem location helpers shared by every front-end.

use std::path::PathBuf;

/// Application data directory (`<data_local>/ripdown`), used for cached binaries.
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ripdown")
}

/// Directory holding the auto-downloaded yt-dlp + ffmpeg binaries.
pub fn libs_dir() -> PathBuf {
    data_dir().join("libs")
}

/// Resolve the output directory: explicit arg → `RIPDOWN_OUTPUT_DIR` → `~/Downloads/ripdown`.
pub fn resolve_output_dir(arg: Option<PathBuf>) -> PathBuf {
    arg.or_else(|| std::env::var("RIPDOWN_OUTPUT_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("ripdown")
        })
}

/// Whether the yt-dlp + ffmpeg binaries have already been downloaded.
pub fn binaries_present() -> bool {
    let libs = libs_dir();
    libs.join("yt-dlp").exists() && libs.join("ffmpeg").exists()
}
