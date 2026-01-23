use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use anyhow::{bail, Context, Result};

use super::protocol::{
    deserialize_response, serialize_request, HistoryEntry, Request, Response, TaskInfo,
};
use crate::config::socket_path;

/// Client for communicating with the daemon
pub struct DaemonClient {
    stream: UnixStream,
}

impl DaemonClient {
    /// Connect to the daemon
    pub fn connect() -> Result<Self> {
        let socket = socket_path().context("Could not determine socket path")?;
        let stream = UnixStream::connect(&socket)
            .with_context(|| format!("Failed to connect to daemon at {}", socket.display()))?;

        Ok(Self { stream })
    }

    /// Send a request and receive a response
    fn request(&mut self, req: Request) -> Result<Response> {
        let data = serialize_request(&req);
        self.stream
            .write_all(&data)
            .context("Failed to send request")?;
        self.stream.flush()?;

        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .context("Failed to read response")?;

        deserialize_response(line.trim().as_bytes())
            .context("Failed to parse response")
    }

    /// Register a task with the daemon
    pub fn register_task(
        &mut self,
        id: String,
        command: String,
        name: Option<String>,
        pid: u32,
    ) -> Result<()> {
        let resp = self.request(Request::RegisterTask {
            id,
            command,
            name,
            pid,
        })?;

        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => bail!("Daemon error: {}", e),
            _ => bail!("Unexpected response"),
        }
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, id: String, exit_code: i32, duration_secs: u64) -> Result<()> {
        let resp = self.request(Request::CompleteTask {
            id,
            exit_code,
            duration_secs,
        })?;

        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => bail!("Daemon error: {}", e),
            _ => bail!("Unexpected response"),
        }
    }

    /// List running tasks
    pub fn list_tasks(&mut self) -> Result<Vec<TaskInfo>> {
        let resp = self.request(Request::ListTasks)?;

        match resp {
            Response::Tasks(tasks) => Ok(tasks),
            Response::Error(e) => bail!("Daemon error: {}", e),
            _ => bail!("Unexpected response"),
        }
    }

    /// Get task history
    pub fn get_history(&mut self, count: usize) -> Result<Vec<HistoryEntry>> {
        let resp = self.request(Request::GetHistory { count })?;

        match resp {
            Response::History(history) => Ok(history),
            Response::Error(e) => bail!("Daemon error: {}", e),
            _ => bail!("Unexpected response"),
        }
    }

    /// Check if daemon is alive
    #[allow(dead_code)]
    pub fn ping(&mut self) -> Result<bool> {
        let resp = self.request(Request::Ping)?;
        Ok(matches!(resp, Response::Pong))
    }

    /// Request daemon shutdown
    pub fn shutdown(&mut self) -> Result<()> {
        let resp = self.request(Request::Shutdown)?;

        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => bail!("Daemon error: {}", e),
            _ => bail!("Unexpected response"),
        }
    }
}
