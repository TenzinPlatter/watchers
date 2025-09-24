use std::io::stdout;

use watchers::{config::Config, file_utils::watch_repo};

const CONFIG_PATH: &str = "./config/config.yml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(CONFIG_PATH)?;
    let _ = config.dump_to(&mut stdout());
    watch_repo(&config.watch_dir)?;

    panic!("Should never get out of watching loop");
}
