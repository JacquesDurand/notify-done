use anyhow::{Context, Result};
use notify_rust::{Notification, Timeout, Urgency};

use crate::config::Config;
use crate::executor::ExecutionResult;

use super::formatter::{format_body, get_icon, get_title, get_urgency};

/// Send a notification for a completed task
pub fn send_notification(config: &Config, result: &ExecutionResult, name: Option<&str>) -> Result<()> {
    let title = get_title(config, result.success);
    let icon = get_icon(config, result.success);
    let urgency_str = get_urgency(config, result.success);
    let body = format_body(&config.format.body, result, name);

    let urgency = match urgency_str {
        "low" => Urgency::Low,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
    };

    let timeout = Timeout::Milliseconds(config.notification.timeout_ms);

    Notification::new()
        .summary(title)
        .body(&body)
        .icon(icon)
        .urgency(urgency)
        .timeout(timeout)
        .show()
        .context("Failed to show notification")?;

    Ok(())
}

/// Check if notification should be shown based on duration and config
pub fn should_notify(config: &Config, result: &ExecutionResult, cli_threshold: Option<u64>, quiet: bool) -> bool {
    if quiet {
        return false;
    }

    if config.general.always_notify {
        return true;
    }

    let threshold = cli_threshold.unwrap_or(config.general.threshold_seconds);
    result.duration.as_secs() >= threshold
}
