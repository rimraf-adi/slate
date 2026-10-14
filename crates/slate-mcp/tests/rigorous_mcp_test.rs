use slate_core::engine::Engine;
use slate_core::model::Project;
use slate_mcp::guardrails::SafetyGuardrails;
use slate_mcp::prompts::list_prompts;
use slate_mcp::resources::list_resources;
use slate_mcp::server::McpServer;
use slate_mcp::tools::McpHandler;

#[test]
fn test_safety_guardrails_thresholds_and_diff() {
    let project = Project::new("Guardrails Project");
    let timeline = project.active_timeline().unwrap();

    // 1. Mutation under threshold (e.g. 10% of 10s) -> allowed without confirmation
    let res_ok = SafetyGuardrails::verify_mutation(timeline, 0.0, 0, false);
    assert!(res_ok.is_ok());

    // 2. Mutation over threshold (e.g. 5.0s out of 10.0s = 50% > 25%) -> rejected when confirm is false
    let big_project = Project::new("Big Project");
    let big_timeline = big_project.active_timeline().unwrap();
    let res_rejected = SafetyGuardrails::verify_mutation(big_timeline, 5.0, 1, false);
    assert!(res_rejected.is_err());

    // 3. Mutation over threshold with confirm=true -> approved
    let res_confirmed = SafetyGuardrails::verify_mutation(big_timeline, 5.0, 1, true);
    assert!(res_confirmed.is_ok());
    let diff = res_confirmed.unwrap();
    assert_eq!(diff.clips_removed, 1);
    assert_eq!(diff.duration_change_seconds, -5.0);
}

#[test]
fn test_prompts_and_resources_definitions() {
    // Prompts
    let prompts = list_prompts();
    assert_eq!(prompts.len(), 3);
    let names: Vec<String> = prompts.into_iter().map(|p| p.name).collect();
    assert!(names.contains(&"rough_cut".to_string()));
    assert!(names.contains(&"make_shorts".to_string()));
    assert!(names.contains(&"clean_audio".to_string()));

    // Resources
    let resources = list_resources();
    assert_eq!(resources.len(), 4);
    let uris: Vec<String> = resources.into_iter().map(|r| r.uri).collect();
    assert!(uris.contains(&"slate://project/timeline".to_string()));
    assert!(uris.contains(&"slate://history".to_string()));
    assert!(uris.contains(&"slate://media/{id}/transcript".to_string()));
    assert!(uris.contains(&"slate://media/{id}/scenes".to_string()));
}

#[tokio::test]
async fn test_mcp_full_tool_call_lifecycle() {
    let project = Project::new("MCP Full Test");
    let engine = Engine::new(project, 100);
    let handler = McpHandler::new_headless(engine);
    let server = McpServer::new(handler);

    // 1. Initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let init_resp = server.handle_message(&init_req.to_string()).await.unwrap();
    assert!(init_resp.contains("slate-mcp"));

    // 2. Call track_add tool
    let add_track_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "track_add",
            "arguments": {
                "track_type": "video",
                "name": "B-Roll Track"
            }
        }
    });
    let add_resp = server.handle_message(&add_track_req.to_string()).await.unwrap();
    assert!(add_resp.contains("B-Roll Track"));

    // 3. Call timeline_summary tool
    let summary_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "timeline_summary",
            "arguments": {}
        }
    });
    let summary_resp = server.handle_message(&summary_req.to_string()).await.unwrap();
    assert!(summary_resp.contains("B-Roll Track"));

    // 4. Call undo tool
    let undo_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "undo",
            "arguments": {}
        }
    });
    let undo_resp = server.handle_message(&undo_req.to_string()).await.unwrap();
    assert!(undo_resp.contains("true") || undo_resp.contains("Undone") || undo_resp.contains("ok"));

    // 5. Call redo tool
    let redo_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "redo",
            "arguments": {}
        }
    });
    let redo_resp = server.handle_message(&redo_req.to_string()).await.unwrap();
    assert!(redo_resp.contains("true") || redo_resp.contains("Redone") || redo_resp.contains("ok"));
}
