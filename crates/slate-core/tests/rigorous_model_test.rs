use slate_core::id::{ClipId, KeyframeId, MediaId, TrackId};
use slate_core::model::{
    AudioParams, Clip, Interpolation, Keyframe, KeyframeTrack, Project, Track, TrackItem,
    TrackType, Transform,
};
use slate_core::time::{RationalTime, TimeRange};

#[test]
fn test_keyframe_track_all_interpolations_and_evaluations() {
    let mut track = KeyframeTrack::<f64>::new();
    assert!(track.is_empty());
    assert_eq!(track.len(), 0);

    // Empty track evaluation returns default fallback
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(1.0, 30), 10.0), 10.0);

    // Single keyframe: evaluation before, at, and after always returns that keyframe's value
    track.keyframes.push(Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::from_seconds(2.0, 30),
        value: 100.0,
        interpolation: Interpolation::Linear,
    });
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(0.0, 30), 0.0), 100.0);
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(2.0, 30), 0.0), 100.0);
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(5.0, 30), 0.0), 100.0);

    // Add second keyframe (Linear)
    track.keyframes.push(Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::from_seconds(4.0, 30),
        value: 200.0,
        interpolation: Interpolation::Linear,
    });

    // Check midpoint linear interpolation: at 3.0s, (100.0 + 200.0) / 2 = 150.0
    let mid_val = track.evaluate_at(RationalTime::from_seconds(3.0, 30), 0.0);
    assert!((mid_val - 150.0).abs() < 1e-4);

    // Quarter point: at 2.5s, 100.0 + 0.25 * 100.0 = 125.0
    let quarter_val = track.evaluate_at(RationalTime::from_seconds(2.5, 30), 0.0);
    assert!((quarter_val - 125.0).abs() < 1e-4);

    // Add third keyframe with Bezier
    track.keyframes.push(Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::from_seconds(6.0, 30),
        value: 400.0,
        interpolation: Interpolation::Bezier {
            cp1: (0.5, 0.0),
            cp2: (0.5, 1.0),
        },
    });
    let bezier_val = track.evaluate_at(RationalTime::from_seconds(5.0, 30), 0.0);
    assert!(bezier_val > 200.0 && bezier_val < 400.0);

    // Insert at front with Hold
    track.keyframes.insert(
        0,
        Keyframe {
            id: KeyframeId::new(),
            time: RationalTime::from_seconds(0.0, 30),
            value: 50.0,
            interpolation: Interpolation::Hold,
        },
    );

    // Between 0.0s and 2.0s, Hold maintains 50.0 until 2.0s
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(0.5, 30), 0.0), 50.0);
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(1.5, 30), 0.0), 50.0);
    assert_eq!(track.evaluate_at(RationalTime::from_seconds(2.0, 30), 0.0), 100.0);
}

#[test]
fn test_transform_and_audio_params_defaults_and_evaluations() {
    let transform = Transform::default();
    let t = RationalTime::from_seconds(1.0, 30);

    assert_eq!(transform.position_x.evaluate_at(t, 0.0), 0.0);
    assert_eq!(transform.position_y.evaluate_at(t, 0.0), 0.0);
    assert_eq!(transform.scale_x.evaluate_at(t, 1.0), 1.0);
    assert_eq!(transform.scale_y.evaluate_at(t, 1.0), 1.0);
    assert_eq!(transform.rotation_degrees.evaluate_at(t, 0.0), 0.0);
    assert_eq!(transform.opacity.evaluate_at(t, 1.0), 1.0);
    assert_eq!(transform.anchor_x, 0.5);
    assert_eq!(transform.anchor_y, 0.5);

    let audio_params = AudioParams::default();
    assert_eq!(audio_params.volume_db.evaluate_at(t, 0.0), 0.0); // 0 dB unity gain
    assert_eq!(audio_params.pan.evaluate_at(t, 0.0), 0.0);       // Center pan
    assert!(!audio_params.mute);
    assert_eq!(audio_params.channel_mapping, vec![0, 1]);
}

#[test]
fn test_track_and_clip_management() {
    let mut track = Track::new("Video Track 1", TrackType::Video);
    assert_eq!(track.track_type, TrackType::Video);
    assert!(track.items.is_empty());
    assert!(!track.locked);
    assert!(!track.muted);

    // Add Clip
    let media_id = MediaId::new();
    let timeline_range = TimeRange::new(
        RationalTime::from_seconds(1.0, 30),
        RationalTime::from_seconds(5.0, 30),
    );
    let source_range = TimeRange::new(
        RationalTime::from_seconds(0.0, 30),
        RationalTime::from_seconds(5.0, 30),
    );
    let clip = Clip::new("Clip 1", media_id, source_range, timeline_range);
    let clip_id = clip.id;

    track.items.push(TrackItem::Clip(clip));
    assert_eq!(track.items.len(), 1);

    // Clip query
    let found = track.find_clip(clip_id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Clip 1");

    let not_found = track.find_clip(ClipId::new());
    assert!(not_found.is_none());

    // Clip mutable query
    let mut_found = track.find_clip_mut(clip_id);
    assert!(mut_found.is_some());
    mut_found.unwrap().name = "Renamed Clip".to_string();
    assert_eq!(track.find_clip(clip_id).unwrap().name, "Renamed Clip");
}

#[test]
fn test_timeline_and_project_lifecycle() {
    let mut project = Project::new("Test Rigorous Project");
    assert_eq!(project.name, "Test Rigorous Project");
    assert_eq!(project.timelines.len(), 1);

    let timeline = project.active_timeline_mut().expect("active timeline exists");
    assert_eq!(timeline.name, "Main Sequence");
    assert_eq!(timeline.settings.width, 1920);
    assert_eq!(timeline.settings.height, 1080);
    assert_eq!(timeline.settings.timebase, 30);
    assert_eq!(timeline.tracks.len(), 0);

    // Add Video Track and Audio Track
    let v_track = Track::new("V1", TrackType::Video);
    let v_track_id = v_track.id;
    let a_track = Track::new("A1", TrackType::Audio);
    let a_track_id = a_track.id;

    timeline.tracks.push(v_track);
    timeline.tracks.push(a_track);

    assert_eq!(timeline.tracks.len(), 2);
    assert!(timeline.find_track(v_track_id).is_some());
    assert!(timeline.find_track(a_track_id).is_some());
    assert!(timeline.find_track(TrackId::new()).is_none());

    // Add clip into V1
    let clip = Clip::new(
        "V1 Clip",
        MediaId::new(),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30)),
    );
    let clip_id = clip.id;
    timeline.find_track_mut(v_track_id).unwrap().items.push(TrackItem::Clip(clip));

    assert!(timeline.find_clip(clip_id).is_some());
    assert_eq!(timeline.duration().to_seconds(), 10.0);

    // Save and Load JSON roundtrip
    let json = serde_json::to_string_pretty(&project).expect("serialize project");
    let deserialized: Project = serde_json::from_str(&json).expect("deserialize project");
    assert_eq!(deserialized.name, project.name);
    assert_eq!(deserialized.timelines[0].tracks.len(), 2);
    assert!(deserialized.timelines[0].find_clip(clip_id).is_some());
}
