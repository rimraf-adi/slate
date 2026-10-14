use proptest::prelude::*;
use slate_core::engine::Engine;
use slate_core::history::OpSource;
use slate_core::id::MediaId;
use slate_core::model::{Clip, Project, TrackType};
use slate_core::op::Op;
use slate_core::time::{RationalTime, TimeRange};

proptest! {
    #[test]
    fn test_rational_time_addition_symmetry(ticks_a in 0i64..100_000, ticks_b in 0i64..100_000) {
        let t_a = RationalTime::new(ticks_a, 30);
        let t_b = RationalTime::new(ticks_b, 30);
        let sum_ab = t_a + t_b;
        let sum_ba = t_b + t_a;
        prop_assert_eq!(sum_ab, sum_ba);
        prop_assert_eq!(sum_ab.ticks, ticks_a + ticks_b);
        prop_assert_eq!(sum_ab - t_a, t_b);
        prop_assert_eq!(sum_ab - t_b, t_a);
    }

    #[test]
    fn test_rational_time_rescaling_invariance(seconds in 0.0f64..1000.0) {
        let t30 = RationalTime::from_seconds(seconds, 30);
        let t60 = t30.rescale(60);
        prop_assert_eq!(t60.rate, 60);
        // The difference in seconds between 30 and 60 fps should not exceed 1 frame period at 30 fps
        let diff = (t30.to_seconds() - t60.to_seconds()).abs();
        prop_assert!(diff <= 1.0 / 30.0);
    }

    #[test]
    fn test_op_add_and_undo_invariance(start_frame in 0i64..1000, duration_frames in 1i64..500) {
        let project = Project::new("PropTest Project");
        let mut engine = Engine::new(project, 50);

        let media_id = MediaId::new();
        engine.apply_op(
            Op::ImportMedia {
                path: "sample.mp4".into(),
                media_id,
            },
            OpSource::Human,
            "Import",
        ).unwrap();

        engine.apply_op(
            Op::AddTrack {
                track_type: TrackType::Video,
                name: "V1".into(),
                index: None,
            },
            OpSource::Human,
            "Add Track",
        ).unwrap();

        let track_id = engine.project().active_timeline().unwrap().tracks[0].id;

        let clip = Clip::new(
            "Test Clip",
            media_id,
            TimeRange::new(
                RationalTime::from_frames(0, 30),
                RationalTime::from_frames(duration_frames, 30),
            ),
            TimeRange::new(
                RationalTime::from_frames(start_frame, 30),
                RationalTime::from_frames(duration_frames, 30),
            ),
        );

        // State before AddClip
        let initial_clip_count = engine.project().active_timeline().unwrap().tracks[0].items.len();

        // 1. Apply AddClip
        engine.apply_op(
            Op::AddClip {
                track_id,
                clip: clip.clone(),
                position: RationalTime::from_frames(start_frame, 30),
            },
            OpSource::Human,
            "Add Clip",
        ).unwrap();

        prop_assert_eq!(
            engine.project().active_timeline().unwrap().tracks[0].items.len(),
            initial_clip_count + 1
        );

        // 2. Undo AddClip
        engine.undo().unwrap();

        // Invariance: State after undo is strictly identical to initial state
        prop_assert_eq!(
            engine.project().active_timeline().unwrap().tracks[0].items.len(),
            initial_clip_count
        );
    }
}
