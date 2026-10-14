use crate::error::{IpcError, LockError};
use crate::transport::lock::{is_process_alive, ProjectLock};
use crate::transport::path::get_lockfile_path;
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use std::path::Path;
use std::time::Duration;

/// Discovers a running Slate editor instance for `project_path` and connects via WebSocket.
pub async fn discover_and_connect(
    project_path: &Path,
    timeout: Duration,
) -> Result<WsClient, IpcError> {
    let lock_path = get_lockfile_path(project_path)?;
    let start = std::time::Instant::now();

    loop {
        if lock_path.exists() {
            if let Ok(meta) = ProjectLock::read_lock(&lock_path) {
                if is_process_alive(meta.pid) {
                    if let Some(port) = meta.port {
                        let url = format!("ws://127.0.0.1:{}", port);
                        if let Ok(client) = WsClientBuilder::default().build(&url).await {
                            return Ok(client);
                        }
                    }
                }
            }
        }

        if start.elapsed() >= timeout {
            return Err(IpcError::Lock(LockError::NotFound(project_path.to_path_buf())));
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
