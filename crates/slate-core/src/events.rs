use crate::history::OpSource;
use crate::id::{MediaId, TimelineId};
use crate::op::{Op, OpError, OpInverse};
use crate::time::RationalTime;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Playing,
    Paused,
    Scrubbing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisTask {
    Transcription,
    WaveformGeneration,
    SceneDetection,
    ProxyTranscode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum EngineEvent {
    OpApplied {
        op: Op,
        source: OpSource,
        label: String,
    },
    OpUndone {
        inverse: OpInverse,
    },
    TimelineChanged {
        timeline_id: TimelineId,
    },
    PlaybackStateChanged(PlaybackState),
    PlayheadMoved(RationalTime),
    MediaImported(MediaId),
    AnalysisProgress {
        media_id: MediaId,
        task: AnalysisTask,
        progress: f32, // 0.0 to 1.0
    },
    ExportProgress {
        job_id: Uuid,
        progress: f32,
    },
    Error(OpError),
}

pub struct EventBus {
    sender: broadcast::Sender<EngineEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: EngineEvent) {
        // Discard send error if there are currently no active receivers
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(512)
    }
}
