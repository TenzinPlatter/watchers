mod cli;
mod config;
mod debouncer;
mod file_utils;
mod git;
mod systemd;
mod watcher;

use anyhow::Result;
use clap::Parser;

use crate::{
    cli::{Cli, Commands},
    watcher::{
        create_watcher, delete_watcher, list_watchers, run_daemon, start_watcher, stop_watcher,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start { name } => {
            start_watcher(name).await?;
        }

        Commands::Stop { name } => {
            stop_watcher(name).await?;
        }

        Commands::Create { name } => {
            create_watcher(name).await?;
        }

        Commands::Delete { name } => {
            delete_watcher(name)?;
        }

        Commands::List {} => {
            list_watchers()?;
        }

        Commands::Daemon { name } => {
            run_daemon(name).await?;
        }
    }

    Ok(())
}
