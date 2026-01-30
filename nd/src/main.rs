mod cli;

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};

use cli::{Cli, Commands, ConfigAction};

/// User configuration (same structure as daemon)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UserConfig {
    #[serde(default)]
    threshold_seconds: Option<u64>,
    #[serde(default)]
    ignore_patterns: Vec<String>,
    #[serde(default)]
    always_notify: Vec<String>,
    #[serde(default)]
    disabled: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cmd_status(),
        Commands::List => cmd_list(),
        Commands::History { count } => cmd_history(count),
        Commands::Config { action } => cmd_config(action),
        Commands::Test => cmd_test(),
        Commands::Run { threshold, command } => cmd_run(threshold, command),
        Commands::Watch => cmd_watch(),
    }
}

fn cmd_status() -> Result<()> {
    // Check if daemon is running
    let output = Command::new("systemctl")
        .args(["is-active", "notify-done"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            println!("Daemon status: running");

            // Get more info
            let info = Command::new("systemctl")
                .args(["status", "notify-done", "--no-pager", "-n", "0"])
                .output()?;

            let stdout = String::from_utf8_lossy(&info.stdout);
            for line in stdout.lines() {
                if line.contains("Active:") || line.contains("Main PID:") {
                    println!("{}", line.trim());
                }
            }
        }
        _ => {
            println!("Daemon status: not running");
            println!("\nTo start the daemon:");
            println!("  sudo systemctl start notify-done");
        }
    }

    Ok(())
}

fn cmd_list() -> Result<()> {
    println!("Currently tracked processes:");
    println!("(Note: Full tracking requires the daemon to be running)");
    println!();

    // For now, show processes belonging to current user that might be tracked
    let uid = unsafe { libc::getuid() };
    let output = Command::new("ps")
        .args([
            "-u",
            &uid.to_string(),
            "-o",
            "pid,etime,comm",
            "--no-headers",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{:>8} {:>12} COMMAND", "PID", "ELAPSED");
    println!("{:-<8} {:-<12} {:-<20}", "", "", "");

    for line in stdout.lines() {
        println!("{}", line);
    }

    Ok(())
}

fn cmd_history(_count: usize) -> Result<()> {
    // History is stored by the daemon
    // For now, we can check a local history file if it exists
    let history_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("notify-done")
        .join("history.json");

    if history_path.exists() {
        let content = std::fs::read_to_string(&history_path)?;
        println!("{}", content);
    } else {
        println!("No history available.");
        println!("History is recorded when the daemon is running.");
    }

    Ok(())
}

fn cmd_config(action: ConfigAction) -> Result<()> {
    let config_path = user_config_path()?;

    match action {
        ConfigAction::Show => {
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                println!("User config ({}):\n", config_path.display());
                println!("{}", content);
            } else {
                println!("No user config found at {}", config_path.display());
                println!("\nUsing system defaults. Run 'nd config init' to create a user config.");
            }

            // Also show system config if readable
            let system_path = PathBuf::from("/etc/notify-done/config.toml");
            if system_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&system_path) {
                    println!("\nSystem config ({}):\n", system_path.display());
                    println!("{}", content);
                }
            }
        }

        ConfigAction::Init => {
            if config_path.exists() {
                println!("Config already exists at {}", config_path.display());
                return Ok(());
            }

            std::fs::create_dir_all(config_path.parent().unwrap())?;

            let default_config = UserConfig {
                threshold_seconds: Some(10),
                ignore_patterns: vec![],
                always_notify: vec![],
                disabled: false,
            };

            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, content)?;

            println!("Created config at {}", config_path.display());
        }

        ConfigAction::Threshold { seconds } => {
            let mut config = load_or_create_config(&config_path)?;
            config.threshold_seconds = Some(seconds);
            save_config(&config_path, &config)?;
            println!("Set threshold to {} seconds", seconds);
        }

        ConfigAction::Ignore { pattern } => {
            let mut config = load_or_create_config(&config_path)?;
            if !config.ignore_patterns.contains(&pattern) {
                config.ignore_patterns.push(pattern.clone());
                save_config(&config_path, &config)?;
                println!("Added '{}' to ignore patterns", pattern);
            } else {
                println!("'{}' is already in ignore patterns", pattern);
            }
        }

        ConfigAction::Always { pattern } => {
            let mut config = load_or_create_config(&config_path)?;
            if !config.always_notify.contains(&pattern) {
                config.always_notify.push(pattern.clone());
                save_config(&config_path, &config)?;
                println!("Added '{}' to always-notify patterns", pattern);
            } else {
                println!("'{}' is already in always-notify patterns", pattern);
            }
        }

        ConfigAction::Disable => {
            let mut config = load_or_create_config(&config_path)?;
            config.disabled = true;
            save_config(&config_path, &config)?;
            println!("Notifications disabled");
        }

        ConfigAction::Enable => {
            let mut config = load_or_create_config(&config_path)?;
            config.disabled = false;
            save_config(&config_path, &config)?;
            println!("Notifications enabled");
        }
    }

    Ok(())
}

fn cmd_test() -> Result<()> {
    println!("Sending test notification...");

    Notification::new()
        .summary("notify-done test")
        .body("If you see this, notifications are working!")
        .icon("dialog-information")
        .appname("notify-done")
        .show()
        .context("Failed to send notification")?;

    println!("Notification sent!");
    Ok(())
}

fn cmd_run(threshold: u64, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command specified");
    }

    let start = Instant::now();

    // Run the command
    let status = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute: {}", command[0]))?;

    let duration = start.elapsed();
    let duration_secs = duration.as_secs();
    let exit_code = status.code().unwrap_or(-1);

    // Only notify if above threshold
    if duration_secs >= threshold {
        let status_str = if status.success() {
            "succeeded"
        } else {
            "failed"
        };

        let body = format!(
            "{}\nDuration: {}\nExit code: {}",
            status_str,
            format_duration(duration_secs),
            exit_code
        );

        Notification::new()
            .summary(&format!("Command completed: {}", command[0]))
            .body(&body)
            .icon("dialog-information")
            .appname("notify-done")
            .show()
            .ok(); // Don't fail if notification fails
    }

    // Exit with the same code as the command
    std::process::exit(exit_code);
}

fn cmd_watch() -> Result<()> {
    println!("Watching for events... (Ctrl+C to stop)");
    println!("(Note: This requires the daemon to be running with debug enabled)");
    println!();

    // Follow the journal for notify-done
    let mut child = Command::new("journalctl")
        .args(["-u", "notify-done", "-f", "-n", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to run journalctl")?;

    // Set up signal handler
    ctrlc::set_handler(move || {
        std::process::exit(0);
    })?;

    child.wait()?;
    Ok(())
}

fn user_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(config_dir.join("notify-done").join("config.toml"))
}

fn load_or_create_config(path: &PathBuf) -> Result<UserConfig> {
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    } else {
        std::fs::create_dir_all(path.parent().unwrap())?;
        Ok(UserConfig::default())
    }
}

fn save_config(path: &PathBuf, config: &UserConfig) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}
