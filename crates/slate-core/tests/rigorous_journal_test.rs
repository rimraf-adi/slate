use slate_core::model::{Project, TrackType};
use slate_core::op::Op;
use slate_core::project::{JournalRecovery, JournalWriter};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_journal_append_and_recovery() {
    let dir = tempdir().expect("create tempdir");
    let bundle_path = dir.path();

    let mut project = Project::new("WAL Test");
    assert_eq!(project.timelines[0].tracks.len(), 0);

    // 1. Write operations into journal
    let mut writer = JournalWriter::open(bundle_path).expect("open journal writer");
    let op1 = Op::AddTrack {
        track_type: TrackType::Video,
        name: "Recovered Video".to_string(),
        index: None,
    };
    let op2 = Op::AddTrack {
        track_type: TrackType::Audio,
        name: "Recovered Audio".to_string(),
        index: None,
    };

    writer.append_op(&op1).expect("append op1");
    writer.append_op(&op2).expect("append op2");

    // 2. Recover operations into project
    let recovered_count =
        JournalRecovery::recover(bundle_path, &mut project).expect("recover journal");
    assert_eq!(recovered_count, 2);
    assert_eq!(project.timelines[0].tracks.len(), 2);
    assert_eq!(project.timelines[0].tracks[0].name, "Recovered Video");
    assert_eq!(project.timelines[0].tracks[1].name, "Recovered Audio");

    // 3. Truncate journal
    writer.truncate().expect("truncate journal");
    let recovered_after_trunc =
        JournalRecovery::recover(bundle_path, &mut project).expect("recover empty");
    assert_eq!(recovered_after_trunc, 0);
}

#[test]
fn test_journal_crc_corruption_detection() {
    let dir = tempdir().expect("create tempdir");
    let bundle_path = dir.path();
    let journal_path = bundle_path.join("autosave.journal");

    // Write a corrupted entry where CRC does not match payload
    let mut file = File::create(&journal_path).expect("create corrupted journal");
    writeln!(file, "deadbeef:{{\"type\":\"AddTrack\",\"payload\":{{\"trackType\":\"Video\",\"name\":\"Fake Track\",\"index\":null}}}}").expect("write line");

    let mut project = Project::new("Corrupt Test");
    let result = JournalRecovery::recover(bundle_path, &mut project);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.contains("CRC mismatch"));
}
