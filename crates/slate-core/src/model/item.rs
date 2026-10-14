use super::clip::Clip;
use super::transition::Transition;
use crate::id::GapId;
use crate::time::{RationalTime, TimeRange};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    pub id: GapId,
    pub timeline_range: TimeRange,
}

impl Gap {
    pub fn new(timeline_range: TimeRange) -> Self {
        Self {
            id: GapId::new(),
            timeline_range,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum TrackItem {
    Clip(Clip),
    Gap(Gap),
    Transition(Transition),
}

impl TrackItem {
    pub fn timeline_range(&self) -> TimeRange {
        match self {
            TrackItem::Clip(c) => c.timeline_range,
            TrackItem::Gap(g) => g.timeline_range,
            TrackItem::Transition(t) => TimeRange::new(RationalTime::ZERO, t.duration),
        }
    }
}
