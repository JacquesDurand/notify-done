use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub command: String,
    pub exit_code: i32,
    pub duration: Duration,
    pub success: bool,
}

/// Execute a command and measure its duration
pub fn execute_command(args: &[String]) -> Result<ExecutionResult> {
    if args.is_empty() {
        bail!("No command provided");
    }

    let command_str = args.join(" ");
    let start = Instant::now();

    let status = run_with_signals(&args[0], &args[1..])?;

    let duration = start.elapsed();
    let exit_code = status.code().unwrap_or(1);

    Ok(ExecutionResult {
        command: command_str,
        exit_code,
        duration,
        success: status.success(),
    })
}

/// Run a command with signal forwarding
fn run_with_signals(program: &str, args: &[String]) -> Result<ExitStatus> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to execute command: {}", program))?;

    // Set up signal handling for the child process
    #[cfg(unix)]
    {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let child_pid = child.id();
        let signal_received = Arc::new(AtomicU32::new(0));
        let signal_clone = signal_received.clone();

        // Set up SIGINT handler
        ctrlc::set_handler(move || {
            signal_clone.store(libc::SIGINT as u32, Ordering::SeqCst);
            // Forward signal to child
            unsafe {
                libc::kill(child_pid as i32, libc::SIGINT);
            }
        })
        .ok();

        let status = child.wait().context("Failed to wait for command")?;

        // Check if we received a signal
        let sig = signal_received.load(Ordering::SeqCst);
        if sig != 0 && !status.success() {
            // Child was killed by signal, return its status
            return Ok(status);
        }

        Ok(status)
    }

    #[cfg(not(unix))]
    {
        child.wait().context("Failed to wait for command")
    }
}

/// Format a duration in a human-readable way
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs == 0 {
        let millis = duration.as_millis();
        return format!("{}ms", millis);
    }

    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}s", seconds));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
    }
}
