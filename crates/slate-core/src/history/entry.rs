use crate::op::{Op, OpInverse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum HistoryError {
    #[error("Undo stack is empty")]
    UndoStackEmpty,
    #[error("Redo stack is empty")]
    RedoStackEmpty,
    #[error("History index out of bounds: {0}")]
    IndexOutOfBounds(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "sourceType", rename_all = "camelCase")]
pub enum OpSource {
    Human,
    Agent { session_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub op: Op,
    pub inverse: OpInverse,
    pub source: OpSource,
    pub timestamp: SystemTime,
    pub label: String,
}

impl HistoryEntry {
    pub fn new(op: Op, inverse: OpInverse, source: OpSource, label: impl Into<String>) -> Self {
        Self {
            op,
            inverse,
            source,
            timestamp: SystemTime::now(),
            label: label.into(),
        }
    }
}
