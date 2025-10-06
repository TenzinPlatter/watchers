//! Configuration management for the file watcher.
//!
//! This module provides configuration loading and validation for the watchers application.
//! The configuration is typically loaded from a YAML file and defines watch directories,
//! timing parameters, and behavior settings.

use std::{fs::{self}, path::PathBuf};

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// Configuration settings for the file watcher.
///
/// The configuration defines all the parameters needed to run the file watcher,
/// including which directory to monitor, timing settings, and git behavior.
///
/// # Example YAML Configuration
///
/// ```yaml
/// watch_dir: "/path/to/monitor"
/// commit_delay_secs: 3
/// auto_push: true
/// ```
///
/// # Fields
///
/// * `name` - Unique name of watcher
/// * `config_path` - Optional path to the configuration file itself
/// * `watch_dir` - Directory to monitor for file changes
/// * `commit_delay_secs` - Seconds to wait after last change before creating commit
/// * `auto_push` - Whether to automatically push commits to remote (defaults to true)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Unique name of watcher
    pub name: String,
    /// Optional path to the configuration file (used for self-reference)
    pub config_path: Option<PathBuf>,
    /// Directory to watch for file changes
    pub watch_dir: PathBuf,
    /// Number of seconds to wait after the last file change before creating a commit
    pub commit_delay_secs: u32,
    /// Whether to automatically push commits to the remote repository
    #[serde(default = "default_true")]
    pub auto_push: bool,
}

impl Config {
    /// Loads configuration from a YAML file.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the YAML configuration file
    ///
    /// # Returns
    ///
    /// Returns a `Config` instance on success, or an error if the file cannot
    /// be read or parsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The configuration file cannot be read
    /// - The YAML content is invalid
    /// - Required fields are missing from the configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use watchers::Config;
    ///
    /// let config = Config::load("./config.yml")
    ///     .expect("Failed to load configuration");
    ///
    /// println!("Watching directory: {:?}", config.watch_dir);
    /// ```
    pub fn load(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(config_path)?;
        let user_config: Config = serde_yaml::from_str(&content)?;

        Ok(user_config)
    }

    /// Serializes the configuration to a YAML string.
    ///
    /// This is useful for debugging or saving configuration state.
    ///
    /// # Returns
    ///
    /// Returns a YAML string representation of the configuration,
    /// or a serialization error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use watchers::Config;
    /// use std::path::PathBuf;
    ///
    /// let config = Config {
    ///     watch_dir: PathBuf::from("/tmp"),
    ///     commit_delay_secs: 5,
    ///     auto_push: false,
    ///     config_path: None,
    /// };
    ///
    /// let yaml = config.dump().unwrap();
    /// println!("Config as YAML:\n{}", yaml);
    /// ```
    pub fn dump(&self) -> serde_yaml::Result<String> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_config_loading() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
watch_dir: "/tmp/test"
commit_delay_secs: 5
auto_push: false
"#).unwrap();

        let config = Config::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.watch_dir, PathBuf::from("/tmp/test"));
        assert_eq!(config.commit_delay_secs, 5);
        assert!(!config.auto_push);
        assert!(config.config_path.is_none());
    }

    #[test]
    fn test_config_auto_push_defaults_to_true() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
watch_dir: "/home/user/project"
commit_delay_secs: 10
"#).unwrap();

        let config = Config::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.watch_dir, PathBuf::from("/home/user/project"));
        assert_eq!(config.commit_delay_secs, 10);
        assert!(config.auto_push); // Should default to true
        assert!(config.config_path.is_none());
    }

    #[test]
    fn test_config_with_optional_fields() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
watch_dir: "/home/user/project"
commit_delay_secs: 10
auto_push: false
config_path: "/etc/watchers.yml"
"#).unwrap();

        let config = Config::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.watch_dir, PathBuf::from("/home/user/project"));
        assert_eq!(config.commit_delay_secs, 10);
        assert!(!config.auto_push);
        assert_eq!(config.config_path, Some(PathBuf::from("/etc/watchers.yml")));
    }

    #[test]
    fn test_config_dump() {
        let config = Config {
            watch_dir: PathBuf::from("/test/path"),
            commit_delay_secs: 3,
            auto_push: false,
            config_path: Some(PathBuf::from("/config/path")),
        };

        let yaml_output = config.dump().unwrap();
        assert!(yaml_output.contains("watch_dir: /test/path"));
        assert!(yaml_output.contains("commit_delay_secs: 3"));
        assert!(yaml_output.contains("auto_push: false"));
        assert!(yaml_output.contains("config_path: /config/path"));
    }

    #[test]
    fn test_config_invalid_file() {
        let result = Config::load("/nonexistent/file.yml");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_invalid_yaml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid: yaml: content: [").unwrap();

        let result = Config::load(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
