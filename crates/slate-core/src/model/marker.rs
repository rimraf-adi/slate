use crate::id::MarkerId;
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub id: MarkerId,
    pub time: RationalTime,
    pub label: String,
    pub note: Option<String>,
    pub color_rgba: [f32; 4],
}

impl Marker {
    pub fn new(time: RationalTime, label: impl Into<String>) -> Self {
        Self {
            id: MarkerId::new(),
            time,
            label: label.into(),
            note: None,
            color_rgba: [0.2, 0.6, 1.0, 1.0], // Default sky blue
        }
    }
}
