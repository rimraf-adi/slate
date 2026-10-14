use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum McpError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Clip not found: {clip_id}")]
    ClipNotFound {
        clip_id: String,
        active_clips: Vec<String>,
        suggestion: String,
    },

    #[error("Track not found: {track_id}")]
    TrackNotFound {
        track_id: String,
        active_tracks: Vec<String>,
        suggestion: String,
    },

    #[error("Overlap conflict: {0}")]
    OverlapConflict(String),

    #[error("Trim point exceeds source boundaries")]
    TrimBeyondMedia {
        clip_id: String,
        requested_time: String,
        available_limit: String,
        suggestion: String,
    },

    #[error("Safety threshold exceeded (>25% timeline mutation): requires confirm: true or prior dry_run")]
    SafetyGuardTriggered {
        deleted_duration_seconds: f64,
        total_timeline_seconds: f64,
        percentage_deleted: f64,
        remedy: String,
    },

    #[error("Media asset is not ready: {media_id} ({stage})")]
    MediaNotReady {
        media_id: String,
        stage: String,
        progress_percentage: f32,
    },

    #[error("Engine error: {0}")]
    Engine(String),

    #[error("Not connected to editor engine")]
    NotConnected(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl McpError {
    pub fn error_code(&self) -> i32 {
        match self {
            McpError::InvalidParams(_) => -32602,
            McpError::ClipNotFound { .. } => -32001,
            McpError::TrackNotFound { .. } => -32002,
            McpError::OverlapConflict(_) => -32003,
            McpError::TrimBeyondMedia { .. } => -32004,
            McpError::SafetyGuardTriggered { .. } => -32005,
            McpError::MediaNotReady { .. } => -32006,
            McpError::Engine(_) => -32010,
            McpError::NotConnected(_) => -32011,
            McpError::Internal(_) => -32603,
        }
    }
}
