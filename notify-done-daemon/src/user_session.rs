use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Information about a user's graphical session
#[derive(Debug, Clone)]
pub struct UserSession {
    pub uid: u32,
    pub username: String,
    pub display: Option<String>,
    pub dbus_address: String,
    pub session_type: SessionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    X11,
    Wayland,
    Unknown,
}

/// Discovers and caches user session information
pub struct SessionDiscovery {
    /// Cached sessions by UID
    sessions: HashMap<u32, UserSession>,
    /// Username cache by UID
    usernames: HashMap<u32, String>,
}

impl SessionDiscovery {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            usernames: HashMap::new(),
        }
    }

    /// Get or discover session for a user
    pub fn get_session(&mut self, uid: u32) -> Option<&UserSession> {
        // Check cache first
        if self.sessions.contains_key(&uid) {
            return self.sessions.get(&uid);
        }

        // Try to discover session
        match self.discover_session(uid) {
            Ok(session) => {
                log::debug!("Discovered session for uid {}: {:?}", uid, session);
                self.sessions.insert(uid, session);
                return self.sessions.get(&uid);
            }
            Err(e) => {
                log::debug!("Failed to discover session for uid {}: {}", uid, e);
                return None;
            }
        }
    }

    /// Discover session for a specific UID
    fn discover_session(&mut self, uid: u32) -> Result<UserSession> {
        let username = self.get_username(uid)?;

        // Use standard D-Bus socket location - don't check existence since
        // we may not have permission to read /run/user/{uid} due to systemd
        // security restrictions, but systemd-run can still access it
        let dbus_address = format!("unix:path=/run/user/{}/bus", uid);
        log::debug!("Using D-Bus address: {}", dbus_address);

        // Try to determine session type from loginctl
        let session_type = self.detect_session_type(uid);

        // Try to get DISPLAY for X11
        let display = if session_type == SessionType::X11 {
            self.get_display_for_user(uid)
        } else {
            None
        };

        Ok(UserSession {
            uid,
            username,
            display,
            dbus_address,
            session_type,
        })
    }

    /// Get username for a UID
    fn get_username(&mut self, uid: u32) -> Result<String> {
        if let Some(name) = self.usernames.get(&uid) {
            return Ok(name.clone());
        }

        // Read from /etc/passwd
        let passwd = std::fs::read_to_string("/etc/passwd")?;
        for line in passwd.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(entry_uid) = parts[2].parse::<u32>() {
                    if entry_uid == uid {
                        let username = parts[0].to_string();
                        self.usernames.insert(uid, username.clone());
                        return Ok(username);
                    }
                }
            }
        }

        anyhow::bail!("Username not found for uid {}", uid)
    }

    /// Detect session type using loginctl
    fn detect_session_type(&self, uid: u32) -> SessionType {
        let output = Command::new("loginctl")
            .args(["list-sessions", "--no-legend"])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return SessionType::Unknown,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                // Format: SESSION UID USER SEAT TTY
                if let Ok(session_uid) = parts[1].parse::<u32>() {
                    if session_uid == uid {
                        let session_id = parts[0];
                        return self.get_session_type_by_id(session_id);
                    }
                }
            }
        }

        SessionType::Unknown
    }

    /// Get session type by session ID
    fn get_session_type_by_id(&self, session_id: &str) -> SessionType {
        let output = Command::new("loginctl")
            .args(["show-session", session_id, "-p", "Type"])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return SessionType::Unknown,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("x11") {
            SessionType::X11
        } else if stdout.contains("wayland") {
            SessionType::Wayland
        } else {
            SessionType::Unknown
        }
    }

    /// Try to get DISPLAY variable for X11 sessions
    fn get_display_for_user(&self, uid: u32) -> Option<String> {
        // Common display values to try
        let displays = [":0", ":1"];

        for display in displays {
            let xauthority = PathBuf::from(format!("/run/user/{}/gdm/Xauthority", uid));
            if xauthority.exists() {
                return Some(display.to_string());
            }

            // Also check home directory
            if let Ok(username) = self.get_username_sync(uid) {
                let home_xauth = PathBuf::from(format!("/home/{}/.Xauthority", username));
                if home_xauth.exists() {
                    return Some(display.to_string());
                }
            }
        }

        // Default to :0 if we can't determine
        Some(":0".to_string())
    }

    /// Synchronous username lookup (for use in non-mutable context)
    fn get_username_sync(&self, uid: u32) -> Result<String> {
        if let Some(name) = self.usernames.get(&uid) {
            return Ok(name.clone());
        }

        let passwd = std::fs::read_to_string("/etc/passwd")?;
        for line in passwd.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(entry_uid) = parts[2].parse::<u32>() {
                    if entry_uid == uid {
                        return Ok(parts[0].to_string());
                    }
                }
            }
        }

        anyhow::bail!("Username not found for uid {}", uid)
    }

    /// Clear all cached sessions
    pub fn clear_cache(&mut self) {
        self.sessions.clear();
    }
}

impl Default for SessionDiscovery {
    fn default() -> Self {
        Self::new()
    }
}
