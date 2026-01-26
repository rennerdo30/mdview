//! Configuration loader

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use super::schema::Config;

/// Load configuration from the default location
pub fn load_default() -> Result<Config, ConfigError> {
    if let Some(config_path) = get_default_config_path() {
        if config_path.exists() {
            return load_from_path(&config_path);
        }
    }

    // Return default config if no config file exists
    Ok(Config::default())
}

/// Load configuration from a specific path
pub fn load_from_path(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    let config: Config = toml::from_str(&content).map_err(ConfigError::Parse)?;
    Ok(config)
}

/// Get the default configuration file path
pub fn get_default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "mdview", "mdview")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Get the configuration directory path
pub fn get_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "mdview", "mdview")
        .map(|dirs| dirs.config_dir().to_path_buf())
}

/// Get the themes directory path
pub fn get_themes_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "mdview", "mdview")
        .map(|dirs| dirs.config_dir().join("themes"))
}

/// Save configuration to a file
pub fn save_config(config: &Config, path: &Path) -> Result<(), ConfigError> {
    let content = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
    }

    std::fs::write(path, content).map_err(ConfigError::Io)?;
    Ok(())
}

/// Create default configuration file if it doesn't exist
pub fn create_default_config() -> Result<PathBuf, ConfigError> {
    let config_path = get_default_config_path().ok_or(ConfigError::NoConfigDir)?;

    if !config_path.exists() {
        let config = Config::default();
        save_config(&config, &config_path)?;
    }

    Ok(config_path)
}

/// Configuration loading errors
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    NoConfigDir,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Parse(e) => write!(f, "Parse error: {}", e),
            ConfigError::Serialize(e) => write!(f, "Serialize error: {}", e),
            ConfigError::NoConfigDir => write!(f, "Could not determine config directory"),
        }
    }
}

impl std::error::Error for ConfigError {}
