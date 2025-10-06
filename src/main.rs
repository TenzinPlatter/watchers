use clap::Parser;
use log::debug;

use watchers::{
    cli::{Cli, Commands}, config::Config, git::handle_event, start_watcher, watch_repo, Watcher
};

const CONFIG_PATH: &str = "./config/config.yml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start { name } => {
            start_watcher(name);
        }

        Commands::Stop { name } => {
            println!("Stopping {}", name);
        }

        Commands::Create { name, path } => {
            println!("Creating {} at {}", name, path);
        }

        Commands::List {} => {
            println!("Listing");
        }
    }

    let config = Config::load(CONFIG_PATH)?;
    debug!(
        "Config:\n{}",
        config.dump().unwrap_or("failed to read config".to_string())
    );

    let mut watcher = Watcher::new(&config, |context| {
        handle_event(context);
    });

    watch_repo(&mut watcher)?;

    panic!("Should never get out of watching loop");
}
