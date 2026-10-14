use crate::error::{IpcError, LockError};
use crate::transport::path::{get_lockfile_path, get_socket_path};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLockMetadata {
    pub pid: u32,
    pub project_path: PathBuf,
    pub project_id: Uuid,
    pub socket_path: PathBuf,
    pub port: Option<u16>,
    pub created_at: u64, // Unix timestamp in seconds
    pub app_version: String,
    pub protocol_version: u32,
}

pub struct ProjectLock {
    pub lock_path: PathBuf,
    pub socket_path: PathBuf,
}

impl ProjectLock {
    /// Attempts to acquire an exclusive lockfile for the project.
    /// If a stale lockfile from a crashed process is detected, it is cleaned up automatically.
    pub fn acquire(project_path: &Path, project_id: Uuid) -> Result<Self, IpcError> {
        Self::acquire_with_port(project_path, project_id, None)
    }

    pub fn acquire_with_port(project_path: &Path, project_id: Uuid, port: Option<u16>) -> Result<Self, IpcError> {
        let lock_path = get_lockfile_path(project_path)?;
        let socket_path = get_socket_path(project_path)?;

        if lock_path.exists() {
            match Self::read_lock(&lock_path) {
                Ok(meta) => {
                    if is_process_alive(meta.pid) {
                        return Err(IpcError::Lock(LockError::AlreadyRunning {
                            pid: meta.pid,
                            project_path: project_path.to_path_buf(),
                        }));
                    } else {
                        tracing::warn!(
                            stale_pid = meta.pid,
                            "Detected stale lockfile from dead editor process. Cleaning up."
                        );
                        Self::cleanup_files(&lock_path, &socket_path);
                    }
                }
                Err(err) => {
                    tracing::warn!("Corrupted lockfile detected: {}. Cleaning up.", err);
                    Self::cleanup_files(&lock_path, &socket_path);
                }
            }
        }

        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }

        let current_pid = std::process::id();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let metadata = ProjectLockMetadata {
            pid: current_pid,
            project_path: project_path.canonicalize().unwrap_or_else(|_| project_path.to_path_buf()),
            project_id,
            socket_path: socket_path.clone(),
            port,
            created_at: now,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: 1,
        };

        let serialized = serde_json::to_string_pretty(&metadata)
            .map_err(|e| IpcError::Serialization(e.to_string()))?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|e| IpcError::Lock(LockError::AcquisitionFailed(e.to_string())))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o600));
        }

        file.write_all(serialized.as_bytes()).map_err(|e| IpcError::Io {
            action: "writing project lockfile",
            path: lock_path.clone(),
            source: e,
        })?;
        file.flush().map_err(|e| IpcError::Io {
            action: "flushing project lockfile",
            path: lock_path.clone(),
            source: e,
        })?;

        Ok(Self {
            lock_path,
            socket_path,
        })
    }

    pub fn read_lock(lock_path: &Path) -> Result<ProjectLockMetadata, IpcError> {
        let mut file = File::open(lock_path).map_err(|e| IpcError::Io {
            action: "opening lockfile",
            path: lock_path.to_path_buf(),
            source: e,
        })?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| IpcError::Io {
            action: "reading lockfile",
            path: lock_path.to_path_buf(),
            source: e,
        })?;
        let meta: ProjectLockMetadata = serde_json::from_str(&content)
            .map_err(|e| IpcError::Serialization(e.to_string()))?;
        Ok(meta)
    }

    pub fn cleanup_files(lock_path: &Path, socket_path: &Path) {
        if lock_path.exists() {
            let _ = fs::remove_file(lock_path);
        }
        if socket_path.exists() {
            let _ = fs::remove_file(socket_path);
        }
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        tracing::debug!("Releasing project lock at {:?}", self.lock_path);
        Self::cleanup_files(&self.lock_path, &self.socket_path);
    }
}

/// Robust cross-platform check whether a process with the given PID is actively running.
pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if res == 0 {
            return true;
        }
        let err = std::io::Error::last_os_error().raw_os_error();
        err == Some(libc::EPERM)
    }

    #[cfg(not(unix))]
    {
        // Fallback for non-unix
        true
    }
}
