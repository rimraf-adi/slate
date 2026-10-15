use slate_core::model::TrackType;
use slate_core::op::Op;
use slate_core::project::ProjectBundle;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_ops_dry_run_and_commit() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("ops_test.slateproj");

    // 1. Initialize empty project
    let status = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("new")
        .arg(&proj_dir)
        .arg("--name")
        .arg("Ops Test")
        .status()
        .expect("slate new failed");
    assert!(status.success());

    let proj_before = ProjectBundle::load_from_bundle(&proj_dir).unwrap();
    let initial_tracks_count = proj_before.timelines[0].tracks.len();

    // 2. Prepare an ops file with AddTrack
    let ops = vec![
        Op::AddTrack {
            name: "New Audio B-Roll".into(),
            track_type: TrackType::Audio,
            index: None,
        },
    ];
    let ops_json = serde_json::to_string(&ops).unwrap();
    let ops_file = dir.path().join("ops.json");
    std::fs::write(&ops_file, ops_json).unwrap();

    // 3. Run dry-run
    let dry_run_output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("ops")
        .arg(&proj_dir)
        .arg("--file")
        .arg(&ops_file)
        .arg("--dry-run")
        .output()
        .expect("slate ops dry-run failed");

    assert!(dry_run_output.status.success());
    let stdout = String::from_utf8_lossy(&dry_run_output.stdout);
    let dry_json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(dry_json["status"], "success");
    assert_eq!(dry_json["dry_run"], true);
    assert_eq!(dry_json["applied_ops"], 1);

    // Verify project on disk was NOT modified by dry-run
    let proj_after_dry = ProjectBundle::load_from_bundle(&proj_dir).unwrap();
    assert_eq!(proj_after_dry.timelines[0].tracks.len(), initial_tracks_count);

    // 4. Run real commit
    let commit_output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("ops")
        .arg(&proj_dir)
        .arg("--file")
        .arg(&ops_file)
        .output()
        .expect("slate ops commit failed");

    assert!(commit_output.status.success());
    let commit_stdout = String::from_utf8_lossy(&commit_output.stdout);
    let commit_json: serde_json::Value = serde_json::from_str(&commit_stdout).unwrap();
    assert_eq!(commit_json["dry_run"], false);
    assert_eq!(commit_json["applied_ops"], 1);

    // Verify project on disk WAS modified
    let proj_after_commit = ProjectBundle::load_from_bundle(&proj_dir).unwrap();
    assert_eq!(proj_after_commit.timelines[0].tracks.len(), initial_tracks_count + 1);
    assert_eq!(
        proj_after_commit.timelines[0].tracks[initial_tracks_count].name,
        "New Audio B-Roll"
    );
}

#[test]
fn test_cli_ops_malformed_json_failure() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("fail_test.slateproj");

    let _ = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("new")
        .arg(&proj_dir)
        .status();

    let malformed_file = dir.path().join("bad.json");
    std::fs::write(&malformed_file, b"not valid json {{{").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("ops")
        .arg(&proj_dir)
        .arg("--file")
        .arg(&malformed_file)
        .output()
        .expect("run ops");

    assert!(!output.status.success(), "Malformed ops JSON must exit with non-zero code");
}

#[test]
fn test_cli_info_missing_file_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("info")
        .arg("/non/existent/path/to/media.mp4")
        .output()
        .expect("run info");

    assert!(!output.status.success(), "Probing non-existent file must fail");
}

#[test]
fn test_cli_analyze_command() {
    let dir = tempdir().unwrap();
    let dummy_file = dir.path().join("dummy.wav");
    std::fs::write(&dummy_file, b"RIFF....WAVEfmt ").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("analyze")
        .arg(&dummy_file)
        .arg("--loudness")
        .arg("--scenes")
        .arg("--silences")
        .arg("--transcribe")
        .output()
        .expect("run analyze");

    assert!(output.status.success(), "analyze command must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse json");

    assert!(json.get("integratedLufs").is_some());
    assert!(json.get("sceneDetectorThreshold").is_some());
    assert!(json.get("vadThreshold").is_some());
    assert!(json.get("transcriptionModel").is_some());
}

#[test]
fn test_cli_doctor_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("doctor")
        .output()
        .expect("run doctor");

    assert!(output.status.success(), "doctor command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse json");
    assert!(json.get("status").is_some());
    assert!(json.get("system").is_some());
    assert!(json.get("diagnostics").is_some());
    assert!(json["diagnostics"].get("gpu").is_some());
    assert!(json["diagnostics"].get("audio").is_some());
    assert!(json["diagnostics"].get("ffmpeg").is_some());
}

#[test]
fn test_cli_timeline_and_track_clip_management() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("full_cli_suite.slateproj");

    // 1. slate new
    let status = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("new")
        .arg(&proj_dir)
        .arg("--name")
        .arg("CLI Studio Project")
        .status()
        .expect("slate new failed");
    assert!(status.success());

    // 2. slate timeline --json
    let tl_output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("timeline")
        .arg(&proj_dir)
        .output()
        .expect("slate timeline");
    assert!(tl_output.status.success());
    let tl_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&tl_output.stdout)).unwrap();
    assert_eq!(tl_json["status"], "success");
    assert_eq!(tl_json["project"]["name"], "CLI Studio Project");
    assert_eq!(tl_json["timeline"]["tracksCount"], 0);

    // 3. slate track add
    let track_add = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("track")
        .arg("add")
        .arg(&proj_dir)
        .arg("V2 Overlay")
        .arg("--type")
        .arg("video")
        .output()
        .expect("slate track add");
    assert!(track_add.status.success(), "track add failed: {}", String::from_utf8_lossy(&track_add.stderr));
    let track_add_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&track_add.stdout)).unwrap();
    assert_eq!(track_add_json["status"], "success");
    assert_eq!(track_add_json["name"], "V2 Overlay");

    // 4. slate track list
    let track_list = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("track")
        .arg("list")
        .arg(&proj_dir)
        .output()
        .expect("slate track list");
    assert!(track_list.status.success());
    let track_list_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&track_list.stdout)).unwrap();
    assert_eq!(track_list_json.as_array().unwrap().len(), 1);

    // 5. slate clip list (initially empty)
    let clip_list = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("clip")
        .arg("list")
        .arg(&proj_dir)
        .output()
        .expect("slate clip list");
    assert!(clip_list.status.success());
    let clip_list_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&clip_list.stdout)).unwrap();
    assert_eq!(clip_list_json.as_array().unwrap().len(), 0);

    // 6. Test export alias
    let export_help = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("export")
        .arg("--help")
        .output()
        .expect("slate export help");
    assert!(export_help.status.success());
}

