use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Request from client to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Register a new running task
    RegisterTask {
        id: String,
        command: String,
        name: Option<String>,
        pid: u32,
    },
    /// Mark a task as completed
    CompleteTask {
        id: String,
        exit_code: i32,
        duration_secs: u64,
    },
    /// List all running tasks
    ListTasks,
    /// Get task history
    GetHistory { count: usize },
    /// Ping to check if daemon is alive
    Ping,
    /// Shutdown the daemon
    Shutdown,
}

/// Response from daemon to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Acknowledgment
    Ok,
    /// Error response
    Error(String),
    /// List of running tasks
    Tasks(Vec<TaskInfo>),
    /// Task history
    History(Vec<HistoryEntry>),
    /// Pong response
    Pong,
}

/// Information about a running task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub command: String,
    pub name: Option<String>,
    pub pid: u32,
    pub started_at: SystemTime,
}

/// Entry in task history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub name: Option<String>,
    pub exit_code: i32,
    pub duration: Duration,
    pub completed_at: SystemTime,
    pub success: bool,
}

impl TaskInfo {
    pub fn running_duration(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.started_at)
            .unwrap_or_default()
    }
}

/// Serialize a request to JSON bytes with newline delimiter
pub fn serialize_request(req: &Request) -> Vec<u8> {
    let mut json = serde_json::to_vec(req).unwrap_or_default();
    json.push(b'\n');
    json
}

/// Serialize a response to JSON bytes with newline delimiter
pub fn serialize_response(resp: &Response) -> Vec<u8> {
    let mut json = serde_json::to_vec(resp).unwrap_or_default();
    json.push(b'\n');
    json
}

/// Deserialize a request from JSON bytes
pub fn deserialize_request(data: &[u8]) -> Option<Request> {
    serde_json::from_slice(data).ok()
}

/// Deserialize a response from JSON bytes
pub fn deserialize_response(data: &[u8]) -> Option<Response> {
    serde_json::from_slice(data).ok()
}
