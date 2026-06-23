mod cli;
mod download;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Download(args) => {
            download::run_download(args).await?;
        }
        Commands::Tui => {
            tui::run().await?;
        }
        Commands::Info { url } => {
            download::run_info(&url).await?;
        }
    }

    Ok(())
}
