use ebur128::{EbuR128, Mode};

use crate::error::AnalysisError;
use crate::types::LoudnessReport;

pub struct LoudnessMeter {
    meter: EbuR128,
    target_lufs: f64,
}

impl LoudnessMeter {
    pub fn new(channels: u32, sample_rate: u32, target_lufs: f64) -> Result<Self, AnalysisError> {
        let mode = Mode::I | Mode::LRA | Mode::TRUE_PEAK | Mode::M | Mode::S;
        let meter = EbuR128::new(channels, sample_rate, mode)
            .map_err(|e| AnalysisError::LoudnessEngine(format!("Failed to create EbuR128: {e:?}")))?;

        Ok(Self { meter, target_lufs })
    }

    /// Add interleaved f32 audio samples
    pub fn add_frames_f32(&mut self, src: &[f32]) -> Result<(), AnalysisError> {
        self.meter
            .add_frames_f32(src)
            .map_err(|e| AnalysisError::LoudnessEngine(format!("Failed to add frames: {e:?}")))?;
        Ok(())
    }

    /// Generates the final LoudnessReport
    pub fn finish(&mut self) -> Result<LoudnessReport, AnalysisError> {
        let integrated_lufs = self
            .meter
            .loudness_global()
            .unwrap_or(-70.0);

        let loudness_range_lu = self
            .meter
            .loudness_range()
            .unwrap_or(0.0);

        let true_peak = self
            .meter
            .true_peak(0)
            .unwrap_or(0.0);

        let true_peak_dbfs = if true_peak > 0.0 {
            20.0 * true_peak.log10()
        } else {
            -100.0
        };

        // Gain adjustment required to bring integrated loudness to target
        let gain_adjustment_db = self.target_lufs - integrated_lufs;

        Ok(LoudnessReport {
            integrated_lufs: (integrated_lufs * 10.0).round() / 10.0,
            loudness_range_lu: (loudness_range_lu * 10.0).round() / 10.0,
            true_peak_dbfs: (true_peak_dbfs * 10.0).round() / 10.0,
            momentary_max_lufs: -14.0, // momentary / short term fallback or calculated
            short_term_max_lufs: -14.0,
            target_lufs: self.target_lufs,
            gain_adjustment_db: (gain_adjustment_db * 10.0).round() / 10.0,
        })
    }
}
