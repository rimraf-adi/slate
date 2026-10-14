use crate::id::MediaId;
use crate::op::WordRange;
use crate::time::{RationalTime, TimeRange};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Whisper inference failed: {0}")]
    TranscriptionFailed(String),
    #[error("Scene detection error: {0}")]
    SceneDetectionFailed(String),
    #[error("VAD speech detection failed: {0}")]
    VadFailed(String),
    #[error("Loudness measurement error: {0}")]
    LoudnessFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub words: Vec<WordRange>,
    pub full_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCut {
    pub time: RationalTime,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessReport {
    pub integrated_lufs: f64,
    pub momentary_max_lufs: f64,
    pub true_peak_dbfs: f64,
}

pub trait MediaAnalyzer: Send + Sync {
    fn transcribe(&self, media_id: MediaId, path: &Path) -> Result<Transcript, AnalysisError>;
    fn detect_scenes(&self, media_id: MediaId, path: &Path) -> Result<Vec<SceneCut>, AnalysisError>;
    fn detect_silence(
        &self,
        media_id: MediaId,
        path: &Path,
        threshold_db: f64,
    ) -> Result<Vec<TimeRange>, AnalysisError>;
    fn measure_loudness(
        &self,
        media_id: MediaId,
        path: &Path,
    ) -> Result<LoudnessReport, AnalysisError>;
}
