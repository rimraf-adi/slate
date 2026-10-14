use slate_core::history::OpSource;
use slate_core::model::{Project, TrackType};
use slate_core::op::Op;
use slate_core::project::ProjectBundle;
use slate_core::Engine;
use slate_ipc::client::EngineClient;
use slate_ipc::server::IpcServer;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_ipc_lifecycle_and_mutation() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("ipc_test.slateproj");

    ProjectBundle::init_bundle(&proj_dir).unwrap();
    let project = Project::new("IPC Test Project");
    ProjectBundle::save_to_bundle(&proj_dir, &project).unwrap();

    let engine = Arc::new(Mutex::new(Engine::new(project.clone(), 100)));

    // 1. Start IPC Server
    let server = IpcServer::start(&proj_dir, project.id.into(), Arc::clone(&engine))
        .await
        .expect("Failed to start IPC server");

    // 2. Connect client via lockfile discovery
    let client = EngineClient::connect_project(&proj_dir, Duration::from_secs(3))
        .await
        .expect("Failed to connect to IPC server");

    // 3. Query timeline summary
    let summary = client
        .get_timeline_summary(None)
        .await
        .expect("Failed to get timeline summary");
    assert_eq!(summary.name, "Main Sequence");

    // 4. Apply AddTrack op over IPC
    let add_track_op = Op::AddTrack {
        track_type: TrackType::Video,
        name: "Agent Track".into(),
        index: Some(0),
    };

    let res = client
        .apply_op(
            add_track_op,
            OpSource::Agent {
                session_id: "agent-007".into(),
            },
            "Add Agent Track",
        )
        .await
        .expect("Failed to apply op");

    assert!(res.ok);

    // Verify track added in engine
    {
        let engine_lock = engine.lock().await;
        let proj = engine_lock.project();
        assert_eq!(proj.timelines[0].tracks.len(), 1);
        assert_eq!(proj.timelines[0].tracks[0].name, "Agent Track");
    }

    // 5. Test Undo over IPC
    let undo_res = client.undo(None).await.expect("Failed to undo");
    assert!(undo_res.ok);
    assert_eq!(undo_res.undone_count, 1);

    {
        let engine_lock = engine.lock().await;
        let proj = engine_lock.project();
        assert_eq!(proj.timelines[0].tracks.len(), 0);
    }

    // 6. Test Redo over IPC
    let redo_res = client.redo(None).await.expect("Failed to redo");
    assert!(redo_res.ok);
    assert_eq!(redo_res.redone_count, 1);

    {
        let engine_lock = engine.lock().await;
        let proj = engine_lock.project();
        assert_eq!(proj.timelines[0].tracks.len(), 1);
    }

    // Stop server
    let _ = server.server_handle.stop();
}

#[tokio::test]
async fn test_ipc_event_subscription_stream() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("sub_test.slateproj");

    ProjectBundle::init_bundle(&proj_dir).unwrap();
    let project = Project::new("Sub Test");
    let engine = Arc::new(Mutex::new(Engine::new(project.clone(), 100)));

    let server = IpcServer::start(&proj_dir, project.id.into(), Arc::clone(&engine))
        .await
        .expect("Failed to start server");

    let client = EngineClient::connect_project(&proj_dir, Duration::from_secs(3))
        .await
        .expect("Failed to connect client");

    let mut sub = client
        .subscribe_events()
        .await
        .expect("Failed to subscribe to events");

    // Spawn task to apply op after a small delay
    let client_clone = client.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = client_clone
            .apply_op(
                Op::AddTrack {
                    track_type: TrackType::Audio,
                    name: "Sub Track".into(),
                    index: Some(0),
                },
                OpSource::Human,
                "Human Add Track",
            )
            .await;
    });

    // Receive event from subscription
    let next_event = tokio::time::timeout(Duration::from_secs(2), sub.next())
        .await
        .expect("Timed out waiting for push notification")
        .expect("Stream terminated unexpectedly")
        .expect("Failed to deserialize event");

    match next_event {
        slate_ipc::EngineEvent::OpApplied { label, source, .. } => {
            assert_eq!(label, "Human Add Track");
            assert_eq!(source, OpSource::Human);
        }
        other => panic!("Unexpected event received: {:?}", other),
    }

    let _ = server.server_handle.stop();
}
