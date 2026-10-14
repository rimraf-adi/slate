use crate::id::CaptionId;
use crate::time::TimeRange;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaptionStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub text_color_rgba: [f32; 4],
    pub bg_color_rgba: [f32; 4],
    pub outline_color_rgba: [f32; 4],
    pub outline_width: f32,
    pub vertical_alignment: String, // "top", "center", "bottom"
    pub margin_bottom_percent: f32,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(),
            font_size: 36.0,
            font_weight: 700,
            text_color_rgba: [1.0, 1.0, 1.0, 1.0],
            bg_color_rgba: [0.0, 0.0, 0.0, 0.75],
            outline_color_rgba: [0.0, 0.0, 0.0, 1.0],
            outline_width: 2.0,
            vertical_alignment: "bottom".to_string(),
            margin_bottom_percent: 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    pub id: CaptionId,
    pub time_range: TimeRange,
    pub text: String,
    pub style: CaptionStyle,
}
