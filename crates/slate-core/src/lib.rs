//! `slate-core` is the foundational headless domain engine for the Slate video editor.
//!
//! It provides:
//! - Frame-accurate rational time arithmetic (`RationalTime`, `TimeRange`, `Timecode`).
//! - Complete non-destructive timeline data model (`Project`, `Timeline`, `Track`, `Clip`, etc.).
//! - Invertible Command-pattern operations (`Op`, `OpApplicable`, `OpInverse`, `OpError`).
//! - Transactional undo/redo stack with human and AI agent provenance tracking (`History`).
//! - Crash-resilient write-ahead logging (WAL) and project directory bundle formats.
//! - Reactive event bus (`EventBus`, `EngineEvent`).
//! - Decoupled trait interfaces for media decoding, GPU compositing, audio mixing, and multimodal analysis.

pub mod engine;
pub mod events;
pub mod history;
pub mod id;
pub mod model;
pub mod op;
pub mod project;
pub mod time;
pub mod traits;

pub mod prelude {
    pub use crate::engine::Engine;
    pub use crate::events::{EngineEvent, EventBus, PlaybackState};
    pub use crate::history::{History, HistoryEntry, HistoryError, OpSource};
    pub use crate::id::*;
    pub use crate::model::*;
    pub use crate::op::*;
    pub use crate::project::*;
    pub use crate::time::*;
    pub use crate::traits::*;
}

pub use engine::Engine;
pub use events::{EngineEvent, EventBus, PlaybackState};
pub use history::{History, HistoryEntry, HistoryError, OpSource};
pub use id::*;
pub use model::*;
pub use op::*;
pub use project::*;
pub use time::*;
pub use traits::*;
