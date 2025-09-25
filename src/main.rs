use log::debug;

use watchers::{Watcher, config::Config, git::handle_event, watch_repo};

const CONFIG_PATH: &str = "./config/config.yml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = Config::load(CONFIG_PATH)?;
    debug!("Config:\n{}", config.dump().unwrap_or("failed to read config".to_string()));

    let mut watcher = Watcher::new(&config, |context| {
        handle_event(context);
    });

    watch_repo(&mut watcher)?;

    panic!("Should never get out of watching loop");
}
