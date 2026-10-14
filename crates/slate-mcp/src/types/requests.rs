use crate::types::time::TimeArg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimeRangeArg {
    pub start: TimeArg,
    pub duration: TimeArg,
}

// 1. Project
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectNewArgs {
    #[schemars(description = "Human-readable project name")]
    pub name: String,
    #[schemars(description = "Timeline frame rate (e.g. 24, 25, 30, 60)")]
    pub fps: Option<f64>,
    #[schemars(description = "Timeline resolution [width, height] in pixels")]
    pub resolution: Option<[u32; 2]>,
    #[schemars(description = "Optional destination file path ending in .slateproj")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectOpenArgs {
    #[schemars(description = "Path to the .slateproj directory on disk")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSaveArgs {
    #[schemars(description = "Optional target path to save the project to")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineSummaryArgs {}

// 2. Media
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaImportArgs {
    #[schemars(description = "Path to the media file on disk to import")]
    pub path: String,
    #[schemars(description = "Whether to immediately trigger speech transcription in background")]
    pub transcribe: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaListArgs {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaProbeArgs {
    #[schemars(description = "Media asset ID or file path to probe")]
    pub path_or_id: String,
}

// 3. Perception
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaTranscriptArgs {
    #[schemars(description = "ID of the media asset to retrieve transcript for")]
    pub media_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaScenesArgs {
    #[schemars(description = "ID of the video media asset to analyze")]
    pub media_id: String,
    #[schemars(description = "Histogram change sensitivity threshold [0.0 - 1.0] (default: 0.35)")]
    pub threshold: Option<f32>,
    #[schemars(description = "Minimum frame duration between cuts (default: 15)")]
    pub min_scene_length_frames: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaSilencesArgs {
    #[schemars(description = "ID of the audio/video media asset to analyze")]
    pub media_id: String,
    #[schemars(description = "Silence energy threshold [0.0 - 1.0] (default: 0.5)")]
    pub threshold: Option<f32>,
    #[schemars(description = "Minimum silence duration in seconds to report (default: 0.5)")]
    pub min_silence_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaLoudnessArgs {
    #[schemars(description = "ID of the media asset to analyze")]
    pub media_id: String,
    #[schemars(description = "Target loudness standard in LUFS (default: -14.0 for YouTube/web, -24.0 for broadcast)")]
    pub target_lufs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RenderFrameArgs {
    #[schemars(description = "Timeline position to render (e.g. '00:01:23:15', '4.5s', or '120f')")]
    pub time: TimeArg,
    #[schemars(description = "Desired preview width in pixels (default: timeline width)")]
    pub width: Option<u32>,
    #[schemars(description = "Desired preview height in pixels (default: timeline height)")]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RenderContactSheetArgs {
    #[schemars(description = "ID of the media asset to generate contact sheet for")]
    pub media_id: String,
    #[schemars(description = "Number of columns in grid (default: 4)")]
    pub cols: Option<usize>,
    #[schemars(description = "Number of rows in grid (default: 4)")]
    pub rows: Option<usize>,
    #[schemars(description = "Thumbnail width (default: 320)")]
    pub thumb_width: Option<u32>,
    #[schemars(description = "Thumbnail height (default: 180)")]
    pub thumb_height: Option<u32>,
}

// 4. Editing
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipAddArgs {
    #[schemars(description = "Track ID to place the clip onto")]
    pub track_id: String,
    #[schemars(description = "Media asset ID to instantiate")]
    pub media_id: String,
    #[schemars(description = "Optional custom name for the clip")]
    pub name: Option<String>,
    #[schemars(description = "Source start point (default: 0)")]
    pub source_start: Option<TimeArg>,
    #[schemars(description = "Source duration to take (e.g. '5.0s')")]
    pub source_duration: TimeArg,
    #[schemars(description = "Timeline placement start position")]
    pub timeline_position: TimeArg,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipTrimArgs {
    #[schemars(description = "Clip ID to trim")]
    pub clip_id: String,
    #[schemars(description = "Edge to adjust: 'start' (in-point) or 'end' (out-point)")]
    pub edge: String,
    #[schemars(description = "Trim offset delta (e.g. '+1.5s', '-10f')")]
    pub delta: TimeArg,
    #[schemars(description = "Trim mode: 'ripple' (shifts neighbors), 'roll', 'slip', 'slide' (default: 'ripple')")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipSplitArgs {
    #[schemars(description = "Clip ID to split (blade cut)")]
    pub clip_id: String,
    #[schemars(description = "Timeline time point at which to split")]
    pub at: TimeArg,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipMoveArgs {
    #[schemars(description = "Clip ID to move")]
    pub clip_id: String,
    #[schemars(description = "Destination track ID")]
    pub target_track: String,
    #[schemars(description = "Destination start position on timeline")]
    pub target_time: TimeArg,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipRemoveArgs {
    #[schemars(description = "Clip ID to remove from timeline")]
    pub clip_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RippleDeleteRangesArgs {
    #[schemars(description = "Array of time ranges to delete and close gap")]
    pub ranges: Vec<TimeRangeArg>,
    #[schemars(description = "Optional track IDs to restrict deletion to (default: all tracks)")]
    pub track_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrackAddArgs {
    #[schemars(description = "Track type: 'video' or 'audio'")]
    pub track_type: String,
    #[schemars(description = "Name for the track (e.g. 'V2' or 'Voice')")]
    pub name: String,
    #[schemars(description = "Optional index position to insert at")]
    pub index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransitionAddArgs {
    #[schemars(description = "Track ID containing the clips")]
    pub track_id: String,
    #[schemars(description = "Clip ID preceding the transition")]
    pub after_clip_id: String,
    #[schemars(description = "Transition type: 'cross_dissolve', 'dip_to_black', 'dip_to_white', or 'audio_crossfade'")]
    pub transition_type: String,
    #[schemars(description = "Transition duration (e.g. '1.0s' or '30f')")]
    pub duration: TimeArg,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EffectApplyArgs {
    #[schemars(description = "Clip ID to attach effect to")]
    pub clip_id: String,
    #[schemars(description = "Effect type: 'color_grade', 'blur', 'chroma_key', etc.")]
    pub effect_type: String,
    #[schemars(description = "Effect parameter dictionary (exposure, contrast, saturation, etc.)")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeyframeSetArgs {
    #[schemars(description = "Clip ID")]
    pub clip_id: String,
    #[schemars(description = "Property name: 'position_x', 'position_y', 'scale_x', 'scale_y', 'rotation', 'opacity', 'volume', 'pan'")]
    pub property: String,
    #[schemars(description = "Timestamp relative to clip start")]
    pub time: TimeArg,
    #[schemars(description = "Numerical property value")]
    pub value: f64,
}

// 5. Text-Based Editing
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditByTranscriptArgs {
    #[schemars(description = "Media asset ID whose transcript is referenced")]
    pub media_id: String,
    #[schemars(description = "Timeline track ID")]
    pub track_id: String,
    #[schemars(description = "Clip ID on the track to cut")]
    pub clip_id: String,
    #[schemars(description = "Array of [start_word_index, end_word_index] pairs to KEEP")]
    pub keep_word_ranges: Vec<[usize; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CaptionsGenerateArgs {
    #[schemars(description = "Media asset ID to generate captions from")]
    pub media_id: String,
    #[schemars(description = "Font size in points (default: 36.0)")]
    pub font_size: Option<f32>,
    #[schemars(description = "Subtitle text color hex string (default: '#FFFF00')")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CaptionsStyleArgs {
    #[schemars(description = "Caption track or block ID")]
    pub caption_id: String,
    #[schemars(description = "Font size in points")]
    pub font_size: Option<f32>,
    #[schemars(description = "Primary font color hex")]
    pub primary_color: Option<String>,
}

// 6. Audio
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AudioDuckArgs {
    #[schemars(description = "Track ID containing background music")]
    pub music_track_id: String,
    #[schemars(description = "Track ID containing speech / voice")]
    pub voice_track_id: String,
    #[schemars(description = "Attenuation level in dB (default: -16.0)")]
    pub duck_level_db: Option<f64>,
    #[schemars(description = "Voice detection threshold in dB (default: -24.0)")]
    pub threshold_db: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AudioNormalizeArgs {
    #[schemars(description = "Clip ID to normalize")]
    pub clip_id: String,
    #[schemars(description = "Target LUFS level (default: -14.0)")]
    pub target_lufs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AudioRemoveSilenceArgs {
    #[schemars(description = "Track ID containing audio")]
    pub track_id: String,
    #[schemars(description = "Clip ID to remove silence from")]
    pub clip_id: String,
    #[schemars(description = "Silence energy threshold [0.0 - 1.0] (default: 0.5)")]
    pub threshold: Option<f32>,
    #[schemars(description = "Minimum silence pause duration in seconds (default: 0.5)")]
    pub min_silence_seconds: Option<f64>,
}

// 7. Batch
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpsApplyArgs {
    #[schemars(description = "List of serialized engine operations to execute atomically")]
    pub ops: Vec<serde_json::Value>,
    #[schemars(description = "Human-readable label for this edit batch in history")]
    pub label: Option<String>,
    #[schemars(description = "If true, simulates edit and returns diff without mutating timeline")]
    pub dry_run: Option<bool>,
    #[schemars(description = "Must be true if mutation exceeds safety threshold (>25% timeline)")]
    pub confirm: Option<bool>,
}

// 8. History
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HistoryListArgs {
    #[schemars(description = "Maximum number of history entries to return (default: 20)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UndoArgs {
    #[schemars(description = "Number of steps to undo (default: 1)")]
    pub steps: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RedoArgs {
    #[schemars(description = "Number of steps to redo (default: 1)")]
    pub steps: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RevertOpArgs {
    #[schemars(description = "0-based index in history stack of the operation to selectively invert")]
    pub history_index: usize,
}

// 9. Export
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportStartArgs {
    #[schemars(description = "Output destination file path (e.g. 'export.mp4')")]
    pub output_path: String,
    #[schemars(description = "Preset name: 'youtube_1080p', 'tiktok_vertical', 'prores_master', or 'audio_only'")]
    pub preset: Option<String>,
    #[schemars(description = "Output resolution [width, height]")]
    pub resolution: Option<[u32; 2]>,
    #[schemars(description = "Output frame rate (default: project rate)")]
    pub fps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobStatusArgs {
    #[schemars(description = "Export job ID")]
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobCancelArgs {
    #[schemars(description = "Export job ID to cancel")]
    pub job_id: String,
}
