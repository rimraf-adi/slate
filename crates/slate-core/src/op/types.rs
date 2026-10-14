use crate::id::{CaptionId, ClipId, EffectId, MarkerId, MediaId, TrackId, TransitionId};
use crate::model::{
    AudioParams, Caption, CaptionStyle, Clip, Effect, Keyframe, Marker, TrackType, Transform,
    Transition,
};
use crate::time::{RationalTime, TimeRange};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrimEdge {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrimMode {
    /// Overwrites / trims clip in place; creates or expands adjacent gaps
    Ripple,
    /// Shifts adjacent clips to close gaps or accommodate expansion
    Roll,
    /// Modifies source in/out while keeping timeline range fixed
    Slip,
    /// Modifies timeline in/out while keeping source range and duration fixed
    Slide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WordRange {
    pub word: String,
    pub start: RationalTime,
    pub end: RationalTime,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuckParams {
    pub duck_level_db: f64,     // e.g. -18.0 dB
    pub threshold_db: f64,      // Voice detection threshold, e.g. -24.0 dB
    pub attack_time: RationalTime,
    pub release_time: RationalTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum KeyframeProperty {
    PositionX,
    PositionY,
    ScaleX,
    ScaleY,
    Rotation,
    Opacity,
    Volume,
    Pan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum Op {
    // Media Pool Operations
    ImportMedia {
        path: PathBuf,
        media_id: MediaId,
    },
    RemoveMedia {
        media_id: MediaId,
    },

    // Track Operations
    AddTrack {
        track_type: TrackType,
        name: String,
        index: Option<usize>,
    },
    RemoveTrack {
        track_id: TrackId,
    },
    ReorderTrack {
        track_id: TrackId,
        new_index: usize,
    },
    SetTrackMute {
        track_id: TrackId,
        muted: bool,
    },
    SetTrackSolo {
        track_id: TrackId,
        solo: bool,
    },
    SetTrackLock {
        track_id: TrackId,
        locked: bool,
    },

    // Clip Editing & Manipulation
    AddClip {
        track_id: TrackId,
        clip: Clip,
        position: RationalTime,
    },
    RemoveClip {
        clip_id: ClipId,
    },
    MoveClip {
        clip_id: ClipId,
        target_track: TrackId,
        target_time: RationalTime,
    },
    TrimClip {
        clip_id: ClipId,
        edge: TrimEdge,
        delta: RationalTime,
        mode: TrimMode,
    },
    SplitClip {
        clip_id: ClipId,
        at: RationalTime,
        new_clip_id: ClipId,
    },
    SlipClip {
        clip_id: ClipId,
        delta: RationalTime,
    },
    SlideClip {
        clip_id: ClipId,
        delta: RationalTime,
    },
    SetClipSpeed {
        clip_id: ClipId,
        speed: f64,
    },

    // Timeline-level Operations
    RippleDelete {
        ranges: Vec<TimeRange>,
        track_ids: Option<Vec<TrackId>>,
    },

    // Transitions
    AddTransition {
        track_id: TrackId,
        after_clip_id: ClipId,
        transition: Transition,
    },
    RemoveTransition {
        transition_id: TransitionId,
    },

    // Effects
    ApplyEffect {
        clip_id: ClipId,
        effect: Effect,
    },
    RemoveEffect {
        clip_id: ClipId,
        effect_id: EffectId,
    },
    UpdateEffect {
        clip_id: ClipId,
        effect_id: EffectId,
        params: serde_json::Value,
    },

    // Keyframes & Spatial Transform
    SetTransform {
        clip_id: ClipId,
        transform: Transform,
    },
    SetKeyframe {
        clip_id: ClipId,
        property: KeyframeProperty,
        keyframe: Keyframe<f64>,
    },
    RemoveKeyframe {
        clip_id: ClipId,
        property: KeyframeProperty,
        time: RationalTime,
    },

    // Audio Processing
    SetAudioParams {
        clip_id: ClipId,
        params: AudioParams,
    },
    NormalizeAudio {
        clip_id: ClipId,
        target_lufs: f64,
    },
    RemoveSilence {
        track_id: TrackId,
        threshold_db: f64,
        min_duration: RationalTime,
        padding: RationalTime,
    },
    AudioDuck {
        music_track: TrackId,
        voice_track: TrackId,
        params: DuckParams,
    },

    // Captions & Subtitles
    AddCaption {
        caption: Caption,
    },
    RemoveCaption {
        caption_id: CaptionId,
    },
    UpdateCaption {
        caption_id: CaptionId,
        text: Option<String>,
        style: Option<CaptionStyle>,
    },
    GenerateCaptions {
        media_id: MediaId,
        style: CaptionStyle,
    },

    // Markers
    AddMarker {
        marker: Marker,
    },
    RemoveMarker {
        marker_id: MarkerId,
    },

    // Multimodal & Text-Based Editing
    EditByTranscript {
        media_id: MediaId,
        track_id: TrackId,
        keep_ranges: Vec<WordRange>,
    },

    // Batch Composition
    Batch {
        ops: Vec<Op>,
        label: String,
    },
}
