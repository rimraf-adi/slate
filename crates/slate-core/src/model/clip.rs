use super::audio::AudioParams;
use super::effect::Effect;
use super::transform::Transform;
use crate::id::{ClipId, MediaId};
use crate::time::TimeRange;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: ClipId,
    pub name: String,
    pub media_id: MediaId,
    /// Bounded range inside the source media
    pub source_range: TimeRange,
    /// Position and duration on the timeline track
    pub timeline_range: TimeRange,
    /// Speed multiplier: 1.0 = normal, 2.0 = 2x, -1.0 = reverse
    pub speed: f64,
    pub transform: Transform,
    pub audio_params: AudioParams,
    pub effects: Vec<Effect>,
}

impl Clip {
    pub fn new(
        name: impl Into<String>,
        media_id: MediaId,
        source_range: TimeRange,
        timeline_range: TimeRange,
    ) -> Self {
        Self {
            id: ClipId::new(),
            name: name.into(),
            media_id,
            source_range,
            timeline_range,
            speed: 1.0,
            transform: Transform::default(),
            audio_params: AudioParams::default(),
            effects: Vec::new(),
        }
    }
}
