use slate_core::engine::Engine;
use slate_core::history::OpSource;
use slate_core::id::{ClipId, MediaId, TrackId};
use slate_core::model::{Clip, Project, TrackItem, TrackType};
use slate_core::op::{Op, TrimEdge, TrimMode};
use slate_core::time::{RationalTime, TimeRange};

fn create_test_engine() -> (Engine, TrackId, MediaId) {
    let project = Project::new("Test Project");
    let mut engine = Engine::new(project, 100);
    let media_id = MediaId::new();

    // 1. Import media
    engine
        .apply_op(
            Op::ImportMedia {
                path: "test.mp4".into(),
                media_id,
            },
            OpSource::Human,
            "Import Video",
        )
        .expect("ImportMedia failed");

    // 2. Add video track
    engine
        .apply_op(
            Op::AddTrack {
                track_type: TrackType::Video,
                name: "V1".into(),
                index: None,
            },
            OpSource::Human,
            "Add Track V1",
        )
        .expect("AddTrack failed");

    let track_id = engine
        .project()
        .active_timeline()
        .unwrap()
        .tracks[0]
        .id;

    (engine, track_id, media_id)
}

#[test]
fn test_clip_add_undo_redo() {
    let (mut engine, track_id, media_id) = create_test_engine();

    let clip = Clip::new(
        "Take 1",
        media_id,
        TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(90, 30),
        ),
        TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(90, 30),
        ),
    );
    let clip_id = clip.id;

    // Apply AddClip
    engine
        .apply_op(
            Op::AddClip {
                track_id,
                clip: clip.clone(),
                position: RationalTime::from_frames(0, 30),
            },
            OpSource::Agent {
                session_id: "test-agent".into(),
            },
            "Agent adds clip",
        )
        .expect("AddClip failed");

    let timeline = engine.project().active_timeline().unwrap();
    assert_eq!(timeline.tracks[0].items.len(), 1);

    // Undo
    engine.undo().expect("Undo failed");
    let timeline = engine.project().active_timeline().unwrap();
    assert_eq!(timeline.tracks[0].items.len(), 0);

    // Redo
    engine.redo().expect("Redo failed");
    let timeline = engine.project().active_timeline().unwrap();
    assert_eq!(timeline.tracks[0].items.len(), 1);
    match &timeline.tracks[0].items[0] {
        TrackItem::Clip(c) => assert_eq!(c.id, clip_id),
        _ => panic!("Expected clip"),
    }
}

#[test]
fn test_clip_split_undo() {
    let (mut engine, track_id, media_id) = create_test_engine();

    let clip = Clip::new(
        "Clip A",
        media_id,
        TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(120, 30),
        ),
        TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(120, 30),
        ),
    );
    let original_clip_id = clip.id;

    engine
        .apply_op(
            Op::AddClip {
                track_id,
                clip,
                position: RationalTime::from_frames(0, 30),
            },
            OpSource::Human,
            "Add clip",
        )
        .unwrap();

    let new_clip_id = ClipId::new();
    // Split at frame 60
    engine
        .apply_op(
            Op::SplitClip {
                clip_id: original_clip_id,
                at: RationalTime::from_frames(60, 30),
                new_clip_id,
            },
            OpSource::Human,
            "Blade cut at 60",
        )
        .unwrap();

    {
        let timeline = engine.project().active_timeline().unwrap();
        assert_eq!(timeline.tracks[0].items.len(), 2);
        if let TrackItem::Clip(first) = &timeline.tracks[0].items[0] {
            assert_eq!(first.id, original_clip_id);
            assert_eq!(first.timeline_range.duration.ticks, 60);
        }
        if let TrackItem::Clip(second) = &timeline.tracks[0].items[1] {
            assert_eq!(second.id, new_clip_id);
            assert_eq!(second.timeline_range.start.ticks, 60);
            assert_eq!(second.timeline_range.duration.ticks, 60);
        }
    }

    // Undo Split
    engine.undo().unwrap();
    {
        let timeline = engine.project().active_timeline().unwrap();
        assert_eq!(timeline.tracks[0].items.len(), 1);
        if let TrackItem::Clip(restored) = &timeline.tracks[0].items[0] {
            assert_eq!(restored.id, original_clip_id);
            assert_eq!(restored.timeline_range.duration.ticks, 120);
        }
    }
}

#[test]
fn test_clip_trim_undo() {
    let (mut engine, track_id, media_id) = create_test_engine();

    let clip = Clip::new(
        "Clip B",
        media_id,
        TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(90, 30),
        ),
        TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(90, 30),
        ),
    );
    let clip_id = clip.id;

    engine
        .apply_op(
            Op::AddClip {
                track_id,
                clip,
                position: RationalTime::from_frames(0, 30),
            },
            OpSource::Human,
            "Add clip",
        )
        .unwrap();

    // Trim End by +30 frames (extend)
    engine
        .apply_op(
            Op::TrimClip {
                clip_id,
                edge: TrimEdge::End,
                delta: RationalTime::from_frames(30, 30),
                mode: TrimMode::Ripple,
            },
            OpSource::Human,
            "Trim End +30",
        )
        .unwrap();

    {
        let timeline = engine.project().active_timeline().unwrap();
        if let TrackItem::Clip(trimmed) = &timeline.tracks[0].items[0] {
            assert_eq!(trimmed.timeline_range.duration.ticks, 120);
        }
    }

    // Undo Trim
    engine.undo().unwrap();
    {
        let timeline = engine.project().active_timeline().unwrap();
        if let TrackItem::Clip(reverted) = &timeline.tracks[0].items[0] {
            assert_eq!(reverted.timeline_range.duration.ticks, 90);
        }
    }
}
