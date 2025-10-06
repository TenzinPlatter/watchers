use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "watchers")]
#[command(about = "File watcher manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Start { name: String },

    Create { name: String, path: String },

    Stop { name: String },

    List {},
}
