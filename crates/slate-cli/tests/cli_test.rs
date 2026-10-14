use slate_core::model::{Clip, Project, Track, TrackType, Transform};
use slate_core::project::ProjectBundle;
use slate_core::time::RationalTime;
use slate_core::{MediaId, TimeRange};
use slate_media::MediaProbe;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_new_project() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("test_proj.slateproj");

    let status = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("new")
        .arg(&proj_dir)
        .arg("--name")
        .arg("Test Project")
        .arg("--fps")
        .arg("60")
        .arg("--width")
        .arg("1280")
        .arg("--height")
        .arg("720")
        .status()
        .expect("Failed to execute slate new");

    assert!(status.success());
    assert!(proj_dir.join("project.json").exists());
    assert!(proj_dir.join("cache").exists());

    let proj = ProjectBundle::load_from_bundle(&proj_dir).expect("Failed to load bundle");
    assert_eq!(proj.name, "Test Project");
    assert_eq!(proj.timelines[0].settings.width, 1280);
    assert_eq!(proj.timelines[0].settings.height, 720);
    assert_eq!(proj.timelines[0].settings.timebase, 60);
}

#[test]
fn test_cli_version_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("--json")
        .arg("version")
        .output()
        .expect("Failed to execute slate version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert!(json.get("os").is_some());
}

#[test]
fn test_cli_golden_render_pipeline() {
    let _ = slate_media::init();

    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("golden_test.slateproj");
    let export_file = dir.path().join("golden_export.mp4");

    // 1. Create project
    ProjectBundle::init_bundle(&proj_dir).unwrap();
    let mut project = Project::new("Golden Test");

    let range = TimeRange::new(RationalTime::new(0, 1), RationalTime::new(1, 1)); // 1 second
    let mut clip = Clip::new("solid_clip", MediaId::new(), range, range);
    clip.transform = Transform::default();

    let mut track = Track::new("Video Track", TrackType::Video);
    track.items.push(slate_core::model::TrackItem::Clip(clip));

    project.timelines[0].tracks.push(track);
    project.timelines[0].settings.width = 320;
    project.timelines[0].settings.height = 240;
    project.timelines[0].settings.timebase = 30;

    ProjectBundle::save_to_bundle(&proj_dir, &project).unwrap();

    // 2. Render project via CLI
    let status = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("render")
        .arg(&proj_dir)
        .arg("--output")
        .arg(&export_file)
        .arg("--preset")
        .arg("16:9")
        .arg("--bitrate")
        .arg("2000")
        .status()
        .expect("Failed to execute slate render");

    assert!(status.success(), "slate render command failed");
    assert!(export_file.exists(), "Export file does not exist");
    assert!(std::fs::metadata(&export_file).unwrap().len() > 1000, "Export file is too small");

    // 3. Probe rendered export
    let probe = MediaProbe::probe(&export_file).expect("Failed to probe rendered export");
    assert_eq!(probe.width, Some(320));
    assert_eq!(probe.height, Some(240));
    assert!(probe.duration.to_seconds() >= 0.9, "Rendered duration should be ~1.0s");

    // 4. Test Thumbnail extraction on the golden file
    let thumb_file = dir.path().join("golden_thumb.png");
    let thumb_status = Command::new(env!("CARGO_BIN_EXE_slate"))
        .arg("thumbnail")
        .arg(&export_file)
        .arg("--output")
        .arg(&thumb_file)
        .arg("--time")
        .arg("0.5")
        .arg("--width")
        .arg("160")
        .status()
        .expect("Failed to execute slate thumbnail");

    assert!(thumb_status.success());
    assert!(thumb_file.exists());
    assert!(std::fs::metadata(&thumb_file).unwrap().len() > 100);
}
