use super::export::ExportPreset;
use super::timeline::{Timeline, TimelineSettings};
use crate::id::{MediaId, ProjectId, TimelineId};
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub file_size_bytes: u64,
    pub duration: RationalTime,
    pub video_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f64>,
    pub timebase: u32,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u32>,
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaReference {
    pub id: MediaId,
    pub original_path: PathBuf,
    pub relative_path: PathBuf,
    pub metadata: MediaMetadata,
    pub proxy_path: Option<PathBuf>,
    pub waveform_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub id: ProjectId,
    pub name: String,
    pub active_timeline_id: TimelineId,
    pub media_pool: Vec<MediaReference>,
    pub timelines: Vec<Timeline>,
    pub export_presets: Vec<ExportPreset>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let initial_timeline = Timeline::new("Main Sequence", TimelineSettings::default());
        let active_timeline_id = initial_timeline.id;

        Self {
            schema_version: 1,
            id: ProjectId::new(),
            name: name.into(),
            active_timeline_id,
            media_pool: Vec::new(),
            timelines: vec![initial_timeline],
            export_presets: vec![ExportPreset::default()],
        }
    }

    pub fn active_timeline(&self) -> Option<&Timeline> {
        self.timelines.iter().find(|t| t.id == self.active_timeline_id)
    }

    pub fn active_timeline_mut(&mut self) -> Option<&mut Timeline> {
        self.timelines.iter_mut().find(|t| t.id == self.active_timeline_id)
    }

    pub fn active_timeline_and_media_mut(
        &mut self,
    ) -> Option<(&mut Timeline, &mut Vec<MediaReference>)> {
        let active_id = self.active_timeline_id;
        let timeline = self.timelines.iter_mut().find(|t| t.id == active_id)?;
        Some((timeline, &mut self.media_pool))
    }
}
