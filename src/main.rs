mod cli;
mod config;
mod daemon;
mod executor;
mod notification;

use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Parser;
use uuid::Uuid;

use cli::{Cli, Commands, DaemonAction};
use config::{config_path, init_config, load_config};
use daemon::{is_daemon_running, run_server, DaemonClient};
use executor::{execute_command, format_duration};
use notification::{send_notification, should_notify};

fn main() -> ExitCode {
    // Check if we're running as daemon child
    if is_daemon_child() {
        if let Err(e) = daemon_main() {
            eprintln!("Daemon error: {:#}", e);
            return ExitCode::from(1);
        }
        return ExitCode::from(0);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Config { path, show, init }) => handle_config(path, show, init),
        Some(Commands::Daemon { action }) => handle_daemon(action),
        Some(Commands::List) => handle_list(),
        Some(Commands::History { count }) => handle_history(count),
        None => {
            if cli.command_args.is_empty() {
                eprintln!("Error: No command provided. Use -- followed by a command.");
                eprintln!("Example: nd -- sleep 5");
                return ExitCode::from(1);
            }
            handle_run(&cli)
        }
    };

    match result {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error: {:#}", e);
            ExitCode::from(1)
        }
    }
}

fn handle_config(show_path: bool, show: bool, init: bool) -> Result<i32> {
    if init {
        let path = init_config()?;
        println!("Created config file: {}", path.display());
        return Ok(0);
    }

    if show_path {
        if let Some(path) = config_path() {
            println!("{}", path.display());
        } else {
            bail!("Could not determine config path");
        }
        return Ok(0);
    }

    if show {
        let config = load_config()?;
        let toml = toml::to_string_pretty(&config)?;
        println!("{}", toml);
        return Ok(0);
    }

    // Default: show path and whether it exists
    if let Some(path) = config_path() {
        let exists = path.exists();
        println!("Config file: {}", path.display());
        println!("Exists: {}", exists);
        if !exists {
            println!("\nRun 'nd config --init' to create a default config file.");
        }
    } else {
        bail!("Could not determine config path");
    }

    Ok(0)
}

fn handle_daemon(action: DaemonAction) -> Result<i32> {
    match action {
        DaemonAction::Start => {
            if is_daemon_running() {
                println!("Daemon is already running.");
                return Ok(0);
            }

            // Fork to background
            #[cfg(unix)]
            {
                use std::process::Command;

                let exe = std::env::current_exe()?;
                Command::new(&exe)
                    .arg("daemon")
                    .arg("start")
                    .env("ND_DAEMON_CHILD", "1")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()?;

                println!("Daemon started in background.");
                return Ok(0);
            }

            #[cfg(not(unix))]
            {
                bail!("Daemon mode is only supported on Unix systems");
            }
        }

        DaemonAction::Stop => {
            if !is_daemon_running() {
                println!("Daemon is not running.");
                return Ok(0);
            }

            let mut client = DaemonClient::connect()?;
            client.shutdown()?;
            println!("Daemon stopped.");
            Ok(0)
        }

        DaemonAction::Status => {
            if is_daemon_running() {
                println!("Daemon is running.");
            } else {
                println!("Daemon is not running.");
            }
            Ok(0)
        }
    }
}

fn handle_list() -> Result<i32> {
    if !is_daemon_running() {
        println!("Daemon is not running. Start it with 'nd daemon start'.");
        return Ok(1);
    }

    let mut client = DaemonClient::connect()?;
    let tasks = client.list_tasks()?;

    if tasks.is_empty() {
        println!("No running tasks.");
    } else {
        println!("{:<36} {:<30} {:<10}", "ID", "Command", "Running");
        println!("{}", "-".repeat(76));
        for task in tasks {
            let duration = format_duration(task.running_duration());
            let display = task.name.as_deref().unwrap_or(&task.command);
            let truncated = if display.len() > 28 {
                format!("{}...", &display[..25])
            } else {
                display.to_string()
            };
            println!("{:<36} {:<30} {:<10}", task.id, truncated, duration);
        }
    }

    Ok(0)
}

fn handle_history(count: usize) -> Result<i32> {
    if !is_daemon_running() {
        println!("Daemon is not running. Start it with 'nd daemon start'.");
        return Ok(1);
    }

    let mut client = DaemonClient::connect()?;
    let history = client.get_history(count)?;

    if history.is_empty() {
        println!("No history.");
    } else {
        println!("{:<30} {:<10} {:<10} {:<6}", "Command", "Duration", "Exit", "Status");
        println!("{}", "-".repeat(56));
        for entry in history {
            let duration = format_duration(entry.duration);
            let display = entry.name.as_deref().unwrap_or(&entry.command);
            let truncated = if display.len() > 28 {
                format!("{}...", &display[..25])
            } else {
                display.to_string()
            };
            let status = if entry.success { "OK" } else { "FAIL" };
            println!(
                "{:<30} {:<10} {:<10} {:<6}",
                truncated, duration, entry.exit_code, status
            );
        }
    }

    Ok(0)
}

fn handle_run(cli: &Cli) -> Result<i32> {
    let config = load_config()?;

    // Generate a task ID for daemon registration
    let task_id = Uuid::new_v4().to_string();
    let command_str = cli.command_args.join(" ");

    // Try to register with daemon if running
    let daemon_client = if is_daemon_running() {
        match DaemonClient::connect() {
            Ok(mut client) => {
                let _ = client.register_task(
                    task_id.clone(),
                    command_str.clone(),
                    cli.name.clone(),
                    std::process::id(),
                );
                Some(client)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Execute the command
    let result = execute_command(&cli.command_args)?;

    // Notify daemon of completion
    if let Some(mut client) = daemon_client {
        let _ = client.complete_task(task_id, result.exit_code, result.duration.as_secs());
    }

    // Send notification if appropriate
    if should_notify(&config, &result, cli.threshold, cli.quiet) {
        if let Err(e) = send_notification(&config, &result, cli.name.as_deref()) {
            eprintln!("Warning: Failed to send notification: {}", e);
        }
    }

    Ok(result.exit_code)
}

// Entry point for daemon child process
#[tokio::main]
async fn daemon_main() -> Result<()> {
    run_server().await
}

// Check if running as daemon child
fn is_daemon_child() -> bool {
    std::env::var("ND_DAEMON_CHILD").is_ok()
}
