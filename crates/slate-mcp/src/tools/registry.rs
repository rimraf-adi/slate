use crate::types::requests::*;
use schemars::schema_for;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolCallResult {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: None,
        }
    }

    pub fn json<T: Serialize>(val: &T) -> Self {
        let text = serde_json::to_string_pretty(val).unwrap_or_else(|e| e.to_string());
        Self::text(text)
    }

    pub fn image_base64(data: String, mime_type: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Image {
                data,
                mime_type: mime_type.into(),
            }],
            is_error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: msg.into() }],
            is_error: Some(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Project
        ToolDefinition {
            name: "project_new".to_string(),
            description: "Create a new Slate project with custom FPS, resolution, and track configuration.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ProjectNewArgs)).unwrap(),
        },
        ToolDefinition {
            name: "project_open".to_string(),
            description: "Open an existing .slateproj project from disk.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ProjectOpenArgs)).unwrap(),
        },
        ToolDefinition {
            name: "project_save".to_string(),
            description: "Save active project to disk.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ProjectSaveArgs)).unwrap(),
        },
        ToolDefinition {
            name: "timeline_summary".to_string(),
            description: "Get a token-light summary of the project timeline: tracks, duration, and clip in/out boundaries.".to_string(),
            input_schema: serde_json::to_value(schema_for!(TimelineSummaryArgs)).unwrap(),
        },
        // Media
        ToolDefinition {
            name: "media_import".to_string(),
            description: "Import a media asset (video, audio, or image) into the project pool.".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaImportArgs)).unwrap(),
        },
        ToolDefinition {
            name: "media_list".to_string(),
            description: "List all imported media assets in the project media pool.".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaListArgs)).unwrap(),
        },
        ToolDefinition {
            name: "media_probe".to_string(),
            description: "Probe metadata (codecs, duration, resolution, audio channels) for a media asset or file.".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaProbeArgs)).unwrap(),
        },
        // Perception
        ToolDefinition {
            name: "media_transcript".to_string(),
            description: "Retrieve word-level timestamped speech transcript for a media asset.".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaTranscriptArgs)).unwrap(),
        },
        ToolDefinition {
            name: "media_scenes".to_string(),
            description: "Detect visual scene cuts and boundaries in a video asset using color histograms.".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaScenesArgs)).unwrap(),
        },
        ToolDefinition {
            name: "media_silences".to_string(),
            description: "Detect pauses and dead air in audio or video media using voice activity detection.".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaSilencesArgs)).unwrap(),
        },
        ToolDefinition {
            name: "media_loudness".to_string(),
            description: "Analyze audio loudness according to ITU-R BS.1770 / EBU R128 standards (integrated LUFS, true peak).".to_string(),
            input_schema: serde_json::to_value(schema_for!(MediaLoudnessArgs)).unwrap(),
        },
        ToolDefinition {
            name: "render_frame".to_string(),
            description: "Render a frame of the timeline offscreen and return base64 PNG image.".to_string(),
            input_schema: serde_json::to_value(schema_for!(RenderFrameArgs)).unwrap(),
        },
        ToolDefinition {
            name: "render_contact_sheet".to_string(),
            description: "Generate a multi-frame contact sheet grid for visual pacing inspection.".to_string(),
            input_schema: serde_json::to_value(schema_for!(RenderContactSheetArgs)).unwrap(),
        },
        // Editing
        ToolDefinition {
            name: "clip_add".to_string(),
            description: "Add a media clip onto a timeline track at a given time position.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ClipAddArgs)).unwrap(),
        },
        ToolDefinition {
            name: "clip_trim".to_string(),
            description: "Trim clip in-point or out-point using ripple, roll, slip, or slide modes.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ClipTrimArgs)).unwrap(),
        },
        ToolDefinition {
            name: "clip_split".to_string(),
            description: "Split a clip at the specified timeline position (blade cut).".to_string(),
            input_schema: serde_json::to_value(schema_for!(ClipSplitArgs)).unwrap(),
        },
        ToolDefinition {
            name: "clip_move".to_string(),
            description: "Move a clip to another track or timeline time position.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ClipMoveArgs)).unwrap(),
        },
        ToolDefinition {
            name: "clip_remove".to_string(),
            description: "Remove a clip from the timeline.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ClipRemoveArgs)).unwrap(),
        },
        ToolDefinition {
            name: "ripple_delete_ranges".to_string(),
            description: "Delete time ranges from timeline and ripple shift subsequent clips to close gaps.".to_string(),
            input_schema: serde_json::to_value(schema_for!(RippleDeleteRangesArgs)).unwrap(),
        },
        ToolDefinition {
            name: "track_add".to_string(),
            description: "Add a new video or audio track to the timeline.".to_string(),
            input_schema: serde_json::to_value(schema_for!(TrackAddArgs)).unwrap(),
        },
        ToolDefinition {
            name: "transition_add".to_string(),
            description: "Add a transition between adjacent clips (e.g. cross dissolve).".to_string(),
            input_schema: serde_json::to_value(schema_for!(TransitionAddArgs)).unwrap(),
        },
        ToolDefinition {
            name: "effect_apply".to_string(),
            description: "Apply a visual effect (color grade, blur, etc.) to a clip.".to_string(),
            input_schema: serde_json::to_value(schema_for!(EffectApplyArgs)).unwrap(),
        },
        ToolDefinition {
            name: "keyframe_set".to_string(),
            description: "Set a keyframe value on a clip parameter (opacity, volume, pan, transform).".to_string(),
            input_schema: serde_json::to_value(schema_for!(KeyframeSetArgs)).unwrap(),
        },
        // Text-Based
        ToolDefinition {
            name: "edit_by_transcript".to_string(),
            description: "Cut timeline to retain only specified word index ranges from transcript dialogue.".to_string(),
            input_schema: serde_json::to_value(schema_for!(EditByTranscriptArgs)).unwrap(),
        },
        ToolDefinition {
            name: "captions_generate".to_string(),
            description: "Generate styled subtitles/captions automatically from speech transcript.".to_string(),
            input_schema: serde_json::to_value(schema_for!(CaptionsGenerateArgs)).unwrap(),
        },
        ToolDefinition {
            name: "captions_style".to_string(),
            description: "Update subtitle styling (font size, color, background).".to_string(),
            input_schema: serde_json::to_value(schema_for!(CaptionsStyleArgs)).unwrap(),
        },
        // Audio
        ToolDefinition {
            name: "audio_duck".to_string(),
            description: "Auto-duck music track under speech vocals.".to_string(),
            input_schema: serde_json::to_value(schema_for!(AudioDuckArgs)).unwrap(),
        },
        ToolDefinition {
            name: "audio_normalize".to_string(),
            description: "Normalize audio clip to target LUFS.".to_string(),
            input_schema: serde_json::to_value(schema_for!(AudioNormalizeArgs)).unwrap(),
        },
        ToolDefinition {
            name: "audio_remove_silence".to_string(),
            description: "Automatically remove dead air pauses from an audio track.".to_string(),
            input_schema: serde_json::to_value(schema_for!(AudioRemoveSilenceArgs)).unwrap(),
        },
        // Batch
        ToolDefinition {
            name: "ops_apply".to_string(),
            description: "Execute a batch of timeline operations atomically with optional dry-run preview.".to_string(),
            input_schema: serde_json::to_value(schema_for!(OpsApplyArgs)).unwrap(),
        },
        // History
        ToolDefinition {
            name: "history_list".to_string(),
            description: "List undo/redo operations history with source provenance.".to_string(),
            input_schema: serde_json::to_value(schema_for!(HistoryListArgs)).unwrap(),
        },
        ToolDefinition {
            name: "undo".to_string(),
            description: "Undo one or more operations on the timeline.".to_string(),
            input_schema: serde_json::to_value(schema_for!(UndoArgs)).unwrap(),
        },
        ToolDefinition {
            name: "redo".to_string(),
            description: "Redo one or more previously undone operations.".to_string(),
            input_schema: serde_json::to_value(schema_for!(RedoArgs)).unwrap(),
        },
        ToolDefinition {
            name: "revert_op".to_string(),
            description: "Selectively revert a specific historical operation by its index.".to_string(),
            input_schema: serde_json::to_value(schema_for!(RevertOpArgs)).unwrap(),
        },
        // Export
        ToolDefinition {
            name: "export_start".to_string(),
            description: "Start a background video render export job to MP4 or ProRes.".to_string(),
            input_schema: serde_json::to_value(schema_for!(ExportStartArgs)).unwrap(),
        },
        ToolDefinition {
            name: "job_status".to_string(),
            description: "Check the progress percentage and status of a running export job.".to_string(),
            input_schema: serde_json::to_value(schema_for!(JobStatusArgs)).unwrap(),
        },
        ToolDefinition {
            name: "job_cancel".to_string(),
            description: "Cancel an ongoing background export job.".to_string(),
            input_schema: serde_json::to_value(schema_for!(JobCancelArgs)).unwrap(),
        },
    ]
}
