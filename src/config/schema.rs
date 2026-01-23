use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub notification: NotificationConfig,
    pub format: FormatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Only notify if task takes longer than this (seconds)
    pub threshold_seconds: u64,
    /// Always show notification regardless of threshold
    pub always_notify: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationConfig {
    /// Notification timeout in milliseconds
    pub timeout_ms: u32,
    /// Urgency level: low, normal, critical
    pub urgency: String,
    /// Icon for successful completion
    pub icon: String,
    /// Icon for failed completion
    pub icon_failure: String,
    /// Urgency level for failures
    pub urgency_failure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    /// Title for successful completion
    pub title_success: String,
    /// Title for failed completion
    pub title_failure: String,
    /// Body template with placeholders: {command}, {duration}, {exit_code}, {name}
    pub body: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            notification: NotificationConfig::default(),
            format: FormatConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            threshold_seconds: 10,
            always_notify: false,
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            urgency: "normal".to_string(),
            icon: "dialog-information".to_string(),
            icon_failure: "dialog-error".to_string(),
            urgency_failure: "critical".to_string(),
        }
    }
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            title_success: "Task Completed".to_string(),
            title_failure: "Task Failed".to_string(),
            body: "Command: {command}\nDuration: {duration}\nExit code: {exit_code}".to_string(),
        }
    }
}
