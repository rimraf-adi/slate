use super::keyframe::{Interpolation, Keyframe, KeyframeTrack};
use crate::id::KeyframeId;
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub position_x: KeyframeTrack<f64>, // Normalized canvas coordinates (-1.0 to 1.0)
    pub position_y: KeyframeTrack<f64>,
    pub scale_x: KeyframeTrack<f64>,    // 1.0 = 100%
    pub scale_y: KeyframeTrack<f64>,
    pub rotation_degrees: KeyframeTrack<f64>,
    pub opacity: KeyframeTrack<f64>,    // 0.0 to 1.0
    pub crop_left: KeyframeTrack<f64>,  // 0.0 to 1.0
    pub crop_right: KeyframeTrack<f64>,
    pub crop_top: KeyframeTrack<f64>,
    pub crop_bottom: KeyframeTrack<f64>,
    pub anchor_x: f64,                  // 0.5 = center
    pub anchor_y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position_x: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 0.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            position_y: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 0.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            scale_x: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 1.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            scale_y: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 1.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            rotation_degrees: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 0.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            opacity: KeyframeTrack {
                keyframes: vec![Keyframe {
                    id: KeyframeId::new(),
                    time: RationalTime::ZERO,
                    value: 1.0,
                    interpolation: Interpolation::Linear,
                }],
            },
            crop_left: KeyframeTrack::new(),
            crop_right: KeyframeTrack::new(),
            crop_top: KeyframeTrack::new(),
            crop_bottom: KeyframeTrack::new(),
            anchor_x: 0.5,
            anchor_y: 0.5,
        }
    }
}
