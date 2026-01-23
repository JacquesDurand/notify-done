# notify-done

A Rust CLI that notifies you when long-running tasks complete.

## Architecture

- **Wrapper-first**: `nd -- <command>` runs command and notifies on completion
- **Optional daemon**: For tracking multiple tasks and history

## Key Files

- `src/cli/args.rs` - Clap argument definitions
- `src/executor/runner.rs` - Command execution with signal forwarding
- `src/notification/builder.rs` - Desktop notification via notify-rust
- `src/daemon/server.rs` - Tokio Unix socket server
- `src/config/schema.rs` - Configuration structure

## Commands

nd --            # Run and notify
nd -t 30 --      # Only notify if > 30s
nd config --init          # Create config file
nd daemon start/stop      # Manage daemon
nd list / nd history      # View tasks (daemon mode)