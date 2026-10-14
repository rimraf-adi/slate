use crate::events::{EngineEvent, EventBus};
use crate::history::{History, HistoryError, OpSource};
use crate::model::Project;
use crate::op::{Op, OpApplicable, OpError};
use crate::project::{JournalRecovery, JournalWriter};
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;

pub struct Engine {
    project: Project,
    history: History,
    event_bus: EventBus,
    journal: Option<JournalWriter>,
    bundle_path: Option<PathBuf>,
}

impl Engine {
    pub fn new(project: Project, history_capacity: usize) -> Self {
        Self {
            project,
            history: History::new(history_capacity),
            event_bus: EventBus::new(512),
            journal: None,
            bundle_path: None,
        }
    }

    pub fn open_bundle(bundle_dir: PathBuf) -> Result<Self, String> {
        let project_json_path = bundle_dir.join("project.json");
        let project_data = std::fs::read_to_string(&project_json_path)
            .map_err(|e| format!("Failed to read project.json: {}", e))?;

        let mut project: Project = serde_json::from_str(&project_data)
            .map_err(|e| format!("Failed to parse project.json: {}", e))?;

        // Crash recovery from journal if present
        let recovered = JournalRecovery::recover(&bundle_dir, &mut project)?;
        if recovered > 0 {
            tracing::info!(
                "Successfully recovered {} uncommitted operations from journal",
                recovered
            );
        }

        let journal = JournalWriter::open(&bundle_dir)
            .map_err(|e| format!("Failed to open journal: {}", e))?;

        let mut engine = Self::new(project, 200);
        engine.bundle_path = Some(bundle_dir);
        engine.journal = Some(journal);

        Ok(engine)
    }

    pub fn apply_op(
        &mut self,
        op: Op,
        source: OpSource,
        label: impl Into<String>,
    ) -> Result<(), OpError> {
        let label_str = label.into();
        let active_timeline = self
            .project
            .timelines
            .iter_mut()
            .find(|t| t.id == self.project.active_timeline_id)
            .ok_or_else(|| OpError::TimelineNotFound {
                timeline_id: self.project.active_timeline_id,
            })?;

        // 1. Apply mutation in-memory
        let inverse = op.apply(active_timeline, &mut self.project.media_pool)?;

        // 2. Append to crash journal
        if let Some(ref mut journal) = self.journal {
            let _ = journal.append_op(&op);
        }

        // 3. Record in undo stack
        self.history
            .push(op.clone(), inverse, source.clone(), label_str.clone());

        // 4. Publish reactive event
        self.event_bus.publish(EngineEvent::OpApplied {
            op,
            source,
            label: label_str,
        });
        self.event_bus.publish(EngineEvent::TimelineChanged {
            timeline_id: self.project.active_timeline_id,
        });

        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), HistoryError> {
        let active_timeline = self
            .project
            .timelines
            .iter_mut()
            .find(|t| t.id == self.project.active_timeline_id)
            .ok_or(HistoryError::UndoStackEmpty)?;

        let entry = self
            .history
            .undo(active_timeline, &mut self.project.media_pool)?;
        let inverse = entry.inverse.clone();

        self.event_bus.publish(EngineEvent::OpUndone { inverse });
        self.event_bus.publish(EngineEvent::TimelineChanged {
            timeline_id: self.project.active_timeline_id,
        });

        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), HistoryError> {
        let active_timeline = self
            .project
            .timelines
            .iter_mut()
            .find(|t| t.id == self.project.active_timeline_id)
            .ok_or(HistoryError::RedoStackEmpty)?;

        let entry = self
            .history
            .redo(active_timeline, &mut self.project.media_pool)?;
        let op = entry.op.clone();
        let source = entry.source.clone();
        let label = entry.label.clone();

        self.event_bus.publish(EngineEvent::OpApplied { op, source, label });
        self.event_bus.publish(EngineEvent::TimelineChanged {
            timeline_id: self.project.active_timeline_id,
        });

        Ok(())
    }

    pub fn save_to_bundle(&mut self, bundle_dir: &Path) -> Result<(), String> {
        let json_path = bundle_dir.join("project.json");
        let temp_path = bundle_dir.join("project.json.tmp");

        let serialized = serde_json::to_string_pretty(&self.project)
            .map_err(|e| format!("Failed to serialize project: {}", e))?;

        std::fs::write(&temp_path, serialized)
            .map_err(|e| format!("Failed to write temp project.json: {}", e))?;

        std::fs::rename(&temp_path, &json_path)
            .map_err(|e| format!("Failed to atomic rename project.json: {}", e))?;

        if let Some(ref mut journal) = self.journal {
            let _ = journal.truncate();
        }

        Ok(())
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.project
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.event_bus.subscribe()
    }
}
