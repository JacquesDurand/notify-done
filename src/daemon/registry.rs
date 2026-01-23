use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;

use super::protocol::{HistoryEntry, TaskInfo};
use crate::config::history_path;

/// Registry for tracking running tasks and history
pub struct TaskRegistry {
    tasks: RwLock<HashMap<String, TaskInfo>>,
    history: RwLock<Vec<HistoryEntry>>,
    history_path: Option<PathBuf>,
    max_history: usize,
}

impl TaskRegistry {
    pub fn new(max_history: usize) -> Arc<Self> {
        let hist_path = history_path();

        // Load history from disk before creating the registry
        let initial_history = hist_path
            .as_ref()
            .filter(|p| p.exists())
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|content| serde_json::from_str::<Vec<HistoryEntry>>(&content).ok())
            .unwrap_or_default();

        Arc::new(Self {
            tasks: RwLock::new(HashMap::new()),
            history: RwLock::new(initial_history),
            history_path: hist_path,
            max_history,
        })
    }

    /// Register a new task
    pub async fn register(&self, info: TaskInfo) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(info.id.clone(), info);
    }

    /// Complete a task and move to history
    pub async fn complete(&self, id: &str, exit_code: i32, duration: Duration) -> Option<TaskInfo> {
        let task = {
            let mut tasks = self.tasks.write().await;
            tasks.remove(id)
        };

        if let Some(ref task) = task {
            let entry = HistoryEntry {
                command: task.command.clone(),
                name: task.name.clone(),
                exit_code,
                duration,
                completed_at: SystemTime::now(),
                success: exit_code == 0,
            };

            let mut history = self.history.write().await;
            history.push(entry);

            // Trim history if needed
            while history.len() > self.max_history {
                history.remove(0);
            }

            // Save to disk
            self.save_history(&history);
        }

        task
    }

    /// List all running tasks
    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Get task history
    pub async fn get_history(&self, count: usize) -> Vec<HistoryEntry> {
        let history = self.history.read().await;
        history.iter().rev().take(count).cloned().collect()
    }

    /// Remove a task (e.g., if process died)
    #[allow(dead_code)]
    pub async fn remove(&self, id: &str) -> Option<TaskInfo> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id)
    }

    fn save_history(&self, history: &[HistoryEntry]) {
        if let Some(ref path) = self.history_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(history) {
                let _ = fs::write(path, json);
            }
        }
    }
}
