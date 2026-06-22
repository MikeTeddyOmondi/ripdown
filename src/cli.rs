use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ripdown",
    about = "⚡ ripdown — blazing-fast video downloader for YouTube, X, Instagram & 1800+ sites",
    long_about = r#"
  ██████╗ ██╗██████╗ ██████╗  ██████╗ ██╗    ██╗███╗   ██╗
  ██╔══██╗██║██╔══██╗██╔══██╗██╔═══██╗██║    ██║████╗  ██║
  ██████╔╝██║██████╔╝██║  ██║██║   ██║██║ █╗ ██║██╔██╗ ██║
  ██╔══██╗██║██╔═══╝ ██║  ██║██║   ██║██║███╗██║██║╚██╗██║
  ██║  ██║██║██║     ██████╔╝╚██████╔╝╚███╔███╔╝██║ ╚████║
  ╚═╝  ╚═╝╚═╝╚═╝     ╚═════╝  ╚═════╝  ╚══╝╚══╝ ╚═╝  ╚═══╝

  Download videos from YouTube, X/Twitter, Instagram, and 1800+ platforms.
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
    /// URL(s) to download — YouTube, X, Instagram, TikTok, Vimeo, etc.
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

#[derive(ValueEnum, Clone, Debug)]
pub enum FormatChoice {
    /// Best available quality (default)
    Best,
    /// 4K / 2160p
    #[value(name = "4k")]
    FourK,
    /// 1080p Full HD
    #[value(name = "1080p")]
    Fhd,
    /// 720p HD
    #[value(name = "720p")]
    Hd,
    /// 480p SD
    #[value(name = "480p")]
    Sd,
    /// Audio only (mp3)
    Audio,
}

impl FormatChoice {
    /// Convert to a yt-dlp format string
    pub fn to_ytdlp_format(&self) -> &'static str {
        match self {
            FormatChoice::Best => "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
            FormatChoice::FourK => "bestvideo[height<=2160][ext=mp4]+bestaudio[ext=m4a]/best[height<=2160]",
            FormatChoice::Fhd => "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/best[height<=1080]",
            FormatChoice::Hd => "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/best[height<=720]",
            FormatChoice::Sd => "bestvideo[height<=480][ext=mp4]+bestaudio[ext=m4a]/best[height<=480]",
            FormatChoice::Audio => "bestaudio[ext=m4a]/bestaudio",
        }
    }
}
