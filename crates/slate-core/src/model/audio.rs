use super::keyframe::{Interpolation, Keyframe, KeyframeTrack};
use crate::id::KeyframeId;
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AudioParams {
    /// 0.0 dB unity gain, -60.0 to +12.0
    pub volume_db: KeyframeTrack<f64>,
    /// -1.0 (Left) to +1.0 (Right)
    pub pan: KeyframeTrack<f64>,
    pub mute: bool,
    /// Channel remap (e.g. [0, 1] for stereo pass-through)
    pub channel_mapping: Vec<usize>,
}

impl Default for AudioParams {
    fn default() -> Self {
        Self {
            volume_db: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 0.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            pan: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 0.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            mute: false,
            channel_mapping: vec![0, 1],
        }
    }
}
