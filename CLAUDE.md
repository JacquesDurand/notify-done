# notify-done

A system-wide process notification daemon using eBPF.

## Architecture

- **eBPF-based**: Uses kernel tracepoints to monitor process execution system-wide
- **Root daemon**: Runs as systemd service with eBPF capabilities
- **User notifications**: Discovers user D-Bus sessions to send notifications

## Project Structure

```
notify-done/
├── xtask/                    # Build tooling (cargo xtask)
├── notify-done-ebpf/         # eBPF programs (no_std)
├── notify-done-common/       # Shared types between eBPF and userspace
├── notify-done-daemon/       # Root systemd daemon
├── nd/                       # CLI tool
└── systemd/                  # Service file
```

## Key Files

- `notify-done-ebpf/src/main.rs` - eBPF tracepoint handlers
- `notify-done-common/src/lib.rs` - Shared event structs
- `notify-done-daemon/src/main.rs` - Daemon entry point
- `notify-done-daemon/src/ebpf_loader.rs` - eBPF program loading
- `notify-done-daemon/src/process_tracker.rs` - Process state tracking
- `notify-done-daemon/src/notifier.rs` - D-Bus notification sending
- `nd/src/main.rs` - CLI commands

## Commands

```bash
# Build
cargo xtask build-ebpf     # Build eBPF programs
cargo xtask build          # Build everything

# CLI
nd status                  # Show daemon status
nd list                    # List tracked processes
nd history                 # Show notification history
nd config show/init        # Manage configuration
nd test                    # Send test notification
nd run -- <command>        # Wrapper mode (explicit tracking)

# Service
sudo systemctl start notify-done
journalctl -u notify-done -f
```

## Requirements

- Linux kernel 5.8+ (ring buffer support)
- Rust nightly (for eBPF build)
- Root privileges for daemon
