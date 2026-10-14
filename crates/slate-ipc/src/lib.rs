pub mod client;
pub mod error;
pub mod protocol;
pub mod server;
pub mod transport;

pub use client::EngineClient;
pub use error::{IpcError, LockError};
pub use protocol::events::EngineEvent;
pub use protocol::requests::*;
pub use protocol::responses::*;
pub use protocol::rpc::{EngineRpcClient, EngineRpcServer};
pub use server::IpcServer;
pub use transport::lock::{is_process_alive, ProjectLock, ProjectLockMetadata};
pub use transport::path::{compute_project_hash, get_lockfile_path, get_socket_path};
