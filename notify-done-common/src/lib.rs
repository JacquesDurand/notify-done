#![cfg_attr(not(feature = "user"), no_std)]

/// Maximum length of the command name
pub const COMM_LEN: usize = 16;

/// Maximum length of the filename path
pub const FILENAME_LEN: usize = 256;

/// Event type discriminator
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    Exec = 1,
    Exit = 2,
}

/// Process execution event - sent when a process calls exec
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessExecEvent {
    /// Event type (always EventType::Exec)
    pub event_type: u8,
    /// Padding for alignment
    pub _pad: [u8; 3],
    /// Process ID (thread group leader)
    pub pid: u32,
    /// Thread group ID (same as pid for single-threaded)
    pub tgid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// User ID
    pub uid: u32,
    /// Timestamp in nanoseconds (monotonic)
    pub timestamp_ns: u64,
    /// Command name (first 16 bytes of executable name)
    pub comm: [u8; COMM_LEN],
    /// Full path to executable
    pub filename: [u8; FILENAME_LEN],
}

/// Process exit event - sent when a process exits
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessExitEvent {
    /// Event type (always EventType::Exit)
    pub event_type: u8,
    /// Padding for alignment
    pub _pad: [u8; 3],
    /// Process ID
    pub pid: u32,
    /// Thread group ID
    pub tgid: u32,
    /// User ID
    pub uid: u32,
    /// Exit code
    pub exit_code: i32,
    /// Timestamp in nanoseconds (monotonic)
    pub timestamp_ns: u64,
    /// Command name
    pub comm: [u8; COMM_LEN],
}

/// Filter configuration stored in eBPF map
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FilterConfig {
    /// Minimum UID to track (default: 1000)
    pub min_uid: u32,
    /// Whether to only track specific UIDs from TRACKED_UIDS map
    pub use_uid_whitelist: u8,
    /// Padding
    pub _pad: [u8; 3],
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_uid: 1000,
            use_uid_whitelist: 0,
            _pad: [0; 3],
        }
    }
}

// Implementations for userspace only
#[cfg(feature = "user")]
mod user_impl {
    use super::*;

    impl ProcessExecEvent {
        pub fn comm_str(&self) -> &str {
            let len = self.comm.iter().position(|&b| b == 0).unwrap_or(COMM_LEN);
            core::str::from_utf8(&self.comm[..len]).unwrap_or("<invalid>")
        }

        pub fn filename_str(&self) -> &str {
            let len = self
                .filename
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(FILENAME_LEN);
            core::str::from_utf8(&self.filename[..len]).unwrap_or("<invalid>")
        }
    }

    impl ProcessExitEvent {
        pub fn comm_str(&self) -> &str {
            let len = self.comm.iter().position(|&b| b == 0).unwrap_or(COMM_LEN);
            core::str::from_utf8(&self.comm[..len]).unwrap_or("<invalid>")
        }
    }

    impl core::fmt::Debug for ProcessExecEvent {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("ProcessExecEvent")
                .field("pid", &self.pid)
                .field("tgid", &self.tgid)
                .field("ppid", &self.ppid)
                .field("uid", &self.uid)
                .field("timestamp_ns", &self.timestamp_ns)
                .field("comm", &self.comm_str())
                .field("filename", &self.filename_str())
                .finish()
        }
    }

    impl core::fmt::Debug for ProcessExitEvent {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("ProcessExitEvent")
                .field("pid", &self.pid)
                .field("tgid", &self.tgid)
                .field("uid", &self.uid)
                .field("exit_code", &self.exit_code)
                .field("timestamp_ns", &self.timestamp_ns)
                .field("comm", &self.comm_str())
                .finish()
        }
    }
}

/// Size of the event ring buffer (256KB)
pub const RING_BUF_SIZE: u32 = 256 * 1024;

/// Maximum number of tracked PIDs
pub const MAX_TRACKED_PIDS: u32 = 65536;

/// Maximum number of tracked UIDs
pub const MAX_TRACKED_UIDS: u32 = 256;
