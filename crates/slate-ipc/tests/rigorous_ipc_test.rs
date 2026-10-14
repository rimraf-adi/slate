use slate_core::history::OpSource;
use slate_core::model::{Project, TrackType};
use slate_core::op::Op;
use slate_core::project::ProjectBundle;
use slate_core::Engine;
use slate_ipc::client::EngineClient;
use slate_ipc::server::IpcServer;
use slate_ipc::transport::lock::{is_process_alive, ProjectLock};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::sync::Mutex;
use uuid::Uuid;

#[test]
fn test_lockfile_lifecycle_and_process_alive() {
    let dir = tempdir().expect("tempdir");
    let project_path = dir.path().join("test.slateproj");
    std::fs::create_dir_all(&project_path).unwrap();

    let project_id = Uuid::new_v4();

    // Current process is alive
    let pid = std::process::id();
    assert!(is_process_alive(pid));

    // Non-existent PID (e.g. 9999999) is not alive
    assert!(!is_process_alive(9999999));

    // 1. Acquire lock
    let lock = ProjectLock::acquire(&project_path, project_id).expect("acquire lock");
    assert!(lock.lock_path.exists());

    // 2. Read lock
    let meta = ProjectLock::read_lock(&lock.lock_path).expect("read lock");
    assert_eq!(meta.pid, pid);
    assert_eq!(meta.project_id, project_id);

    // 3. Drop lock releases files
    drop(lock);
    assert!(!project_path.join("slate.lock").exists());
}

#[tokio::test]
async fn test_ipc_engine_server_and_client_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let proj_dir = dir.path().join("ipc_test.slateproj");

    ProjectBundle::init_bundle(&proj_dir).unwrap();
    let project = Project::new("IPC Test Project");
    ProjectBundle::save_to_bundle(&proj_dir, &project).unwrap();

    let engine = Arc::new(Mutex::new(Engine::new(project.clone(), 100)));

    // 1. Start IPC Server
    let _server = IpcServer::start(&proj_dir, project.id.into(), Arc::clone(&engine))
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
    assert_eq!(summary.tracks.len(), 0);

    // 4. Query full timeline
    let timeline = client
        .get_timeline(None)
        .await
        .expect("Failed to get timeline");
    assert_eq!(timeline.timeline.name, "Main Sequence");

    // 5. Query media list (empty)
    let media = client.list_media().await.expect("list media");
    assert!(media.items.is_empty());

    // 6. Apply AddTrack op
    let add_track_op = Op::AddTrack {
        track_type: TrackType::Audio,
        name: "Voiceover".into(),
        index: None,
    };
    let res = client
        .apply_op(
            add_track_op,
            OpSource::Human,
            "Add Voiceover Track",
        )
        .await
        .expect("apply op");
    assert!(res.ok);

    // 7. Verify track was added
    let updated_summary = client.get_timeline_summary(None).await.expect("summary after op");
    assert_eq!(updated_summary.tracks.len(), 1);
    assert_eq!(updated_summary.tracks[0].name, "Voiceover");

    // 8. Test undo over IPC
    let undo_res = client.undo(None).await.expect("undo");
    assert!(undo_res.ok);

    let summary_after_undo = client.get_timeline_summary(None).await.expect("summary after undo");
    assert_eq!(summary_after_undo.tracks.len(), 0);

    // 9. Test redo over IPC
    let redo_res = client.redo(None).await.expect("redo");
    assert!(redo_res.ok);

    let summary_after_redo = client.get_timeline_summary(None).await.expect("summary after redo");
    assert_eq!(summary_after_redo.tracks.len(), 1);
}
