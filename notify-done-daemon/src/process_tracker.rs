use std::collections::HashMap;
use std::time::{Duration, Instant};

use notify_done_common::{ProcessExecEvent, ProcessExitEvent};

/// Information about a tracked process
#[derive(Debug, Clone)]
pub struct TrackedProcess {
    pub pid: u32,
    pub tgid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub comm: String,
    pub filename: String,
    pub start_time: Instant,
    pub start_timestamp_ns: u64,
}

/// A process that has completed execution
#[derive(Debug, Clone)]
pub struct CompletedProcess {
    pub pid: u32,
    pub tgid: u32,
    pub uid: u32,
    pub comm: String,
    pub filename: String,
    pub exit_code: i32,
    pub duration: Duration,
}

/// Tracks active processes and computes durations on exit
pub struct ProcessTracker {
    /// Active processes indexed by TGID
    processes: HashMap<u32, TrackedProcess>,
    /// Completed processes (recent history)
    history: Vec<CompletedProcess>,
    /// Maximum history size
    max_history: usize,
}

impl ProcessTracker {
    pub fn new(max_history: usize) -> Self {
        Self {
            processes: HashMap::new(),
            history: Vec::new(),
            max_history,
        }
    }

    /// Handle a process exec event
    pub fn on_exec(&mut self, event: &ProcessExecEvent) {
        let tracked = TrackedProcess {
            pid: event.pid,
            tgid: event.tgid,
            ppid: event.ppid,
            uid: event.uid,
            comm: event.comm_str().to_string(),
            filename: event.filename_str().to_string(),
            start_time: Instant::now(),
            start_timestamp_ns: event.timestamp_ns,
        };

        log::debug!(
            "Tracking process: pid={} comm={} filename={}",
            tracked.pid,
            tracked.comm,
            tracked.filename
        );

        self.processes.insert(event.tgid, tracked);
    }

    /// Handle a process exit event, returns CompletedProcess if we were tracking it
    pub fn on_exit(&mut self, event: &ProcessExitEvent) -> Option<CompletedProcess> {
        let tracked = self.processes.remove(&event.tgid)?;

        // Calculate duration using kernel timestamps if available,
        // otherwise fall back to userspace timing
        let duration = if event.timestamp_ns > tracked.start_timestamp_ns {
            Duration::from_nanos(event.timestamp_ns - tracked.start_timestamp_ns)
        } else {
            tracked.start_time.elapsed()
        };

        let completed = CompletedProcess {
            pid: tracked.pid,
            tgid: tracked.tgid,
            uid: tracked.uid,
            comm: tracked.comm,
            filename: tracked.filename,
            exit_code: event.exit_code,
            duration,
        };

        log::debug!(
            "Process completed: pid={} comm={} duration={:?} exit_code={}",
            completed.pid,
            completed.comm,
            completed.duration,
            completed.exit_code
        );

        // Add to history
        self.history.push(completed.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        Some(completed)
    }

    /// Get list of currently tracked processes
    pub fn active_processes(&self) -> impl Iterator<Item = &TrackedProcess> {
        self.processes.values()
    }

    /// Get process history
    pub fn history(&self) -> &[CompletedProcess] {
        &self.history
    }

    /// Get count of active processes
    pub fn active_count(&self) -> usize {
        self.processes.len()
    }

    /// Clean up stale processes (those that have been running for too long without exit)
    pub fn cleanup_stale(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.processes.retain(|_, p| {
            let age = now.duration_since(p.start_time);
            if age > max_age {
                log::warn!(
                    "Cleaning up stale process: pid={} comm={} age={:?}",
                    p.pid,
                    p.comm,
                    age
                );
                false
            } else {
                true
            }
        });
    }
}
