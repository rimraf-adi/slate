use jsonrpsee::types::ErrorObjectOwned;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("I/O error during {action} at {path:?}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid project path: {path:?} ({source})")]
    InvalidProjectPath {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Lock error: {0}")]
    Lock(#[from] LockError),

    #[error("Engine error: {0}")]
    Engine(String),

    #[error("JSON-RPC error ({code}): {message}")]
    Rpc {
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    },
}

#[derive(Error, Debug)]
pub enum LockError {
    #[error("Project is already open in another Slate editor process (PID: {pid}) at {project_path:?}")]
    AlreadyRunning { pid: u32, project_path: PathBuf },

    #[error("Failed to acquire project lock: {0}")]
    AcquisitionFailed(String),

    #[error("Failed to read or parse lockfile: {0}")]
    ReadFailed(String),

    #[error("No running Slate editor found for project at {0:?}")]
    NotFound(PathBuf),
}

impl From<IpcError> for ErrorObjectOwned {
    fn from(err: IpcError) -> Self {
        match err {
            IpcError::Rpc { code, message, data } => ErrorObjectOwned::owned(code, message, data),
            other => ErrorObjectOwned::owned(-32000, other.to_string(), None::<()>),
        }
    }
}
