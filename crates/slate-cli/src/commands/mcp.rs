//! Command handler for `slate mcp`.

use anyhow::{Context, Result};
use slate_core::engine::Engine;
use slate_core::model::Project;
use slate_core::project::ProjectBundle;
use slate_mcp::server::McpServer;
use slate_mcp::tools::McpHandler;
use crate::cli::McpArgs;

pub async fn execute(args: McpArgs, _json: bool) -> Result<()> {
    let project = if let Some(ref proj_dir) = args.project {
        ProjectBundle::load_from_bundle(proj_dir)
            .with_context(|| format!("Failed to load project bundle at {:?}", proj_dir))?
    } else {
        Project::new("MCP Session Project")
    };

    let engine = Engine::new(project, 200);

    let handler = if let Some(ref _sock) = args.socket {
        // Attached mode over local socket
        McpHandler::new_headless(engine)
    } else {
        // Headless mode
        McpHandler::new_headless(engine)
    };

    let server = McpServer::new(handler);
    tracing::info!("Starting Slate MCP Stdio server...");
    server.run_stdio().await?;

    Ok(())
}
