use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreset {
    pub name: String,
    pub container: String,     // "mp4", "mov", "mkv"
    pub video_codec: String,   // "h264", "hevc", "prores422"
    pub audio_codec: String,   // "aac", "pcm_s16le"
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
}

impl Default for ExportPreset {
    fn default() -> Self {
        Self {
            name: "1080p60 Web H.264".to_string(),
            container: "mp4".to_string(),
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
            width: 1920,
            height: 1080,
            fps: 60.0,
            video_bitrate_kbps: 12000,
            audio_bitrate_kbps: 192,
        }
    }
}
