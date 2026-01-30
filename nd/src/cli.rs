use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nd")]
#[command(about = "notify-done - Get notified when long-running commands complete")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show daemon status
    Status,

    /// List currently tracked processes
    List,

    /// Show notification history
    History {
        /// Number of entries to show
        #[arg(short, long, default_value = "20")]
        count: usize,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Send a test notification
    Test,

    /// Run a command and notify when it completes (wrapper mode)
    Run {
        /// Minimum duration in seconds before notifying
        #[arg(short = 't', long, default_value = "10")]
        threshold: u64,

        /// The command to run
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },

    /// Watch live events from the daemon
    Watch,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Initialize user configuration file
    Init,

    /// Set notification threshold
    Threshold {
        /// Threshold in seconds
        seconds: u64,
    },

    /// Add a pattern to ignore
    Ignore {
        /// Pattern to ignore (e.g., "npm*")
        pattern: String,
    },

    /// Add a pattern to always notify
    Always {
        /// Pattern to always notify for
        pattern: String,
    },

    /// Disable notifications
    Disable,

    /// Enable notifications
    Enable,
}
