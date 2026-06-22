mod cli;
mod downloader;
mod queue;
mod tui;
mod types;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Download(args) => {
            downloader::run_download(args).await?;
        }
        Commands::Tui => {
            tui::run().await?;
        }
        Commands::Info { url } => {
            downloader::run_info(&url).await?;
        }
    }

    Ok(())
}
