use std::{fs::{self, File}, io::{self, Write}, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub config_path: Option<PathBuf>,
    pub watch_dir: PathBuf,
    #[serde(default)]
    pub verbose_logging: bool,
}

impl Config {
    pub fn load(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(config_path)?;
        let user_config: Config = serde_yaml::from_str(&content)?;

        Ok(user_config)
    }

    pub fn dump_to<T: Write>(&self, fd: &mut T) -> Result<(), Box<dyn std::error::Error>> {
        let s = serde_yaml::to_string(self)?;
        fd.write_all(s.as_bytes())?;
        Ok(())
    }
}
