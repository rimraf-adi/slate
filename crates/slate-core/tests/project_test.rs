use slate_core::id::MediaId;
use slate_core::model::{Project, TrackType};
use slate_core::op::{Op, OpApplicable};
use slate_core::project::{JournalRecovery, JournalWriter, ProjectBundle};
use tempfile::tempdir;

#[test]
fn test_project_bundle_roundtrip() {
    let dir = tempdir().expect("failed to create temp dir");
    let bundle_path = dir.path().join("test.slateproj");

    let mut project = Project::new("My Film");
    let media_id = MediaId::new();
    let import_op = Op::ImportMedia {
        path: "raw/clip1.mov".into(),
        media_id,
    };
    {
        let (timeline, media_pool) = project.active_timeline_and_media_mut().unwrap();
        import_op
            .apply(timeline, media_pool)
            .expect("import failed");
    }

    // Save bundle
    ProjectBundle::save_to_bundle(&bundle_path, &project).expect("save_to_bundle failed");

    // Check directory structure
    assert!(bundle_path.join("project.json").exists());
    assert!(bundle_path.join("cache").exists());
    assert!(bundle_path.join("cache/proxies").exists());
    assert!(bundle_path.join("cache/waveforms").exists());
    assert!(bundle_path.join("cache/thumbnails").exists());

    // Load bundle
    let loaded = ProjectBundle::load_from_bundle(&bundle_path).expect("load_from_bundle failed");
    assert_eq!(loaded.name, "My Film");
    assert_eq!(loaded.media_pool.len(), 1);
    assert_eq!(loaded.media_pool[0].id, media_id);
}

#[test]
fn test_crash_journal_recovery() {
    let dir = tempdir().expect("failed to create temp dir");

    let mut writer = JournalWriter::open(dir.path()).expect("journal writer init");
    let media_id = MediaId::new();

    let op1 = Op::ImportMedia {
        path: "take1.mp4".into(),
        media_id,
    };
    let op2 = Op::AddTrack {
        track_type: TrackType::Audio,
        name: "A1".into(),
        index: None,
    };

    writer.append_op(&op1).expect("append op1 failed");
    writer.append_op(&op2).expect("append op2 failed");
    drop(writer);

    // Replay recovery
    let mut project = Project::new("Recovered Project");
    let count = JournalRecovery::recover(dir.path(), &mut project)
        .expect("recover failed");

    assert_eq!(count, 2);
    assert_eq!(project.media_pool.len(), 1);
    let timeline = project.active_timeline().unwrap();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].name, "A1");
}
