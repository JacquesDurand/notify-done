use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build tasks for notify-done")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the eBPF programs
    BuildEbpf {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build everything (eBPF + userspace)
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildEbpf { release } => build_ebpf(release),
        Commands::Build { release } => {
            build_ebpf(release)?;
            build_userspace(release)
        }
    }
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn build_ebpf(release: bool) -> Result<()> {
    let root = project_root();
    let ebpf_dir = root.join("notify-done-ebpf");

    // Set BPF target arch based on host architecture
    let bpf_arch = std::env::consts::ARCH;

    // Set RUSTFLAGS to include the bpf_target_arch cfg
    let rustflags = format!("--cfg bpf_target_arch=\"{}\"", bpf_arch);
    let existing_flags = std::env::var("RUSTFLAGS").unwrap_or_default();
    let combined_flags = if existing_flags.is_empty() {
        rustflags
    } else {
        format!("{} {}", existing_flags, rustflags)
    };

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&ebpf_dir)
        .env("RUSTFLAGS", &combined_flags)
        .args([
            "+nightly",
            "build",
            "--target",
            "bpfel-unknown-none",
            "-Z",
            "build-std=core",
        ]);

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("Failed to run cargo build for eBPF")?;

    if !status.success() {
        bail!("eBPF build failed");
    }

    // Copy the built binary to a known location
    let profile = if release { "release" } else { "debug" };
    let src = ebpf_dir
        .join("target")
        .join("bpfel-unknown-none")
        .join(profile)
        .join("notify-done-ebpf");
    let dst = root.join("target").join("ebpf").join("notify-done-ebpf");

    std::fs::create_dir_all(dst.parent().unwrap())?;
    if src.exists() {
        std::fs::copy(&src, &dst).context("Failed to copy eBPF binary")?;
        println!("eBPF program built: {}", dst.display());
    }

    Ok(())
}

fn build_userspace(release: bool) -> Result<()> {
    let root = project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root)
        .args(["build", "--workspace", "--exclude", "notify-done-ebpf"]);

    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .context("Failed to run cargo build for userspace")?;

    if !status.success() {
        bail!("Userspace build failed");
    }

    println!("Userspace programs built successfully");
    Ok(())
}
