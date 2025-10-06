use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    pub watch_dir: PathBuf,
    pub commit_delay_secs: u32,
    #[serde(default = "default_true")]
    pub auto_push: bool,
}

#[derive(Clone, Serialize)]
pub struct Config {
    pub name: String,
    pub watch_dir: PathBuf,
    pub commit_delay_secs: u32,
    pub auto_push: bool,
}

pub fn get_watchers_config_dir() -> PathBuf {
    let proj_dir = ProjectDirs::from("", "", "watchers").unwrap();
    proj_dir.config_dir().to_path_buf()
}

impl Config {
    pub fn new(name: impl Into<String>, path: impl AsRef<Path>) -> Config {
        Config {
            name: name.into(),
            watch_dir: path.as_ref().to_path_buf(),
            commit_delay_secs: 60,
            auto_push: true,
        }
    }

    pub fn get_watcher_config_path(name: &str) -> Result<PathBuf> {
        let config_dir = get_watchers_config_dir();
        let yml_ext = config_dir.join(format!("{}.yml", name));
        if yml_ext.is_file() {
            return Ok(yml_ext);
        }

        let yaml_ext = config_dir.join(format!("{}.yaml", name));
        Ok(yaml_ext)
    }

    pub fn from_file<P: AsRef<Path>>(config_path: P) -> Result<Config> {
        let path = config_path.as_ref();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid config filename"))?
            .to_string();

        let content = fs::read_to_string(config_path)?;
        let user_config: ConfigFile =
            serde_yaml::from_str(&content).context("Failed to load config")?;
        Ok(Config {
            name,
            watch_dir: user_config.watch_dir,
            commit_delay_secs: user_config.commit_delay_secs,
            auto_push: user_config.auto_push,
        })
    }

    pub fn dump(&self) -> serde_yaml::Result<String> {
        serde_yaml::to_string(self)
    }
}
