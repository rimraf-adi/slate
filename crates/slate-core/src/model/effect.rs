use crate::id::EffectId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub id: EffectId,
    /// e.g. "color_grading", "chroma_key", "gaussian_blur", "lut_apply", "equalizer"
    pub effect_type: String,
    pub name: String,
    pub enabled: bool,
    pub parameters: serde_json::Value,
}
