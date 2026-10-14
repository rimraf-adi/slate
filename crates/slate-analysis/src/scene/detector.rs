use slate_core::time::RationalTime;

use crate::scene::distance::chi_squared_distance;
use crate::scene::hist::ColorHistogram;
use crate::types::{SceneCut, SceneCutType, SceneParams};

pub struct SceneDetector {
    params: SceneParams,
}

impl SceneDetector {
    pub fn new(params: SceneParams) -> Self {
        Self { params }
    }

    /// Detects scene cuts across a sequence of RGBA frame buffers.
    /// `fps` is used to convert frame numbers to `RationalTime`.
    pub fn detect_from_rgba_frames(
        &self,
        frames: &[&[u8]],
        fps: u32,
    ) -> Vec<SceneCut> {
        if frames.len() < 2 {
            return Vec::new();
        }

        let mut cuts = Vec::new();
        let mut prev_hist = ColorHistogram::from_rgba_frame(frames[0], self.params.color_bins);
        let mut last_cut_frame = 0u64;

        for (i, frame) in frames.iter().enumerate().skip(1) {
            let curr_hist = ColorHistogram::from_rgba_frame(frame, self.params.color_bins);
            let score = chi_squared_distance(&prev_hist, &curr_hist);

            let frame_idx = i as u64;
            if score >= self.params.threshold
                && (frame_idx - last_cut_frame) >= self.params.min_scene_length_frames
            {
                cuts.push(SceneCut {
                    frame: frame_idx,
                    time: RationalTime::from_frames(frame_idx as i64, fps),
                    cut_type: SceneCutType::Cut,
                    score,
                });
                last_cut_frame = frame_idx;
            }

            prev_hist = curr_hist;
        }

        cuts
    }
}
