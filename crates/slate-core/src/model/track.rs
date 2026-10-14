use super::clip::Clip;
use super::item::TrackItem;
use crate::id::{ClipId, TrackId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackType {
    Video,
    Audio,
    Caption,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub track_type: TrackType,
    pub items: Vec<TrackItem>,
    pub muted: bool,
    pub solo: bool,
    pub locked: bool,
    pub volume_db: f64,
    pub pan: f64,
    pub height: f32, // Display height in pixels in the UI
}

impl Track {
    pub fn new(name: impl Into<String>, track_type: TrackType) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            track_type,
            items: Vec::new(),
            muted: false,
            solo: false,
            locked: false,
            volume_db: 0.0,
            pan: 0.0,
            height: match track_type {
                TrackType::Video => 64.0,
                TrackType::Audio => 48.0,
                TrackType::Caption => 32.0,
            },
        }
    }

    /// Finds a clip by its unique ID
    pub fn find_clip(&self, id: ClipId) -> Option<&Clip> {
        self.items.iter().find_map(|item| match item {
            TrackItem::Clip(clip) if clip.id == id => Some(clip),
            _ => None,
        })
    }

    /// Finds a mutable clip by its unique ID
    pub fn find_clip_mut(&mut self, id: ClipId) -> Option<&mut Clip> {
        self.items.iter_mut().find_map(|item| match item {
            TrackItem::Clip(clip) if clip.id == id => Some(clip),
            _ => None,
        })
    }

    /// Returns the end time of the last item on the track
    pub fn duration(&self) -> crate::time::RationalTime {
        self.items
            .iter()
            .map(|item| item.timeline_range().end_time())
            .max()
            .unwrap_or(crate::time::RationalTime::ZERO)
    }
}
