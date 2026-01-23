use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::watch;

use super::protocol::{
    deserialize_request, serialize_response, Request, Response, TaskInfo,
};
use super::registry::TaskRegistry;
use crate::config::socket_path;

/// Start the daemon server
pub async fn run_server() -> Result<()> {
    let socket = socket_path().context("Could not determine socket path")?;

    // Remove existing socket if present
    if socket.exists() {
        std::fs::remove_file(&socket).ok();
    }

    let listener = UnixListener::bind(&socket)
        .with_context(|| format!("Failed to bind to socket: {}", socket.display()))?;

    eprintln!("Daemon started, listening on {}", socket.display());

    let registry = TaskRegistry::new(100);
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let registry = Arc::clone(&registry);
                        let shutdown_tx = shutdown_tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, registry, shutdown_tx).await {
                                eprintln!("Client error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    eprintln!("Shutdown signal received");
                    break;
                }
            }
        }
    }

    // Cleanup socket
    std::fs::remove_file(&socket).ok();
    eprintln!("Daemon stopped");

    Ok(())
}

async fn handle_client(
    stream: tokio::net::UnixStream,
    registry: Arc<TaskRegistry>,
    shutdown_tx: watch::Sender<bool>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let response = if let Some(request) = deserialize_request(line.trim().as_bytes()) {
            match request {
                Request::RegisterTask { id, command, name, pid } => {
                    let info = TaskInfo {
                        id,
                        command,
                        name,
                        pid,
                        started_at: std::time::SystemTime::now(),
                    };
                    registry.register(info).await;
                    Response::Ok
                }

                Request::CompleteTask { id, exit_code, duration_secs } => {
                    registry
                        .complete(&id, exit_code, Duration::from_secs(duration_secs))
                        .await;
                    Response::Ok
                }

                Request::ListTasks => {
                    let tasks = registry.list_tasks().await;
                    Response::Tasks(tasks)
                }

                Request::GetHistory { count } => {
                    let history = registry.get_history(count).await;
                    Response::History(history)
                }

                Request::Ping => Response::Pong,

                Request::Shutdown => {
                    let _ = shutdown_tx.send(true);
                    Response::Ok
                }
            }
        } else {
            Response::Error("Invalid request".to_string())
        };

        let data = serialize_response(&response);
        writer.write_all(&data).await?;
        writer.flush().await?;

        line.clear();
    }

    Ok(())
}

/// Check if daemon is running by attempting to connect
pub fn is_daemon_running() -> bool {
    use std::os::unix::net::UnixStream;

    let Some(socket) = socket_path() else {
        return false;
    };

    UnixStream::connect(&socket).is_ok()
}
