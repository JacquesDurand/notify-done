use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use aya::maps::{MapData, RingBuf};

use notify_done_common::{EventType, ProcessExecEvent, ProcessExitEvent};

use crate::config::{DaemonConfig, EffectiveConfig, UserConfig};
use crate::notifier::Notifier;
use crate::process_tracker::{CompletedProcess, ProcessTracker};
use crate::user_session::SessionDiscovery;

/// Processes events from the eBPF ring buffer
pub struct EventProcessor {
    tracker: ProcessTracker,
    sessions: SessionDiscovery,
    notifier: Notifier,
    config: DaemonConfig,
    user_configs: HashMap<u32, Option<UserConfig>>,
}

impl EventProcessor {
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            tracker: ProcessTracker::new(1000), // Keep last 1000 completed processes
            sessions: SessionDiscovery::new(),
            notifier: Notifier::new(),
            config,
            user_configs: HashMap::new(),
        }
    }

    /// Process events from the ring buffer
    pub async fn process_events(&mut self, ring_buf: &mut RingBuf<MapData>) -> Result<()> {
        while let Some(event) = ring_buf.next() {
            let data: &[u8] = &event;
            if data.is_empty() {
                continue;
            }

            // First byte is event type
            let event_type = data[0];

            match event_type {
                t if t == EventType::Exec as u8 => {
                    if data.len() >= size_of::<ProcessExecEvent>() {
                        let exec_event: ProcessExecEvent =
                            unsafe { std::ptr::read_unaligned(data.as_ptr() as *const _) };
                        self.handle_exec(&exec_event);
                    }
                }
                t if t == EventType::Exit as u8 => {
                    if data.len() >= size_of::<ProcessExitEvent>() {
                        let exit_event: ProcessExitEvent =
                            unsafe { std::ptr::read_unaligned(data.as_ptr() as *const _) };
                        self.handle_exit(&exit_event).await;
                    }
                }
                _ => {
                    log::warn!("Unknown event type: {}", event_type);
                }
            }
        }

        Ok(())
    }

    /// Handle an exec event
    fn handle_exec(&mut self, event: &ProcessExecEvent) {
        if self.config.debug {
            log::debug!("Exec event: {:?}", event);
        }
        self.tracker.on_exec(event);
    }

    /// Handle an exit event
    async fn handle_exit(&mut self, event: &ProcessExitEvent) {
        if self.config.debug {
            log::debug!("Exit event: {:?}", event);
        }

        match self.tracker.on_exit(event) {
            Some(completed) => {
                self.maybe_notify(&completed).await;
            }
            None => {
                log::debug!(
                    "Exit event for untracked process: tgid={} comm={}",
                    event.tgid,
                    event.comm_str()
                );
            }
        }
    }

    /// Check if we should send a notification and do so if needed
    async fn maybe_notify(&mut self, process: &CompletedProcess) {
        // Get or load user config
        let user_config = self.get_user_config(process.uid);
        let effective = EffectiveConfig::new(&self.config, user_config.as_ref());

        // Check if we should notify
        let duration_secs = process.duration.as_secs();
        if !effective.should_notify(&process.comm, duration_secs) {
            log::debug!(
                "Skipping notification for {} (duration={}s, threshold={}s)",
                process.comm,
                duration_secs,
                effective.threshold_seconds
            );
            return;
        }

        // Get user session
        let session = match self.sessions.get_session(process.uid) {
            Some(s) => s.clone(),
            None => {
                log::warn!(
                    "No session found for uid {}, skipping notification",
                    process.uid
                );
                return;
            }
        };

        // Send notification
        if let Err(e) = self.notifier.notify(&session, process).await {
            log::error!(
                "Failed to send notification to user {}: {}",
                session.username,
                e
            );
        } else {
            log::info!(
                "Sent notification to {} for '{}' ({}s, exit {})",
                session.username,
                process.comm,
                duration_secs,
                process.exit_code
            );
        }
    }

    /// Get user config, loading if necessary
    fn get_user_config(&mut self, uid: u32) -> Option<UserConfig> {
        self.user_configs.entry(uid).or_insert_with(|| {
            let config = match UserConfig::load_for_uid(uid) {
                Ok(c) => c,
                Err(e) => {
                    log::debug!("Failed to load user config for uid {}: {}", uid, e);
                    None
                }
            };
            config
        });
        self.user_configs.get(&uid).cloned().flatten()
    }

    /// Clean up stale processes periodically
    pub fn cleanup(&mut self) {
        // Remove processes that have been running for more than 24 hours without exit
        self.tracker.cleanup_stale(Duration::from_secs(86400));

        // Refresh session cache
        self.sessions.clear_cache();

        // Clear user config cache
        self.user_configs.clear();
    }
}
