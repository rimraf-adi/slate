use crate::id::TransitionId;
use crate::time::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TransitionAlignment {
    StartAtCut,
    CenterAtCut,
    EndAtCut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub id: TransitionId,
    /// e.g. "cross_dissolve", "dip_to_black", "slide", "wipe"
    pub transition_type: String,
    pub duration: RationalTime,
    pub alignment: TransitionAlignment,
    pub parameters: serde_json::Value,
}
