use super::types::KeyframeProperty;
use crate::id::{CaptionId, ClipId, EffectId, MarkerId, MediaId, TrackId, TransitionId};
use crate::model::{
    AudioParams, Caption, CaptionStyle, Clip, Effect, Keyframe, KeyframeTrack, Marker,
    MediaReference, Track, TrackItem, Transform, Transition,
};
use crate::time::{RationalTime, TimeRange};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OpInverse {
    // Media
    UnimportMedia {
        media_id: MediaId,
    },
    ReimportMedia {
        reference: MediaReference,
    },

    // Track
    RemoveAddedTrack {
        track_id: TrackId,
    },
    RestoreRemovedTrack {
        index: usize,
        track: Track,
    },
    RevertReorderTrack {
        track_id: TrackId,
        original_index: usize,
    },
    RevertTrackState {
        track_id: TrackId,
        muted: bool,
        solo: bool,
        locked: bool,
    },

    // Clip
    RemoveAddedClip {
        clip_id: ClipId,
    },
    RestoreRemovedClip {
        track_id: TrackId,
        index: usize,
        clip: Clip,
    },
    RevertMoveClip {
        clip_id: ClipId,
        original_track: TrackId,
        original_time: RationalTime,
    },
    RevertTrimClip {
        clip_id: ClipId,
        original_source: TimeRange,
        original_timeline: TimeRange,
    },
    UnsplitClip {
        kept_clip_id: ClipId,
        removed_clip_id: ClipId,
        original_source_duration: RationalTime,
    },
    RevertSlipClip {
        clip_id: ClipId,
        delta: RationalTime,
    },
    RevertSlideClip {
        clip_id: ClipId,
        delta: RationalTime,
    },
    RevertClipSpeed {
        clip_id: ClipId,
        original_speed: f64,
    },

    // Ripple
    RestoreRipple {
        restored_tracks: Vec<(TrackId, Vec<TrackItem>)>,
    },

    // Transitions
    RemoveAddedTransition {
        transition_id: TransitionId,
    },
    RestoreRemovedTransition {
        track_id: TrackId,
        after_clip_id: ClipId,
        transition: Transition,
    },

    // Effects
    RemoveAppliedEffect {
        clip_id: ClipId,
        effect_id: EffectId,
    },
    RestoreRemovedEffect {
        clip_id: ClipId,
        index: usize,
        effect: Effect,
    },
    RevertUpdateEffect {
        clip_id: ClipId,
        effect_id: EffectId,
        original_params: serde_json::Value,
    },

    // Transform & Keyframes
    RevertTransform {
        clip_id: ClipId,
        original_transform: Transform,
    },
    RestoreRemovedKeyframe {
        clip_id: ClipId,
        property: KeyframeProperty,
        keyframe: Keyframe<f64>,
    },
    RemoveAddedKeyframe {
        clip_id: ClipId,
        property: KeyframeProperty,
        time: RationalTime,
    },

    // Audio
    RevertAudioParams {
        clip_id: ClipId,
        original_params: AudioParams,
    },
    RevertSilenceRemoval {
        track_id: TrackId,
        original_items: Vec<TrackItem>,
    },
    RevertAudioDuck {
        music_track: TrackId,
        original_volume_keyframes: KeyframeTrack<f64>,
    },

    // Captions & Markers
    RemoveAddedCaption {
        caption_id: CaptionId,
    },
    RestoreRemovedCaption {
        caption: Caption,
    },
    RevertUpdateCaption {
        caption_id: CaptionId,
        original_text: String,
        original_style: CaptionStyle,
    },
    RemoveAddedMarker {
        marker_id: MarkerId,
    },
    RestoreRemovedMarker {
        marker: Marker,
    },

    // Batch Inverse
    BatchInverse {
        inverses: Vec<OpInverse>,
        label: String,
    },
}
