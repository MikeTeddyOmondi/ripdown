use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

// `FormatChoice` is domain logic and lives in core; re-export it so the rest of
// the CLI keeps a single import path.
pub use ripdown_core::FormatChoice;

#[derive(Parser)]
#[command(
    name = "ripdown",
    about = "⚡ ripdown — a blazing-fast video downloader for YouTube",
    long_about = r#"
  ██████╗ ██╗██████╗ ██████╗  ██████╗ ██╗    ██╗███╗   ██╗
  ██╔══██╗██║██╔══██╗██╔══██╗██╔═══██╗██║    ██║████╗  ██║
  ██████╔╝██║██████╔╝██║  ██║██║   ██║██║ █╗ ██║██╔██╗ ██║
  ██╔══██╗██║██╔═══╝ ██║  ██║██║   ██║██║███╗██║██║╚██╗██║
  ██║  ██║██║██║     ██████╔╝╚██████╔╝╚███╔███╔╝██║ ╚████║
  ╚═╝  ╚═╝╚═╝╚═╝     ╚═════╝  ╚═════╝  ╚══╝╚══╝ ╚═╝  ╚═══╝

  Download videos from YouTube — fast.
  Powered by yt-dlp under the hood. Ships its own yt-dlp + ffmpeg binaries.
"#,
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download a video or playlist from a URL
    #[command(alias = "dl")]
    Download(DownloadArgs),

    /// Launch the interactive TUI (terminal user interface)
    #[command(alias = "ui")]
    Tui,

    /// Fetch and display metadata for a URL without downloading
    Info {
        /// The video URL to inspect
        url: String,
    },
}

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    /// URL(s) to download from YouTube
    #[arg(required = true, value_name = "URL")]
    pub urls: Vec<String>,

    /// Output directory for downloaded files
    #[arg(short, long, value_name = "DIR", env = "RIPDOWN_OUTPUT_DIR")]
    pub output: Option<PathBuf>,

    /// Download format / quality preference
    #[arg(short, long, value_enum, default_value = "best")]
    pub format: FormatChoice,

    /// Extract audio only (saves as mp3)
    #[arg(short = 'a', long)]
    pub audio_only: bool,

    /// Download entire playlist (if URL is a playlist)
    #[arg(short = 'p', long)]
    pub playlist: bool,

    /// Maximum number of parallel downloads
    #[arg(short = 'j', long, default_value = "3", value_name = "N")]
    pub jobs: usize,

    /// Subtitle language code to embed (e.g. "en", "fr")
    #[arg(long, value_name = "LANG")]
    pub subs: Option<String>,

    /// Print verbose yt-dlp output
    #[arg(short, long)]
    pub verbose: bool,
}
