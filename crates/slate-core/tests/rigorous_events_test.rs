use slate_core::events::{EngineEvent, EventBus, PlaybackState};
use slate_core::history::OpSource;
use slate_core::model::TrackType;
use slate_core::op::Op;
use slate_core::time::RationalTime;

#[tokio::test]
async fn test_event_bus_publish_subscribe_and_multiple_listeners() {
    let bus = EventBus::new(64);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    // 1. Publish PlaybackStateChanged
    bus.publish(EngineEvent::PlaybackStateChanged(PlaybackState::Playing));

    let ev1 = rx1.recv().await.expect("rx1 receive");
    let ev2 = rx2.recv().await.expect("rx2 receive");

    assert!(matches!(ev1, EngineEvent::PlaybackStateChanged(PlaybackState::Playing)));
    assert!(matches!(ev2, EngineEvent::PlaybackStateChanged(PlaybackState::Playing)));

    // 2. Publish PlayheadMoved
    let target_time = RationalTime::from_seconds(3.5, 30);
    bus.publish(EngineEvent::PlayheadMoved(target_time));

    let ev1_time = rx1.recv().await.expect("rx1 receive time");
    let ev2_time = rx2.recv().await.expect("rx2 receive time");

    if let EngineEvent::PlayheadMoved(t) = ev1_time {
        assert_eq!(t.to_seconds(), 3.5);
    } else {
        panic!("expected PlayheadMoved event on rx1");
    }

    if let EngineEvent::PlayheadMoved(t) = ev2_time {
        assert_eq!(t.to_seconds(), 3.5);
    } else {
        panic!("expected PlayheadMoved event on rx2");
    }

    // 3. Publish OpApplied event with Agent source
    let op = Op::AddTrack {
        track_type: TrackType::Video,
        name: "Agent Track".to_string(),
        index: None,
    };
    bus.publish(EngineEvent::OpApplied {
        op: op.clone(),
        source: OpSource::Agent {
            session_id: "agent-007".to_string(),
        },
        label: "Agent Added Track".to_string(),
    });

    let ev1_op = rx1.recv().await.expect("rx1 receive op");
    let ev2_op = rx2.recv().await.expect("rx2 receive op");

    if let EngineEvent::OpApplied { op: received_op, source, label } = ev1_op {
        assert_eq!(received_op, op);
        assert_eq!(label, "Agent Added Track");
        if let OpSource::Agent { session_id } = source {
            assert_eq!(session_id, "agent-007");
        } else {
            panic!("expected OpSource::Agent");
        }
    } else {
        panic!("expected OpApplied event on rx1");
    }

    assert!(matches!(ev2_op, EngineEvent::OpApplied { .. }));
}
