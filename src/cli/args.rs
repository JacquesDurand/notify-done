use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "nd", about = "Notify when long-running tasks complete")]
#[command(version, author)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Only notify if command takes longer than this (seconds)
    #[arg(short = 't', long = "threshold", global = true)]
    pub threshold: Option<u64>,

    /// Custom name for task in notification
    #[arg(short = 'n', long = "name", global = true)]
    pub name: Option<String>,

    /// Suppress notification
    #[arg(short = 'q', long = "quiet", global = true)]
    pub quiet: bool,

    /// Command to execute (after --)
    #[arg(last = true)]
    pub command_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show or edit configuration
    Config {
        /// Show config file path
        #[arg(long)]
        path: bool,

        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Initialize default config file
        #[arg(long)]
        init: bool,
    },

    /// Daemon management
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// List running tasks (daemon mode)
    List,

    /// Show completed tasks (daemon mode)
    History {
        /// Number of entries to show
        #[arg(short = 'c', long, default_value = "10")]
        count: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum DaemonAction {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
}
