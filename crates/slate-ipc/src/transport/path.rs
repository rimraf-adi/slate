use crate::error::IpcError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Computes a deterministic 16-hex-character hash from a canonical project path.
pub fn compute_project_hash(project_path: &Path) -> Result<String, IpcError> {
    let canonical = project_path.canonicalize().map_err(|e| IpcError::InvalidProjectPath {
        path: project_path.to_path_buf(),
        source: e,
    })?;

    let path_str = canonical.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let result = hasher.finalize();
    Ok(hex::encode(&result[..8])) // 8 bytes = 16 hex chars
}

/// Resolves the runtime directory for socket and lockfile storage.
pub fn resolve_runtime_dir() -> Result<PathBuf, IpcError> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir).join("slate");
            let _ = std::fs::create_dir_all(&path);
            return Ok(path);
        }
        let uid = unsafe { libc::getuid() };
        let path = PathBuf::from(format!("/tmp/slate-{}", uid));
        let _ = std::fs::create_dir_all(&path);
        Ok(path)
    }

    #[cfg(target_os = "macos")]
    {
        let uid = unsafe { libc::getuid() };
        let path = PathBuf::from(format!("/tmp/slate-{}", uid));
        let _ = std::fs::create_dir_all(&path);
        Ok(path)
    }

    #[cfg(windows)]
    {
        let local_app_data =
            std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        let path = PathBuf::from(local_app_data).join("Slate").join("run");
        let _ = std::fs::create_dir_all(&path);
        Ok(path)
    }
}

/// Returns the exact socket address or named pipe path for a given project path.
pub fn get_socket_path(project_path: &Path) -> Result<PathBuf, IpcError> {
    let hash = compute_project_hash(project_path)?;

    #[cfg(unix)]
    {
        let runtime_dir = resolve_runtime_dir()?;
        Ok(runtime_dir.join(format!("slate-{}.sock", hash)))
    }

    #[cfg(windows)]
    {
        Ok(PathBuf::from(format!(r"\\.\pipe\slate-{}", hash)))
    }
}

/// Returns the lockfile path associated with a given project path.
pub fn get_lockfile_path(project_path: &Path) -> Result<PathBuf, IpcError> {
    let hash = compute_project_hash(project_path)?;
    let runtime_dir = resolve_runtime_dir()?;
    Ok(runtime_dir.join(format!("slate-{}.lock", hash)))
}
