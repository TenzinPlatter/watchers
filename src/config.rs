use std::{fs::{self}, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub config_path: Option<PathBuf>,
    pub watch_dir: PathBuf,
    pub commit_delay_secs: u32,
    pub auto_push: bool,
}

impl Config {
    pub fn load(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(config_path)?;
        let user_config: Config = serde_yaml::from_str(&content)?;

        Ok(user_config)
    }

    pub fn dump(&self) -> serde_yaml::Result<String> {
        serde_yaml::to_string(self)
    }
}
