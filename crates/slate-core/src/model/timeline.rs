use super::caption::Caption;
use super::clip::Clip;
use super::marker::Marker;
use super::track::Track;
use crate::id::{ClipId, TimelineId, TrackId};
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSettings {
    pub timebase: u32,
    pub width: u32,
    pub height: u32,
    pub drop_frame: bool,
    pub sample_rate: u32,
}

impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            timebase: 30,
            width: 1920,
            height: 1080,
            drop_frame: false,
            sample_rate: 48000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub id: TimelineId,
    pub name: String,
    pub settings: TimelineSettings,
    pub tracks: Vec<Track>,
    pub markers: Vec<Marker>,
    pub captions: Vec<Caption>,
}

impl Timeline {
    pub fn new(name: impl Into<String>, settings: TimelineSettings) -> Self {
        Self {
            id: TimelineId::new(),
            name: name.into(),
            settings,
            tracks: Vec::new(),
            markers: Vec::new(),
            captions: Vec::new(),
        }
    }

    /// Finds a track by its ID
    pub fn find_track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    /// Finds a mutable track by its ID
    pub fn find_track_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    /// Searches across all tracks for a clip by its ID
    pub fn find_clip(&self, id: ClipId) -> Option<(&Track, &Clip)> {
        for track in &self.tracks {
            if let Some(clip) = track.find_clip(id) {
                return Some((track, clip));
            }
        }
        None
    }

    /// Total timeline duration (maximum end time among all tracks)
    pub fn duration(&self) -> RationalTime {
        self.tracks
            .iter()
            .map(|t| t.duration())
            .max()
            .unwrap_or(RationalTime::ZERO)
    }
}
