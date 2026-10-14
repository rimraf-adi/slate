use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Media decode error: {0}")]
    MediaDecode(String),

    #[error("Model not found at {0}")]
    ModelNotFound(PathBuf),

    #[error("Transcription error: {0}")]
    TranscriptionEngine(String),

    #[error("Voice activity detection error: {0}")]
    VadEngine(String),

    #[error("Scene detection error: {0}")]
    SceneDetection(String),

    #[error("Loudness analysis error: {0}")]
    LoudnessEngine(String),

    #[error("Timeline mapping error: {0}")]
    TimelineMapping(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Task was cancelled")]
    Cancelled,
}
