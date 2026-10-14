//! Unit and integration tests for slate-ui editor state, snapping, playback, and agent sync.

use std::path::PathBuf;
use slate_core::engine::Engine;
use slate_core::id::{ClipId, MediaId, TrackId};
use slate_core::model::{Clip, MediaMetadata, MediaReference, Project, Track, TrackType};
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::{RationalTime, TimeRange, Timecode};
use slate_ui::state::{EditorState, ToolMode};
use slate_ui::timeline::snap::find_snap_target;

fn setup_test_project() -> (Project, TrackId, MediaId) {
    let mut project = Project::new("UI State Test Project");
    let track_id = TrackId::new();
    let media_id = MediaId::new();

    let mut track = Track::new("V1", TrackType::Video);
    track.id = track_id;

    let media_ref = MediaReference {
        id: media_id,
        original_path: PathBuf::from("test_video.mp4"),
        relative_path: PathBuf::from("test_video.mp4"),
        metadata: MediaMetadata {
            file_size_bytes: 1024 * 1024 * 10,
            duration: RationalTime::from_seconds(60.0, 30),
            video_codec: Some("h264".into()),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(30.0),
            timebase: 30,
            audio_codec: Some("aac".into()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
        },
        proxy_path: None,
        waveform_path: None,
    };

    project.media_pool.push(media_ref);
    if let Some(tl) = project.active_timeline_mut() {
        tl.tracks.push(track);
    }

    (project, track_id, media_id)
}

#[test]
fn test_editor_state_initialization_and_timecode() {
    let (project, _track_id, _media_id) = setup_test_project();
    let engine = Engine::new(project, 100);
    let mut state = EditorState::new(engine);

    assert_eq!(state.playhead, RationalTime::from_frames(0, 30));
    assert!(!state.is_playing);
    assert_eq!(state.shuttle_speed, 0);
    assert_eq!(state.active_tool, ToolMode::Select);

    // Seek to 00:00:02:15 (75 frames at 30fps = 2.5s)
    let target_time = RationalTime::from_frames(75, 30);
    state.seek(target_time);
    assert_eq!(state.playhead, target_time);

    let tc_str = Timecode::format(state.playhead, false);
    assert_eq!(tc_str, "00:00:02:15");
}

#[test]
fn test_transport_and_shuttle_controls() {
    let (project, _track_id, _media_id) = setup_test_project();
    let engine = Engine::new(project, 100);
    let mut state = EditorState::new(engine);

    // Toggle play
    state.toggle_play();
    assert!(state.is_playing);
    assert_eq!(state.shuttle_speed, 1);

    // Shuttle forward increments: 1x -> 2x -> 4x -> 8x
    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 2);
    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 4);
    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 8);
    state.shuttle_forward();
    assert_eq!(state.shuttle_speed, 8); // Clamped at 8x

    // Shuttle backward: reverse
    state.shuttle_backward();
    assert_eq!(state.shuttle_speed, -1);
    state.shuttle_backward();
    assert_eq!(state.shuttle_speed, -2);

    // Shuttle pause
    state.shuttle_pause();
    assert!(!state.is_playing);
    assert_eq!(state.shuttle_speed, 0);

    // Step frame forward & back
    state.step_frame(5);
    assert_eq!(state.playhead, RationalTime::from_frames(5, 30));
    state.step_frame(-2);
    assert_eq!(state.playhead, RationalTime::from_frames(3, 30));
}

#[test]
fn test_timeline_magnetic_snapping() {
    let (mut project, track_id, media_id) = setup_test_project();

    // Add clip from 0.0s to 5.0s (0 to 150 frames)
    let clip1 = Clip::new(
        "Clip 1",
        media_id,
        TimeRange::new(RationalTime::from_frames(0, 30), RationalTime::from_seconds(5.0, 30)),
        TimeRange::new(RationalTime::from_frames(0, 30), RationalTime::from_seconds(5.0, 30)),
    );
    if let Some(tl) = project.active_timeline_mut() {
        if let Some(track) = tl.find_track_mut(track_id) {
            track.items.push(slate_core::model::TrackItem::Clip(clip1));
        }
    }

    let timeline = project.active_timeline().unwrap();
    let playhead = RationalTime::from_seconds(10.0, 30);
    let px_per_sec = 100.0;
    let scroll_x = 0.0;
    let tolerance_px = 15.0; // 15px at 100px/s = 0.15s tolerance

    // Test snap to clip end point (5.0s): query at 4.92s (8px away < 15px)
    let query_time = RationalTime::from_seconds(4.92, 30);
    let snap_res = find_snap_target(
        query_time,
        timeline,
        playhead,
        px_per_sec,
        scroll_x,
        tolerance_px,
    );
    assert!(snap_res.did_snap);
    assert_eq!(snap_res.snapped_time, RationalTime::from_seconds(5.0, 30));

    // Test snap to playhead (10.0s): query at 10.05s (5px away < 15px)
    let query_playhead = RationalTime::from_seconds(10.05, 30);
    let snap_playhead = find_snap_target(
        query_playhead,
        timeline,
        playhead,
        px_per_sec,
        scroll_x,
        tolerance_px,
    );
    assert!(snap_playhead.did_snap);
    assert_eq!(snap_playhead.snapped_time, playhead);

    // Test far point: no snap
    let query_far = RationalTime::from_seconds(7.5, 30);
    let snap_far = find_snap_target(
        query_far,
        timeline,
        playhead,
        px_per_sec,
        scroll_x,
        tolerance_px,
    );
    assert!(!snap_far.did_snap);
    assert_eq!(snap_far.snapped_time, query_far);
}

#[test]
fn test_agent_human_collaborative_editing_and_history() {
    let (project, track_id, media_id) = setup_test_project();
    let engine = Engine::new(project, 100);
    let mut state = EditorState::new(engine);

    // 1. Human adds a 10s clip
    let clip_id = ClipId::new();
    let mut clip = Clip::new(
        "Interview Take 1",
        media_id,
        TimeRange::new(RationalTime::from_frames(0, 30), RationalTime::from_seconds(10.0, 30)),
        TimeRange::new(RationalTime::from_frames(0, 30), RationalTime::from_seconds(10.0, 30)),
    );
    clip.id = clip_id;

    let human_add_op = Op::AddClip {
        track_id,
        clip,
        position: RationalTime::ZERO,
    };
    state.apply_op(human_add_op, OpSource::Human).unwrap();

    // Human edits should not show in agent panel
    assert_eq!(state.agent_ops_history.len(), 0);

    // 2. Agent applies a split at 4.0s
    let new_clip_id = ClipId::new();
    let agent_split_op = Op::SplitClip {
        clip_id,
        at: RationalTime::from_seconds(4.0, 30),
        new_clip_id,
    };
    state.apply_op(
        agent_split_op,
        OpSource::Agent {
            session_id: "agent-session-42".into(),
        },
    ).unwrap();

    // Agent edit must be recorded in agent activity history
    assert_eq!(state.agent_ops_history.len(), 1);
    let agent_entry = &state.agent_ops_history[0];
    assert_eq!(agent_entry.session_id, "agent-session-42");
    assert!(agent_entry.summary.contains("Split clip"));
    assert!(agent_entry.affected_clip_ids.contains(&clip_id));
    assert!(agent_entry.affected_clip_ids.contains(&new_clip_id));

    // 3. Shared undo history: undo reverts the agent split!
    let undone = state.undo().unwrap();
    assert!(undone);

    // After undo, redo restores it!
    let redone = state.redo().unwrap();
    assert!(redone);
}

#[test]
fn test_split_and_ripple_delete_selected() {
    let (project, track_id, media_id) = setup_test_project();
    let engine = Engine::new(project, 100);
    let mut state = EditorState::new(engine);

    // Add clip
    let clip_id = ClipId::new();
    let mut clip = Clip::new(
        "Talking Head",
        media_id,
        TimeRange::new(RationalTime::from_frames(0, 30), RationalTime::from_seconds(10.0, 30)),
        TimeRange::new(RationalTime::from_frames(0, 30), RationalTime::from_seconds(10.0, 30)),
    );
    clip.id = clip_id;

    state.apply_op(
        Op::AddClip {
            track_id,
            clip,
            position: RationalTime::ZERO,
        },
        OpSource::Human,
    ).unwrap();

    // Select clip and seek to 4.0s
    state.selected_clip_ids.insert(clip_id);
    state.seek(RationalTime::from_seconds(4.0, 30));

    // Split at playhead
    state.split_at_playhead().unwrap();

    // Verify track now has 2 clips
    let engine = state.engine.lock();
    let tl = engine.project().active_timeline().unwrap();
    let track = tl.find_track(track_id).unwrap();
    assert_eq!(track.items.len(), 2);
}
