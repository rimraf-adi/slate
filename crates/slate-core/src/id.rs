use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generates a new random unique identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Creates an identifier wrapping an existing UUID.
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the inner UUID reference.
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_id!(ProjectId, "Unique identifier for a project container.");
define_id!(TimelineId, "Unique identifier for a timeline sequence.");
define_id!(TrackId, "Unique identifier for a timeline track.");
define_id!(ClipId, "Unique identifier for a timeline clip.");
define_id!(GapId, "Unique identifier for an empty gap interval.");
define_id!(TransitionId, "Unique identifier for an in/out or cross transition.");
define_id!(MediaId, "Unique identifier for imported source media asset.");
define_id!(EffectId, "Unique identifier for an applied audio or video effect instance.");
define_id!(MarkerId, "Unique identifier for a timeline marker annotation.");
define_id!(CaptionId, "Unique identifier for an aligned caption item.");
define_id!(KeyframeId, "Unique identifier for an animation keyframe point.");
define_id!(JobId, "Unique identifier for an asynchronous task (render, proxy, transcription).");

/// Fast O(1) entity lookup index mapping ClipId to containing TrackId and position.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimelineIndex {
    /// Maps clip ID to the parent track ID containing it
    pub clip_to_track: HashMap<ClipId, TrackId>,
    /// Maps effect ID to the clip owning it
    pub effect_to_clip: HashMap<EffectId, ClipId>,
    /// Maps transition ID to the track containing it
    pub transition_to_track: HashMap<TransitionId, TrackId>,
}

impl TimelineIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_clip(&mut self, clip_id: ClipId, track_id: TrackId) {
        self.clip_to_track.insert(clip_id, track_id);
    }

    pub fn remove_clip(&mut self, clip_id: &ClipId) -> Option<TrackId> {
        self.clip_to_track.remove(clip_id)
    }

    pub fn insert_effect(&mut self, effect_id: EffectId, clip_id: ClipId) {
        self.effect_to_clip.insert(effect_id, clip_id);
    }

    pub fn remove_effect(&mut self, effect_id: &EffectId) -> Option<ClipId> {
        self.effect_to_clip.remove(effect_id)
    }

    pub fn insert_transition(&mut self, transition_id: TransitionId, track_id: TrackId) {
        self.transition_to_track.insert(transition_id, track_id);
    }

    pub fn remove_transition(&mut self, transition_id: &TransitionId) -> Option<TrackId> {
        self.transition_to_track.remove(transition_id)
    }

    pub fn clear(&mut self) {
        self.clip_to_track.clear();
        self.effect_to_clip.clear();
        self.transition_to_track.clear();
    }
}
