use watchers::{watch_directory, config::Config};

const CONFIG_PATH: &str = "./config/config.yml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(CONFIG_PATH)?;
    watch_directory(&config.watch_dir)?;

    todo!()
}
