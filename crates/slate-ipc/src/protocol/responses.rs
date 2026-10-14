use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use slate_core::history::HistoryEntry;
use slate_core::id::{MediaId, TimelineId};
use slate_core::model::{MediaMetadata, MediaReference, Timeline};
use slate_core::time::RationalTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOpResult {
    pub ok: bool,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplyBatchResult {
    pub ok: bool,
    pub applied_count: usize,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UndoResult {
    pub ok: bool,
    pub undone_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RedoResult {
    pub ok: bool,
    pub redone_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevertResult {
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListResult {
    pub can_undo: bool,
    pub can_redo: bool,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimelineResult {
    pub timeline: Timeline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimelineClipSummary {
    pub clip_id: String,
    pub name: String,
    pub start: RationalTime,
    pub duration: RationalTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimelineTrackSummary {
    pub track_id: String,
    pub name: String,
    pub track_type: String,
    pub clips: Vec<TimelineClipSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSummaryResult {
    pub timeline_id: TimelineId,
    pub name: String,
    pub duration: RationalTime,
    pub tracks: Vec<TimelineTrackSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaListResult {
    pub items: Vec<MediaReference>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMediaResult {
    pub metadata: MediaMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportMediaResult {
    pub media_id: MediaId,
    pub media_ref: MediaReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackResult {
    pub is_playing: bool,
    pub speed: f64,
    pub position: RationalTime,
}
