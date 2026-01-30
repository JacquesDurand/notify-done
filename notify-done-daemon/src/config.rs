use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// System-wide daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Minimum UID to track (default: 1000)
    #[serde(default = "default_min_uid")]
    pub min_uid: u32,

    /// Minimum duration in seconds before sending notification
    #[serde(default = "default_threshold_seconds")]
    pub threshold_seconds: u64,

    /// Command patterns to ignore (glob-style)
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Whether to log all events (debug mode)
    #[serde(default)]
    pub debug: bool,
}

fn default_min_uid() -> u32 {
    1000
}

fn default_threshold_seconds() -> u64 {
    10
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            min_uid: default_min_uid(),
            threshold_seconds: default_threshold_seconds(),
            ignore_patterns: default_ignore_patterns(),
            debug: false,
        }
    }
}

fn default_ignore_patterns() -> Vec<String> {
    vec![
        // Editors and pagers
        "vim".into(),
        "nvim".into(),
        "nano".into(),
        "less".into(),
        "more".into(),
        "man".into(),
        // Shells
        "bash".into(),
        "zsh".into(),
        "fish".into(),
        "sh".into(),
        // Interactive tools
        "ssh".into(),
        "tmux".into(),
        "screen".into(),
        "htop".into(),
        "top".into(),
        // Very short-lived commands
        "ls".into(),
        "cat".into(),
        "grep".into(),
        "find".into(),
        "pwd".into(),
        "cd".into(),
        "echo".into(),
        "printf".into(),
        "test".into(),
        "[".into(),
    ]
}

/// Per-user configuration (overrides system config)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// Override threshold for this user
    pub threshold_seconds: Option<u64>,

    /// Additional patterns to ignore
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Patterns to always notify (even if in system ignore list)
    #[serde(default)]
    pub always_notify: Vec<String>,

    /// Disable notifications entirely
    #[serde(default)]
    pub disabled: bool,
}

impl DaemonConfig {
    /// Load configuration from system path
    pub fn load() -> Result<Self> {
        let path = Self::system_config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// System configuration file path
    pub fn system_config_path() -> PathBuf {
        PathBuf::from("/etc/notify-done/config.toml")
    }

    /// Check if a command should be ignored
    pub fn should_ignore(&self, comm: &str) -> bool {
        self.ignore_patterns.iter().any(|p| {
            if p.contains('*') {
                // Simple glob matching
                let parts: Vec<&str> = p.split('*').collect();
                if parts.len() == 2 {
                    comm.starts_with(parts[0]) && comm.ends_with(parts[1])
                } else {
                    comm == p
                }
            } else {
                comm == p
            }
        })
    }
}

impl UserConfig {
    /// Load user configuration from their home directory
    pub fn load_for_uid(uid: u32) -> Result<Option<Self>> {
        let path = Self::user_config_path(uid)?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(Some(toml::from_str(&content)?))
        } else {
            Ok(None)
        }
    }

    /// Get config path for a user
    fn user_config_path(uid: u32) -> Result<PathBuf> {
        // Read passwd to get home directory
        let passwd_entry = std::fs::read_to_string("/etc/passwd")?;
        for line in passwd_entry.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 6 {
                if let Ok(entry_uid) = parts[2].parse::<u32>() {
                    if entry_uid == uid {
                        let home = parts[5];
                        return Ok(PathBuf::from(home).join(".config/notify-done/config.toml"));
                    }
                }
            }
        }
        Ok(PathBuf::from(format!("/home/{}", uid)).join(".config/notify-done/config.toml"))
    }
}

/// Combined configuration for a specific user
pub struct EffectiveConfig {
    pub threshold_seconds: u64,
    pub ignore_set: HashSet<String>,
    pub always_notify: HashSet<String>,
    pub disabled: bool,
}

impl EffectiveConfig {
    pub fn new(daemon: &DaemonConfig, user: Option<&UserConfig>) -> Self {
        let threshold_seconds = user
            .and_then(|u| u.threshold_seconds)
            .unwrap_or(daemon.threshold_seconds);

        let mut ignore_set: HashSet<String> = daemon.ignore_patterns.iter().cloned().collect();
        let mut always_notify = HashSet::new();
        let mut disabled = false;

        if let Some(user) = user {
            ignore_set.extend(user.ignore_patterns.iter().cloned());
            always_notify.extend(user.always_notify.iter().cloned());
            disabled = user.disabled;
        }

        Self {
            threshold_seconds,
            ignore_set,
            always_notify,
            disabled,
        }
    }

    pub fn should_notify(&self, comm: &str, duration_secs: u64) -> bool {
        if self.disabled {
            return false;
        }

        // Check always_notify first
        if self.always_notify.contains(comm) {
            return duration_secs >= self.threshold_seconds;
        }

        // Check ignore list
        if self.ignore_set.contains(comm) {
            return false;
        }

        duration_secs >= self.threshold_seconds
    }
}
