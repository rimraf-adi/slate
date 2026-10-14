use crate::types::time::TimeResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSummaryResponse {
    pub project_name: String,
    pub fps: f64,
    pub resolution: [u32; 2],
    pub duration: TimeResponse,
    pub track_count: usize,
    pub clip_count: usize,
    pub tracks: Vec<TrackSummary>,
    pub clips: Vec<ClipSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub id: String,
    pub name: String,
    pub track_type: String,
    pub clip_count: usize,
    pub muted: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipSummary {
    pub id: String,
    pub name: String,
    pub track_id: String,
    pub media_id: String,
    pub timeline_start: TimeResponse,
    pub timeline_duration: TimeResponse,
    pub source_start: TimeResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MutationDiffSummary {
    pub clips_added: usize,
    pub clips_removed: usize,
    pub clips_modified: usize,
    pub duration_change_seconds: f64,
    pub total_duration_seconds: f64,
    pub mutation_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub progress_percentage: f32,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryResponse {
    pub index: usize,
    pub label: String,
    pub source: String,
    pub timestamp: String,
    pub can_undo: bool,
}
