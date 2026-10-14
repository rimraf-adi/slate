use crate::id::{CaptionId, ClipId, EffectId, MarkerId, MediaId, TimelineId, TrackId, TransitionId};
use crate::model::TrackType;
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "error", rename_all = "camelCase")]
pub enum OpError {
    #[error("Timeline with ID '{timeline_id}' was not found in the project")]
    TimelineNotFound { timeline_id: TimelineId },

    #[error("Clip with ID '{clip_id}' was not found on any track")]
    ClipNotFound { clip_id: ClipId },

    #[error("Track with ID '{track_id}' was not found in the timeline")]
    TrackNotFound { track_id: TrackId },

    #[error("Media with ID '{media_id}' has not been imported into the project media pool")]
    MediaNotImported { media_id: MediaId },

    #[error("Transition with ID '{transition_id}' was not found")]
    TransitionNotFound { transition_id: TransitionId },

    #[error("Effect with ID '{effect_id}' was not found on clip '{clip_id}'")]
    EffectNotFound {
        effect_id: EffectId,
        clip_id: ClipId,
    },

    #[error("Marker with ID '{marker_id}' was not found")]
    MarkerNotFound { marker_id: MarkerId },

    #[error("Caption with ID '{caption_id}' was not found")]
    CaptionNotFound { caption_id: CaptionId },

    #[error("Track '{track_id}' is locked and cannot be modified")]
    TrackLocked { track_id: TrackId },

    #[error("Requested time range [{requested_start}, {requested_end}) conflicts with existing clip '{conflicting_clip_id}'")]
    OverlapConflict {
        conflicting_clip_id: ClipId,
        requested_start: RationalTime,
        requested_end: RationalTime,
    },

    #[error("Trim would exceed media source boundaries: requested {requested}, maximum valid duration is {available}")]
    TrimBeyondMedia {
        clip_id: ClipId,
        requested: RationalTime,
        available: RationalTime,
    },

    #[error("Split position {at} falls outside the clip's timeline range [{start}, {end})")]
    SplitOutsideClip {
        clip_id: ClipId,
        at: RationalTime,
        start: RationalTime,
        end: RationalTime,
    },

    #[error("Cannot add {track_type:?} clip to incompatible track {track_id}")]
    TrackTypeMismatch {
        track_id: TrackId,
        track_type: TrackType,
    },

    #[error("Invalid speed multiplier {speed}; must be non-zero and between -100.0 and 100.0")]
    InvalidSpeed { speed: f64 },

    #[error("Batch operation failed at sub-operation #{index}: {cause}")]
    BatchFailed {
        index: usize,
        cause: Box<OpError>,
    },
}
