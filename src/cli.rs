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

    Stop { name: String },

    Create { name: String },

    Delete { name: String },

    List {},

    #[command(hide = true, name = "__daemon")]
    Daemon { name: String }
}
