use slate_core::engine::Engine;
use slate_core::events::{EngineEvent, PlaybackState};
use slate_core::history::OpSource;
use slate_core::id::{MediaId, TrackId};
use slate_core::model::{Clip, MediaMetadata, MediaReference, Project, Track, TrackItem, TrackType};
use slate_core::op::Op;
use slate_core::time::{RationalTime, TimeRange};
use slate_ui::state::{EditorState, ExportPreset, ToolMode};
use slate_ui::timeline::snap::find_snap_target;
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

fn create_state_with_clips() -> (EditorState, TrackId) {
    let mut project = Project::new("UI Rigorous Project");
    let media_id = MediaId::new();
    project.media_pool.push(create_test_media(media_id));

    let timeline = project.active_timeline_mut().unwrap();
    let mut track = Track::new("V1", TrackType::Video);
    let track_id = track.id;

    // Clip 1: [0.0s..5.0s)
    let clip1 = Clip::new(
        "Clip 1",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
    );
    // Clip 2: [5.0s..10.0s)
    let clip2 = Clip::new(
        "Clip 2",
        media_id,
        TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30)),
        TimeRange::new(
            RationalTime::from_seconds(5.0, 30),
            RationalTime::from_seconds(5.0, 30),
        ),
    );

    track.items.push(TrackItem::Clip(clip1));
    track.items.push(TrackItem::Clip(clip2));
    timeline.tracks.push(track);

    let engine = Engine::new(project, 100);
    let state = EditorState::new(engine);
    (state, track_id)
}

#[test]
fn test_magnetic_snapping_rigorous() {
    let (state, _track_id) = create_state_with_clips();
    let engine = state.engine.lock();
    let timeline = engine.project().active_timeline().unwrap();

    let pixels_per_sec = 100.0;
    let scroll_x = 0.0;
    let tolerance = 10.0; // 10 pixels = 0.1s at 100px/s

    let playhead = RationalTime::from_seconds(2.5, 30);

    // 1. Target near Clip 1 start (0.0s) -> snap to 0.0s
    let target_near_0 = RationalTime::from_seconds(0.05, 30); // 5px distance < 10px
    let res0 = find_snap_target(target_near_0, timeline, playhead, pixels_per_sec, scroll_x, tolerance);
    assert!(res0.did_snap);
    assert_eq!(res0.snapped_time.to_seconds(), 0.0);

    // 2. Target near Clip 1 end / Clip 2 start (5.0s) -> snap to 5.0s
    let target_near_5 = RationalTime::from_seconds(5.08, 30); // 8px distance < 10px
    let res5 = find_snap_target(target_near_5, timeline, playhead, pixels_per_sec, scroll_x, tolerance);
    assert!(res5.did_snap);
    assert_eq!(res5.snapped_time.to_seconds(), 5.0);

    // 3. Target near Playhead (2.5s) -> snap to playhead
    let target_near_ph = RationalTime::from_seconds(2.54, 30); // 4px distance < 10px
    let res_ph = find_snap_target(target_near_ph, timeline, playhead, pixels_per_sec, scroll_x, tolerance);
    assert!(res_ph.did_snap);
    assert_eq!(res_ph.snapped_time.to_seconds(), 2.5);

    // 4. Target far from any boundary -> no snap
    let target_far = RationalTime::from_seconds(3.5, 30); // 1.0s = 100px away > 10px
    let res_far = find_snap_target(target_far, timeline, playhead, pixels_per_sec, scroll_x, tolerance);
    assert!(!res_far.did_snap);
}

#[test]
fn test_shuttle_and_frame_stepping_controls() {
    let (mut state, _) = create_state_with_clips();

    assert_eq!(state.shuttle_speed, 0);
    assert!(!state.is_playing);

    // Shuttle Forward: 1x -> 2x -> 4x -> 8x
    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 1);
    assert!(state.is_playing);

    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 2);

    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 4);

    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 8);

    state.shuttle_forward(); // Stays clamped at 8x
    assert_eq!(state.shuttle_speed, 8);

    // Shuttle Pause
    state.shuttle_pause();
    assert_eq!(state.shuttle_speed, 0);
    assert!(!state.is_playing);

    // Shuttle Backward: -1x -> -2x -> -4x -> -8x
    state.shuttle_backward();
    assert_eq!(state.shuttle_speed, -1);
    assert!(state.is_playing);

    state.shuttle_backward();
    assert_eq!(state.shuttle_speed, -2);

    state.shuttle_backward();
    assert_eq!(state.shuttle_speed, -4);

    state.shuttle_backward();
    assert_eq!(state.shuttle_speed, -8);

    // Toggle play
    state.shuttle_pause();
    state.toggle_play();
    assert!(state.is_playing);
    assert_eq!(state.shuttle_speed, 1);
    state.toggle_play();
    assert!(!state.is_playing);
    assert_eq!(state.shuttle_speed, 0);

    // Frame stepping
    state.seek(RationalTime::from_frames(10, 30));
    state.step_frame(1);
    assert_eq!(state.playhead.to_frames(), 11);

    state.step_frame(-5);
    assert_eq!(state.playhead.to_frames(), 6);

    state.step_frame(-10); // Clamped at 0
    assert_eq!(state.playhead.to_frames(), 0);
}

#[test]
fn test_event_handling_and_agent_activity_recording() {
    let (mut state, _track_id) = create_state_with_clips();

    // 1. handle EngineEvent::PlayheadMoved
    state.handle_event(EngineEvent::PlayheadMoved(RationalTime::from_seconds(4.2, 30)));
    assert_eq!(state.playhead.to_seconds(), 4.2);
    assert_eq!(state.playback_clock.current_seconds(), 4.2);

    // 2. handle EngineEvent::PlaybackStateChanged
    state.handle_event(EngineEvent::PlaybackStateChanged(PlaybackState::Playing));
    assert!(state.is_playing);
    assert!(state.playback_clock.is_playing());

    state.handle_event(EngineEvent::PlaybackStateChanged(PlaybackState::Paused));
    assert!(!state.is_playing);
    assert!(!state.playback_clock.is_playing());

    // 3. Apply operation from Agent -> recorded in agent_ops_history
    let op = Op::AddTrack {
        track_type: TrackType::Audio,
        name: "AI Audio Track".to_string(),
        index: None,
    };
    state.apply_op(
        op,
        OpSource::Agent {
            session_id: "agent-session-42".to_string(),
        },
    ).expect("apply agent op");

    assert_eq!(state.agent_ops_history.len(), 1);
    assert_eq!(state.agent_ops_history[0].session_id, "agent-session-42");
    assert!(state.agent_ops_history[0].summary.contains("Timeline edit operation"));

    // 4. Undo and Redo through state
    assert!(state.undo().unwrap());
    assert!(state.redo().unwrap());
}

#[test]
fn test_export_presets_dimensions_and_tools_selection() {
    assert_eq!(ExportPreset::YouTube1080p.dimensions(), (1920, 1080));
    assert_eq!(ExportPreset::YouTube4K.dimensions(), (3840, 2160));
    assert_eq!(ExportPreset::ShortsTikTok9x16.dimensions(), (1080, 1920));
    assert_eq!(ExportPreset::Square1x1.dimensions(), (1080, 1080));
    assert_eq!(ExportPreset::ProRes422Hq.dimensions(), (1920, 1080));
    assert_eq!(ExportPreset::AudioOnlyAac.dimensions(), (0, 0));

    let (mut state, _) = create_state_with_clips();
    assert_eq!(state.active_tool, ToolMode::Select);

    state.active_tool = ToolMode::Blade;
    assert_eq!(state.active_tool, ToolMode::Blade);

    state.active_tool = ToolMode::TrimRipple;
    assert_eq!(state.active_tool, ToolMode::TrimRipple);
}
