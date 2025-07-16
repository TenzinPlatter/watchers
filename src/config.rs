use std::{fs::{self, File}, io::Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub config_path: String,
    pub watch_dir: String,
    #[serde(default)]
    pub verbose_logging: bool,
}

impl Config {
    pub fn load(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(config_path)?;
        let user_config: Config = serde_yaml::from_str(&content)?;

        Ok(user_config)
    }

    pub fn dump_to(&self, file: &mut File) -> Result<(), Box<dyn std::error::Error>> {
        let s = serde_yaml::to_string(self)?;
        if file.write(s.as_bytes())? != s.len() {
            return Err("Failed to write entire config".into());
        }

        Ok(())
    }
}
