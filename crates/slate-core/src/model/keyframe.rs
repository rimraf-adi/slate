use crate::id::KeyframeId;
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Interpolation {
    Hold,
    Linear,
    Bezier {
        /// Control point 1: [time_offset_normalized, value_offset]
        cp1: (f32, f32),
        /// Control point 2: [time_offset_normalized, value_offset]
        cp2: (f32, f32),
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Keyframe<T> {
    pub id: KeyframeId,
    pub time: RationalTime,
    pub value: T,
    pub interpolation: Interpolation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyframeTrack<T> {
    pub keyframes: Vec<Keyframe<T>>,
}

impl<T> Default for KeyframeTrack<T> {
    fn default() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }
}

impl<T> KeyframeTrack<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keyframes.len()
    }
}

impl<T: Copy + Into<f64>> KeyframeTrack<T> {
    /// Evaluates the property value at a specific timeline position using interpolation.
    pub fn evaluate_at(&self, time: RationalTime, default: f64) -> f64 {
        if self.keyframes.is_empty() {
            return default;
        }

        let time_rescaled = time.rescale(self.keyframes[0].time.rate);

        // Before first keyframe
        if time_rescaled <= self.keyframes[0].time {
            return self.keyframes[0].value.into();
        }

        // After last keyframe
        let last = self.keyframes.last().unwrap();
        if time_rescaled >= last.time {
            return last.value.into();
        }

        // Find surrounding keyframes via binary search
        let idx = match self
            .keyframes
            .binary_search_by(|k| k.time.cmp(&time_rescaled))
        {
            Ok(exact) => return self.keyframes[exact].value.into(),
            Err(insertion) => insertion,
        };

        let k0 = &self.keyframes[idx - 1];
        let k1 = &self.keyframes[idx];

        let span = (k1.time - k0.time).to_seconds();
        if span <= 0.0 {
            return k0.value.into();
        }
        let elapsed = (time_rescaled - k0.time).to_seconds();
        let t = (elapsed / span).clamp(0.0, 1.0) as f32;

        let v0: f64 = k0.value.into();
        let v1: f64 = k1.value.into();

        match k0.interpolation {
            Interpolation::Hold => v0,
            Interpolation::Linear => v0 + (v1 - v0) * (t as f64),
            Interpolation::Bezier { cp1, cp2 } => {
                let factor = solve_cubic_bezier(t, cp1.0, cp2.0, cp1.1, cp2.1);
                v0 + (v1 - v0) * (factor as f64)
            }
        }
    }
}

/// Computes cubic bezier interpolation factor using Newton-Raphson approximation
pub fn solve_cubic_bezier(t: f32, x1: f32, x2: f32, y1: f32, y2: f32) -> f32 {
    let mut u = t;
    for _ in 0..8 {
        let x = 3.0 * (1.0 - u) * (1.0 - u) * u * x1 + 3.0 * (1.0 - u) * u * u * x2 + u * u * u;
        let dx = 3.0 * (1.0 - u) * (1.0 - u) * x1
            + 6.0 * (1.0 - u) * u * (x2 - x1)
            + 3.0 * u * u * (1.0 - x2);
        if dx.abs() < 1e-5 {
            break;
        }
        u -= (x - t) / dx;
        u = u.clamp(0.0, 1.0);
    }
    3.0 * (1.0 - u) * (1.0 - u) * u * y1 + 3.0 * (1.0 - u) * u * u * y2 + u * u * u
}
