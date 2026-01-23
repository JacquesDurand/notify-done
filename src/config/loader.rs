use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::schema::Config;

/// Get the config directory path
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("notify-done"))
}

/// Get the config file path
pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("config.toml"))
}

/// Load config from file, falling back to defaults
pub fn load_config() -> Result<Config> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };

    if !path.exists() {
        return Ok(Config::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

/// Create default config file
pub fn init_config() -> Result<PathBuf> {
    let dir = config_dir().context("Could not determine config directory")?;
    let path = dir.join("config.toml");

    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config directory: {}", dir.display()))?;

    let config = Config::default();
    let contents = toml::to_string_pretty(&config).context("Failed to serialize default config")?;

    fs::write(&path, contents)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(path)
}

/// Get the socket path for daemon communication
pub fn socket_path() -> Option<PathBuf> {
    dirs::runtime_dir()
        .or_else(|| std::env::var("XDG_RUNTIME_DIR").ok().map(PathBuf::from))
        .or_else(|| Some(PathBuf::from("/tmp")))
        .map(|p| p.join("notify-done.sock"))
}

/// Get the history file path
pub fn history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("notify-done").join("history.json"))
}
