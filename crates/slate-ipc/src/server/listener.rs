use crate::error::IpcError;
use crate::protocol::rpc::EngineRpcServer;
use crate::server::state::ServerState;
use crate::transport::lock::ProjectLock;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use slate_core::Engine;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct IpcServer {
    pub lock: ProjectLock,
    pub server_handle: ServerHandle,
    pub local_addr: SocketAddr,
}

impl IpcServer {
    /// Starts an IPC JSON-RPC server for a project, binding to a local port and writing the lockfile.
    pub async fn start(
        project_path: &Path,
        project_id: Uuid,
        engine: Arc<Mutex<Engine>>,
    ) -> Result<Self, IpcError> {
        let state = ServerState::new(engine);
        let rpc_module = state.into_rpc();

        // Bind to localhost on an ephemeral port
        let server = ServerBuilder::default()
            .build("127.0.0.1:0")
            .await
            .map_err(|e| IpcError::Engine(format!("Failed to bind server: {}", e)))?;

        let local_addr = server
            .local_addr()
            .map_err(|e| IpcError::Engine(format!("Failed to query local addr: {}", e)))?;

        let lock = ProjectLock::acquire_with_port(project_path, project_id, Some(local_addr.port()))?;

        let server_handle = server.start(rpc_module);

        Ok(Self {
            lock,
            server_handle,
            local_addr,
        })
    }
}
