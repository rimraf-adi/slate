use slate_core::engine::Engine;
use slate_core::model::Project;
use slate_core::time::RationalTime;
use slate_mcp::server::McpServer;
use slate_mcp::tools::McpHandler;
use slate_mcp::types::time::{TimeArg, TimeResponse};

#[test]
fn test_time_arg_parsing_and_response() {
    let rate = 30;

    // Frames suffix
    let t1 = TimeArg::parse("90f", rate).unwrap();
    assert_eq!(t1.to_rational().to_frames(), 90);
    assert_eq!(t1.to_rational().to_seconds(), 3.0);

    // Seconds suffix
    let t2 = TimeArg::parse("2.5s", rate).unwrap();
    assert_eq!(t2.to_rational().to_frames(), 75);

    // SMPTE timecode
    let t3 = TimeArg::parse("00:00:04:15", rate).unwrap();
    assert_eq!(t3.to_rational().to_frames(), 135);

    // Raw integers (treated as frames)
    let t4 = TimeArg::parse("60", rate).unwrap();
    assert_eq!(t4.to_rational().to_frames(), 60);

    // Raw float (treated as seconds)
    let t5 = TimeArg::parse("1.5", rate).unwrap();
    assert_eq!(t5.to_rational().to_frames(), 45);

    // Tri-format response
    let resp = TimeResponse::from_rational(RationalTime::from_frames(90, 30), false);
    assert_eq!(resp.frames, 90);
    assert_eq!(resp.seconds, 3.0);
    assert_eq!(resp.timecode, "00:00:03:00");
}

#[tokio::test]
async fn test_mcp_initialize_and_tools_list() {
    let project = Project::new("MCP Test Project");
    let engine = Engine::new(project, 100);
    let handler = McpHandler::new_headless(engine);
    let server = McpServer::new(handler);

    // Initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let reply = server.handle_message(&init_req.to_string()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(resp["result"]["serverInfo"]["name"], "slate-mcp");

    // Ping
    let ping_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping",
        "params": {}
    });
    let reply_ping = server.handle_message(&ping_req.to_string()).await.unwrap();
    let resp_ping: serde_json::Value = serde_json::from_str(&reply_ping).unwrap();
    assert!(resp_ping["result"].is_object());

    // Tools list
    let list_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list",
        "params": {}
    });
    let reply_tools = server.handle_message(&list_req.to_string()).await.unwrap();
    let resp_tools: serde_json::Value = serde_json::from_str(&reply_tools).unwrap();
    let tools = resp_tools["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 26, "Expected at least 26 tools, got {}", tools.len());

    // Verify all tools have name, description, and valid inputSchema
    for tool in tools {
        assert!(tool.get("name").is_some());
        assert!(tool.get("description").is_some());
        assert!(tool.get("inputSchema").is_some());
    }
}

#[tokio::test]
async fn test_mcp_tool_execution_workflow() {
    let project = Project::new("Podcast Workflow");
    let engine = Engine::new(project, 100);
    let handler = McpHandler::new_headless(engine);
    let server = McpServer::new(handler);

    // 1. project_new
    let new_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "project_new",
            "arguments": {
                "name": "Brand New Project",
                "fps": 30.0,
                "resolution": [1920, 1080]
            }
        }
    });
    let reply = server.handle_message(&new_req.to_string()).await.unwrap();
    assert!(reply.contains("Brand New Project"));

    // 2. timeline_summary
    let summary_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "timeline_summary",
            "arguments": {}
        }
    });
    let reply_summary = server.handle_message(&summary_req.to_string()).await.unwrap();
    let resp_sum: serde_json::Value = serde_json::from_str(&reply_summary).unwrap();
    let content_text = resp_sum["result"]["content"][0]["text"].as_str().unwrap();
    let summary_obj: serde_json::Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(summary_obj["projectName"], "Brand New Project");
    assert_eq!(summary_obj["tracks"].as_array().unwrap().len(), 2);

    let v1_id = summary_obj["tracks"][0]["id"].as_str().unwrap().to_string();

    // 3. media_import
    let import_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "media_import",
            "arguments": {
                "path": "interview.mp4",
                "transcribe": true
            }
        }
    });
    let reply_import = server.handle_message(&import_req.to_string()).await.unwrap();
    let resp_imp: serde_json::Value = serde_json::from_str(&reply_import).unwrap();
    let imp_text = resp_imp["result"]["content"][0]["text"].as_str().unwrap();
    let imp_obj: serde_json::Value = serde_json::from_str(imp_text).unwrap();
    let media_id = imp_obj["mediaId"].as_str().unwrap().to_string();

    // 4. clip_add
    let clip_add_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "tools/call",
        "params": {
            "name": "clip_add",
            "arguments": {
                "track_id": v1_id,
                "media_id": media_id,
                "name": "Interview Take 1",
                "source_start": "0f",
                "source_duration": "10.0s",
                "timeline_position": "00:00:00:00"
            }
        }
    });
    let reply_clip = server.handle_message(&clip_add_req.to_string()).await.unwrap();
    let resp_clip: serde_json::Value = serde_json::from_str(&reply_clip).unwrap();
    let clip_text = resp_clip["result"]["content"][0]["text"].as_str().unwrap();
    let clip_obj: serde_json::Value = serde_json::from_str(clip_text).unwrap();
    let clip_id = clip_obj["clipId"].as_str().unwrap().to_string();

    // 5. clip_split
    let split_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "tools/call",
        "params": {
            "name": "clip_split",
            "arguments": {
                "clip_id": clip_id,
                "at": "5.0s"
            }
        }
    });
    let reply_split = server.handle_message(&split_req.to_string()).await.unwrap();
    assert!(reply_split.contains("newClipId"));

    // 6. render_frame
    let render_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 15,
        "method": "tools/call",
        "params": {
            "name": "render_frame",
            "arguments": {
                "time": "2.5s",
                "width": 160,
                "height": 90
            }
        }
    });
    let reply_render = server.handle_message(&render_req.to_string()).await.unwrap();
    let resp_render: serde_json::Value = serde_json::from_str(&reply_render).unwrap();
    let content_block = &resp_render["result"]["content"][0];
    assert_eq!(content_block["type"], "image");
    assert_eq!(content_block["mimeType"], "image/png");
    assert!(!content_block["data"].as_str().unwrap().is_empty());

    // 7. undo
    let undo_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 16,
        "method": "tools/call",
        "params": {
            "name": "undo",
            "arguments": {
                "steps": 1
            }
        }
    });
    let reply_undo = server.handle_message(&undo_req.to_string()).await.unwrap();
    let resp_undo: serde_json::Value = serde_json::from_str(&reply_undo).unwrap();
    let undo_text = resp_undo["result"]["content"][0]["text"].as_str().unwrap();
    let undo_obj: serde_json::Value = serde_json::from_str(undo_text).unwrap();
    assert_eq!(undo_obj["canRedo"], true);
}

#[tokio::test]
async fn test_mcp_safety_guardrail() {
    let project = Project::new("Safety Guardrail Project");
    let engine = Engine::new(project, 100);
    let handler = McpHandler::new_headless(engine);
    let server = McpServer::new(handler);

    // Initial setup: add track and 10s clip
    let setup_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "tools/call",
        "params": {
            "name": "project_new",
            "arguments": { "name": "Guardrail Test", "fps": 30.0 }
        }
    });
    server.handle_message(&setup_req.to_string()).await;

    // Get track ID
    let sum_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "tools/call",
        "params": { "name": "timeline_summary", "arguments": {} }
    });
    let reply_sum = server.handle_message(&sum_req.to_string()).await.unwrap();
    let sum_json: serde_json::Value = serde_json::from_str(&reply_sum).unwrap();
    let content_str = sum_json["result"]["content"][0]["text"].as_str().unwrap();
    let sum_obj: serde_json::Value = serde_json::from_str(content_str).unwrap();
    let track_id = sum_obj["tracks"][0]["id"].as_str().unwrap().to_string();

    // Import media first so media_id exists in project media pool
    let import_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "tools/call",
        "params": {
            "name": "media_import",
            "arguments": {
                "path": "safety_test_video.mp4"
            }
        }
    });
    let reply_imp = server.handle_message(&import_req.to_string()).await.unwrap();
    let imp_json: serde_json::Value = serde_json::from_str(&reply_imp).unwrap();
    let imp_text = imp_json["result"]["content"][0]["text"].as_str().unwrap();
    let imp_obj: serde_json::Value = serde_json::from_str(imp_text).unwrap();
    let media_id = imp_obj["mediaId"].as_str().unwrap().to_string();

    // Add 10s clip
    let clip_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "tools/call",
        "params": {
            "name": "clip_add",
            "arguments": {
                "track_id": track_id,
                "media_id": media_id,
                "source_duration": "10.0s",
                "timeline_position": "0f"
            }
        }
    });
    server.handle_message(&clip_req.to_string()).await;

    // Now attempt a destructive delete of 5 seconds (50% of the 10s timeline) without confirm
    let destructive_delete_op = serde_json::json!({
        "type": "rippleDelete",
        "payload": {
            "ranges": [
                {
                    "start": { "ticks": 0, "rate": 30 },
                    "duration": { "ticks": 150, "rate": 30 } // 5 seconds
                }
            ],
            "trackIds": null
        }
    });

    let batch_unconfirmed_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "tools/call",
        "params": {
            "name": "ops_apply",
            "arguments": {
                "ops": [destructive_delete_op],
                "confirm": false
            }
        }
    });
    let reply_batch = server.handle_message(&batch_unconfirmed_req.to_string()).await.unwrap();
    assert!(reply_batch.contains("Safety threshold exceeded"));
    assert!(reply_batch.contains("-32005")); // Error code for SafetyGuardTriggered

    // Test dry_run: true succeeds and returns diff preview
    let dry_run_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 24,
        "method": "tools/call",
        "params": {
            "name": "ops_apply",
            "arguments": {
                "ops": [destructive_delete_op],
                "dry_run": true
            }
        }
    });
    let reply_dry_run = server.handle_message(&dry_run_req.to_string()).await.unwrap();
    let resp_dry: serde_json::Value = serde_json::from_str(&reply_dry_run).unwrap();
    let dry_text = resp_dry["result"]["content"][0]["text"].as_str().unwrap();
    let dry_obj: serde_json::Value = serde_json::from_str(dry_text).unwrap();
    assert_eq!(dry_obj["dryRun"], true);
    assert_eq!(dry_obj["diff"]["mutationPercentage"], 50.0);

    // Now retry with confirm: true - should succeed!
    let confirmed_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 25,
        "method": "tools/call",
        "params": {
            "name": "ops_apply",
            "arguments": {
                "ops": [destructive_delete_op],
                "confirm": true
            }
        }
    });
    let reply_confirmed = server.handle_message(&confirmed_req.to_string()).await.unwrap();
    let resp_conf: serde_json::Value = serde_json::from_str(&reply_confirmed).unwrap();
    let conf_text = resp_conf["result"]["content"][0]["text"].as_str().unwrap();
    let conf_obj: serde_json::Value = serde_json::from_str(conf_text).unwrap();
    assert_eq!(conf_obj["success"], true);
}

#[tokio::test]
async fn test_mcp_resources_and_prompts() {
    let project = Project::new("Resources & Prompts Project");
    let engine = Engine::new(project, 100);
    let handler = McpHandler::new_headless(engine);
    let server = McpServer::new(handler);

    // Resources list
    let res_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "resources/list",
        "params": {}
    });
    let reply_res = server.handle_message(&res_req.to_string()).await.unwrap();
    assert!(reply_res.contains("slate://project/timeline"));

    // Prompts list
    let prompt_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "prompts/list",
        "params": {}
    });
    let reply_prompt = server.handle_message(&prompt_req.to_string()).await.unwrap();
    assert!(reply_prompt.contains("rough_cut"));
    assert!(reply_prompt.contains("make_shorts"));
    assert!(reply_prompt.contains("clean_audio"));

    // Prompts get
    let get_prompt_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 32,
        "method": "prompts/get",
        "params": {
            "name": "rough_cut",
            "arguments": {
                "media_path": "raw_footage.mp4",
                "style": "punchy"
            }
        }
    });
    let reply_get = server.handle_message(&get_prompt_req.to_string()).await.unwrap();
    assert!(reply_get.contains("raw_footage.mp4"));
    assert!(reply_get.contains("punchy"));
}
