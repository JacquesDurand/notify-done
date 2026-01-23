use crate::config::Config;
use crate::executor::{format_duration, ExecutionResult};

/// Format the notification body using template placeholders
pub fn format_body(template: &str, result: &ExecutionResult, name: Option<&str>) -> String {
    let duration_str = format_duration(result.duration);
    let display_name = name.unwrap_or(&result.command);

    template
        .replace("{command}", &result.command)
        .replace("{duration}", &duration_str)
        .replace("{exit_code}", &result.exit_code.to_string())
        .replace("{name}", display_name)
}

/// Get the appropriate title based on success/failure
pub fn get_title(config: &Config, success: bool) -> &str {
    if success {
        &config.format.title_success
    } else {
        &config.format.title_failure
    }
}

/// Get the appropriate icon based on success/failure
pub fn get_icon(config: &Config, success: bool) -> &str {
    if success {
        &config.notification.icon
    } else {
        &config.notification.icon_failure
    }
}

/// Get the appropriate urgency based on success/failure
pub fn get_urgency(config: &Config, success: bool) -> &str {
    if success {
        &config.notification.urgency
    } else {
        &config.notification.urgency_failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_body() {
        let result = ExecutionResult {
            command: "cargo build".to_string(),
            exit_code: 0,
            duration: Duration::from_secs(65),
            success: true,
        };

        let template = "Command: {command}\nDuration: {duration}\nExit: {exit_code}";
        let formatted = format_body(template, &result, None);

        assert!(formatted.contains("cargo build"));
        assert!(formatted.contains("1m 5s"));
        assert!(formatted.contains("Exit: 0"));
    }

    #[test]
    fn test_format_body_with_name() {
        let result = ExecutionResult {
            command: "cargo build --release".to_string(),
            exit_code: 0,
            duration: Duration::from_secs(30),
            success: true,
        };

        let template = "Task: {name}";
        let formatted = format_body(template, &result, Some("Release Build"));

        assert!(formatted.contains("Release Build"));
    }
}
