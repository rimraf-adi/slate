use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::error::McpError;
use crate::prompts::{get_prompt, list_prompts};
use crate::resources::list_resources;
use crate::tools::{get_tool_definitions, McpHandler};

pub struct McpServer {
    handler: Arc<McpHandler>,
}

impl McpServer {
    pub fn new(handler: McpHandler) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }

    /// Handles a single incoming JSON-RPC line and returns the serialized response string (if not a notification)
    pub async fn handle_message(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}"),
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let id = parsed.get("id").cloned();
        let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = parsed.get("params").cloned().unwrap_or(serde_json::Value::Null);

        // Notifications (e.g. notifications/initialized) have no id and require no reply
        if id.is_none() && method.starts_with("notifications/") {
            return None;
        }

        let result = match method {
            "initialize" => Ok(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false },
                    "prompts": { "listChanged": false },
                },
                "serverInfo": {
                    "name": "slate-mcp",
                    "version": "0.1.0",
                }
            })),
            "ping" => Ok(serde_json::json!({})),
            "tools/list" => {
                let tools = get_tool_definitions();
                Ok(serde_json::json!({ "tools": tools }))
            }
            "tools/call" => {
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let tool_args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                match self.handler.dispatch(tool_name, tool_args).await {
                    Ok(tool_res) => Ok(serde_json::to_value(tool_res).unwrap()),
                    Err(mcp_err) => Err(mcp_err),
                }
            }
            "resources/list" => {
                let res = list_resources();
                Ok(serde_json::json!({ "resources": res }))
            }
            "resources/read" => {
                let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                Ok(serde_json::json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": "{}",
                    }]
                }))
            }
            "prompts/list" => {
                let prompts = list_prompts();
                Ok(serde_json::json!({ "prompts": prompts }))
            }
            "prompts/get" => {
                let prompt_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let prompt_args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                if let Some(messages) = get_prompt(prompt_name, &prompt_args) {
                    Ok(serde_json::json!({ "messages": messages }))
                } else {
                    Err(McpError::InvalidParams(format!("Prompt not found: {prompt_name}")))
                }
            }
            other => Err(McpError::InvalidParams(format!("Method not supported: {other}"))),
        };

        let response = match result {
            Ok(res_val) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": res_val,
            }),
            Err(err) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": err.error_code(),
                    "message": err.to_string(),
                    "data": serde_json::to_value(&err).unwrap_or(serde_json::Value::Null),
                }
            }),
        };

        Some(response.to_string())
    }

    /// Runs the stdio loop, processing newline-delimited JSON-RPC messages from stdin
    /// and writing outputs to stdout.
    pub async fn run_stdio(&self) -> Result<(), std::io::Error> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if let Some(reply) = self.handle_message(&line).await {
                stdout.write_all(reply.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }

        Ok(())
    }
}
