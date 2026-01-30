use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;

use crate::process_tracker::CompletedProcess;
use crate::user_session::{SessionType, UserSession};

/// Sends desktop notifications to users
pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }

    /// Send a notification for a completed process
    pub async fn notify(&self, session: &UserSession, process: &CompletedProcess) -> Result<()> {
        let summary = format!("Command completed: {}", process.comm);
        let body = self.format_body(process);

        // Use notify-send via sudo to send notification as the user
        self.send_notify_send(session, &summary, &body)
    }

    /// Format the notification body
    fn format_body(&self, process: &CompletedProcess) -> String {
        let duration = format_duration(process.duration);
        let status = if process.exit_code == 0 {
            "succeeded"
        } else {
            "failed"
        };

        format!(
            "{}\nDuration: {}\nExit code: {}",
            status, duration, process.exit_code
        )
    }

    /// Send notification using notify-send command as the target user
    fn send_notify_send(&self, session: &UserSession, summary: &str, body: &str) -> Result<()> {
        // Build environment variables
        let xdg_runtime_dir = format!("/run/user/{}", session.uid);

        let mut env_vars = vec![
            format!("XDG_RUNTIME_DIR={}", xdg_runtime_dir),
            format!("DBUS_SESSION_BUS_ADDRESS={}", session.dbus_address),
        ];

        if let Some(display) = &session.display {
            env_vars.push(format!("DISPLAY={}", display));
        }

        if session.session_type == SessionType::Wayland {
            env_vars.push(format!("WAYLAND_DISPLAY=/run/user/{}/wayland-0", session.uid));
        }

        // Use systemd-run to run in the user's systemd scope
        // This properly inherits the user's session environment
        let mut cmd = Command::new("systemd-run");
        cmd.args([
            "--user",
            "--machine", &format!("{}@.host", session.username),
            "--quiet",
            "--pipe",
            "--wait",
            "--collect",
        ]);
        for env_var in &env_vars {
            cmd.args(["--setenv", env_var]);
        }
        cmd.args(["notify-send", "--app-name=notify-done", summary, body]);

        let output = cmd.output().context("Failed to run systemd-run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("notify-send failed (exit {}): {}", output.status, stderr);
        }

        Ok(())
    }

    /// Send a test notification
    pub async fn send_test(&self, session: &UserSession) -> Result<()> {
        let summary = "notify-done test";
        let body = "If you see this, notifications are working!";
        self.send_notify_send(session, summary, body)
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Format duration in human-readable form
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}
