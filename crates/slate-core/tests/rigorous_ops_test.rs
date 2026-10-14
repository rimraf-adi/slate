use slate_core::history::{History, OpSource};
use slate_core::id::{ClipId, MediaId, TrackId};
use slate_core::model::{
    Clip, MediaMetadata, MediaReference, Timeline, TimelineSettings, Track, TrackItem, TrackType,
};
use slate_core::op::{Op, OpApplicable, TrimEdge, TrimMode};
use slate_core::time::{RationalTime, TimeRange};
use std::path::PathBuf;

fn create_test_media(id: MediaId) -> MediaReference {
    MediaReference {
        id,
        original_path: PathBuf::from("/dummy.mp4"),
        relative_path: PathBuf::from("dummy.mp4"),
        metadata: MediaMetadata {
            file_size_bytes: 1024,
            duration: RationalTime::from_seconds(100.0, 30),
            video_codec: Some("h264".to_string()),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(30.0),
            timebase: 30,
            audio_codec: Some("aac".to_string()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
        },
        proxy_path: None,
        waveform_path: None,
    }
}

fn setup_test_timeline() -> (Timeline, TrackId) {
    let mut timeline = Timeline::new("Test Timeline", TimelineSettings::default());
    let track = Track::new("V1", TrackType::Video);
    let track_id = track.id;
    timeline.tracks.push(track);
    (timeline, track_id)
}

#[test]
fn test_op_add_and_remove_track_symmetry() {
    let mut timeline = Timeline::new("Test Timeline", TimelineSettings::default());
    let mut media_pool = Vec::new();

    let add_track_op = Op::AddTrack {
        track_type: TrackType::Audio,
        name: "A1".to_string(),
        index: None,
    };

    assert!(add_track_op.validate(&timeline, &media_pool).is_ok());
    let inverse = add_track_op.apply(&mut timeline, &mut media_pool).unwrap();

    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].name, "A1");
    let added_track_id = timeline.tracks[0].id;

    // Undo via inverse
    slate_core::history::apply_inverse(&inverse, &mut timeline, &mut media_pool);
    assert_eq!(timeline.tracks.len(), 0);

    // Re-apply by executing RemoveTrack on non-existent track -> returns TrackNotFound
    let invalid_remove = Op::RemoveTrack {
        track_id: added_track_id,
    };
    assert!(invalid_remove.validate(&timeline, &media_pool).is_err());
}

#[test]
fn test_op_add_and_remove_clip_symmetry() {
    let (mut timeline, track_id) = setup_test_timeline();
    let media_id = MediaId::new();
    let mut media_pool = vec![create_test_media(media_id)];

    let clip = Clip::new(
        "Intro Clip",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
    );
    let clip_id = clip.id;

    let add_clip_op = Op::AddClip {
        track_id,
        clip: clip.clone(),
        position: RationalTime::ZERO,
    };

    assert!(add_clip_op.validate(&timeline, &media_pool).is_ok());
    let inverse = add_clip_op.apply(&mut timeline, &mut media_pool).unwrap();

    assert_eq!(timeline.tracks[0].items.len(), 1);
    assert!(timeline.find_clip(clip_id).is_some());

    // Invert (removes clip)
    slate_core::history::apply_inverse(&inverse, &mut timeline, &mut media_pool);
    assert_eq!(timeline.tracks[0].items.len(), 0);
    assert!(timeline.find_clip(clip_id).is_none());
}

#[test]
fn test_op_trim_clip_start_and_end() {
    let (mut timeline, _track_id) = setup_test_timeline();
    let media_id = MediaId::new();
    let mut media_pool = vec![create_test_media(media_id)];

    let clip = Clip::new(
        "Trim Target",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30)),
    );
    let clip_id = clip.id;
    timeline.tracks[0].items.push(TrackItem::Clip(clip));

    // Trim start edge by -2.0 seconds from source (keeps 8.0s)
    let trim_start_op = Op::TrimClip {
        clip_id,
        edge: TrimEdge::Start,
        delta: RationalTime::from_seconds(-2.0, 30),
        mode: TrimMode::Ripple,
    };

    assert!(trim_start_op.validate(&timeline, &media_pool).is_ok());
    let inverse = trim_start_op.apply(&mut timeline, &mut media_pool).unwrap();

    let (_, trimmed) = timeline.find_clip(clip_id).unwrap();
    assert_eq!(trimmed.timeline_range.duration.to_seconds(), 12.0);

    // Undo trim
    slate_core::history::apply_inverse(&inverse, &mut timeline, &mut media_pool);
    let (_, restored) = timeline.find_clip(clip_id).unwrap();
    assert_eq!(restored.timeline_range.duration.to_seconds(), 10.0);
}

#[test]
fn test_op_split_clip_and_rejoin_undo() {
    let (mut timeline, _track_id) = setup_test_timeline();
    let media_id = MediaId::new();
    let mut media_pool = vec![create_test_media(media_id)];

    let clip = Clip::new(
        "Split Me",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30)),
    );
    let original_clip_id = clip.id;
    let new_clip_id = ClipId::new();
    timeline.tracks[0].items.push(TrackItem::Clip(clip));

    // Split at 4.0s
    let split_op = Op::SplitClip {
        clip_id: original_clip_id,
        at: RationalTime::from_seconds(4.0, 30),
        new_clip_id,
    };

    assert!(split_op.validate(&timeline, &media_pool).is_ok());
    let inverse = split_op.apply(&mut timeline, &mut media_pool).unwrap();

    assert_eq!(timeline.tracks[0].items.len(), 2);
    let (_, left) = timeline.find_clip(original_clip_id).unwrap();
    let (_, right) = timeline.find_clip(new_clip_id).unwrap();

    assert_eq!(left.timeline_range.duration.to_seconds(), 4.0);
    assert_eq!(right.timeline_range.start.to_seconds(), 4.0);
    assert_eq!(right.timeline_range.duration.to_seconds(), 6.0);

    // Undo split
    slate_core::history::apply_inverse(&inverse, &mut timeline, &mut media_pool);
    assert_eq!(timeline.tracks[0].items.len(), 1);
    let (_, restored) = timeline.find_clip(original_clip_id).unwrap();
    assert_eq!(restored.timeline_range.duration.to_seconds(), 10.0);
    assert!(timeline.find_clip(new_clip_id).is_none());
}

#[test]
fn test_op_move_clip_across_time_and_tracks() {
    let (mut timeline, _track1_id) = setup_test_timeline();
    let media_id = MediaId::new();
    let mut media_pool = vec![create_test_media(media_id)];

    let track2 = Track::new("V2", TrackType::Video);
    let track2_id = track2.id;
    timeline.tracks.push(track2);

    let clip = Clip::new(
        "Mover",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
    );
    let clip_id = clip.id;
    timeline.tracks[0].items.push(TrackItem::Clip(clip));

    // Move to track2 at time 8.0s
    let move_op = Op::MoveClip {
        clip_id,
        target_track: track2_id,
        target_time: RationalTime::from_seconds(8.0, 30),
    };

    assert!(move_op.validate(&timeline, &media_pool).is_ok());
    let inverse = move_op.apply(&mut timeline, &mut media_pool).unwrap();

    assert_eq!(timeline.tracks[0].items.len(), 0);
    assert_eq!(timeline.tracks[1].items.len(), 1);
    let (track, moved) = timeline.find_clip(clip_id).unwrap();
    assert_eq!(track.id, track2_id);
    assert_eq!(moved.timeline_range.start.to_seconds(), 8.0);

    // Undo move
    slate_core::history::apply_inverse(&inverse, &mut timeline, &mut media_pool);
    assert_eq!(timeline.tracks[0].items.len(), 1);
    assert_eq!(timeline.tracks[1].items.len(), 0);
    let (_, restored) = timeline.find_clip(clip_id).unwrap();
    assert_eq!(restored.timeline_range.start.to_seconds(), 0.0);
}

#[test]
fn test_op_ripple_delete_ranges() {
    let (mut timeline, track_id) = setup_test_timeline();
    let media_id = MediaId::new();
    let mut media_pool = vec![create_test_media(media_id)];

    // Add 3 consecutive clips: [0..3), [3..6), [6..9)
    for i in 0..3 {
        let clip = Clip::new(
            format!("Clip {}", i),
            media_id,
            TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(3.0, 30)),
            TimeRange::new(
                RationalTime::from_seconds(i as f64 * 3.0, 30),
                RationalTime::from_seconds(3.0, 30),
            ),
        );
        timeline.tracks[0].items.push(TrackItem::Clip(clip));
    }
    assert_eq!(timeline.tracks[0].items.len(), 3);

    // Ripple delete range [3..6) (the middle clip)
    let ripple_op = Op::RippleDelete {
        ranges: vec![TimeRange::new(
            RationalTime::from_seconds(3.0, 30),
            RationalTime::from_seconds(3.0, 30),
        )],
        track_ids: Some(vec![track_id]),
    };

    assert!(ripple_op.validate(&timeline, &media_pool).is_ok());
    let inverse = ripple_op.apply(&mut timeline, &mut media_pool).unwrap();

    // Middle clip removed, last clip shifted left by 3.0s
    assert_eq!(timeline.tracks[0].items.len(), 2);
    let second_clip = match &timeline.tracks[0].items[1] {
        TrackItem::Clip(c) => c,
        _ => panic!("expected clip"),
    };
    assert_eq!(second_clip.timeline_range.start.to_seconds(), 3.0);

    // Undo ripple delete
    slate_core::history::apply_inverse(&inverse, &mut timeline, &mut media_pool);
    assert_eq!(timeline.tracks[0].items.len(), 3);
}

#[test]
fn test_op_batch_atomicity_and_history_stack() {
    let (mut timeline, track_id) = setup_test_timeline();
    let media_id = MediaId::new();
    let mut media_pool = vec![create_test_media(media_id)];
    let mut history = History::new(50);

    let clip1 = Clip::new(
        "Batch 1",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(2.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(2.0, 30)),
    );
    let clip2 = Clip::new(
        "Batch 2",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(2.0, 30)),
        TimeRange::new(
            RationalTime::from_seconds(2.0, 30),
            RationalTime::from_seconds(2.0, 30),
        ),
    );

    let batch = Op::Batch {
        ops: vec![
            Op::AddClip {
                track_id,
                clip: clip1,
                position: RationalTime::ZERO,
            },
            Op::AddClip {
                track_id,
                clip: clip2,
                position: RationalTime::from_seconds(2.0, 30),
            },
        ],
        label: "Add Two Clips in Batch".to_string(),
    };

    assert!(batch.validate(&timeline, &media_pool).is_ok());
    let inverse = batch.apply(&mut timeline, &mut media_pool).unwrap();
    history.push(
        batch.clone(),
        inverse,
        OpSource::Human,
        "Add Two Clips in Batch",
    );

    assert_eq!(timeline.tracks[0].items.len(), 2);
    assert!(history.can_undo());
    assert!(!history.can_redo());

    // Undo batch
    history.undo(&mut timeline, &mut media_pool).unwrap();
    assert_eq!(timeline.tracks[0].items.len(), 0);
    assert!(!history.can_undo());
    assert!(history.can_redo());

    // Redo batch
    history.redo(&mut timeline, &mut media_pool).unwrap();
    assert_eq!(timeline.tracks[0].items.len(), 2);
}
