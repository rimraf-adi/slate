//! Central state management for Slate video editor.

use std::collections::HashSet;
use std::sync::Arc;
use parking_lot::Mutex;
use uuid::Uuid;

use slate_core::engine::Engine;
use slate_core::events::{EngineEvent, PlaybackState};
use slate_core::id::{ClipId, TrackId};
use slate_core::model::TrackItem;
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::{RationalTime, TimeRange, Timecode};
use slate_audio::PlaybackClock;
use slate_analysis::types::Transcript;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    Select,
    Blade,
    TrimRipple,
    Slip,
    Slide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentPresence {
    pub session_id: String,
    pub active_range: Option<TimeRange>,
    pub last_action: String,
    pub timestamp_secs: f64,
}

#[derive(Debug, Clone)]
pub struct AgentOpEntry {
    pub op_id: Uuid,
    pub session_id: String,
    pub op_name: String,
    pub summary: String,
    pub timestamp_secs: f64,
    pub affected_clip_ids: Vec<ClipId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportPreset {
    YouTube1080p,
    YouTube4K,
    ShortsTikTok9x16,
    Square1x1,
    ProRes422Hq,
    AudioOnlyAac,
}

impl ExportPreset {
    pub fn name(&self) -> &'static str {
        match self {
            Self::YouTube1080p => "YouTube HD (1080p 16:9)",
            Self::YouTube4K => "YouTube Ultra HD (4K 16:9)",
            Self::ShortsTikTok9x16 => "Shorts / Reels / TikTok (9:16)",
            Self::Square1x1 => "Instagram Square (1:1)",
            Self::ProRes422Hq => "Apple ProRes 422 HQ (Master)",
            Self::AudioOnlyAac => "Audio Only (AAC 320kbps)",
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::YouTube1080p => (1920, 1080),
            Self::YouTube4K => (3840, 2160),
            Self::ShortsTikTok9x16 => (1080, 1920),
            Self::Square1x1 => (1080, 1080),
            Self::ProRes422Hq => (1920, 1080),
            Self::AudioOnlyAac => (0, 0),
        }
    }
}

pub struct ExportDialogState {
    pub is_open: bool,
    pub preset: ExportPreset,
    pub output_path: String,
    pub is_exporting: bool,
    pub progress: f32, // 0.0 to 1.0
    pub status_text: String,
}

impl Default for ExportDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            preset: ExportPreset::YouTube1080p,
            output_path: "export.mp4".to_string(),
            is_exporting: false,
            progress: 0.0,
            status_text: "Ready".to_string(),
        }
    }
}

/// Central reactive state holding engine, playback clock, selection, and tools.
pub struct EditorState {
    pub engine: Arc<Mutex<Engine>>,
    pub playback_clock: Arc<PlaybackClock>,
    pub playhead: RationalTime,
    pub is_playing: bool,
    pub shuttle_speed: i8, // -8, -4, -2, -1, 0, 1, 2, 4, 8

    // Timeline view navigation
    pub zoom_px_per_sec: f32,
    pub scroll_x: f32,
    pub snap_enabled: bool,
    pub snap_tolerance_px: f32,

    // Interaction & Selection
    pub active_tool: ToolMode,
    pub selected_clip_ids: HashSet<ClipId>,
    pub active_track_id: Option<TrackId>,
    pub is_dragging_clip: bool,
    pub drag_start_pos: Option<(f32, f32)>,

    // Agent live synchronization
    pub agent_presence: Option<AgentPresence>,
    pub agent_ops_history: Vec<AgentOpEntry>,

    // Analysis & Transcript
    pub transcript: Option<Transcript>,
    pub selected_transcript_words: HashSet<usize>,

    // Meters & Diagnostics
    pub audio_peak_l: f32,
    pub audio_peak_r: f32,
    pub dropped_frames: u64,
    pub last_frame_time: std::time::Instant,

    // Studio status & feedback notification
    pub status_text: String,
    pub status_is_error: bool,
    pub status_time: Option<std::time::Instant>,

    // Media Pool Folders & Bins
    pub media_active_bin: String,
    pub media_custom_bins: Vec<String>,
    pub media_search_query: String,
    pub media_list_view: bool,
    pub request_file_browser_import: bool,

    // Export
    pub export_state: ExportDialogState,
}

impl EditorState {
    pub fn new(engine: Engine) -> Self {
        let timebase = engine.project().active_timeline()
            .map(|t| t.settings.timebase)
            .unwrap_or(30);
        let clock = Arc::new(PlaybackClock::new(timebase));
        let engine_arc = Arc::new(Mutex::new(engine));

        Self {
            engine: engine_arc,
            playback_clock: clock,
            playhead: RationalTime::from_frames(0, timebase),
            is_playing: false,
            shuttle_speed: 0,
            zoom_px_per_sec: 60.0,
            scroll_x: 0.0,
            snap_enabled: true,
            snap_tolerance_px: 10.0,
            active_tool: ToolMode::Select,
            selected_clip_ids: HashSet::new(),
            active_track_id: None,
            is_dragging_clip: false,
            drag_start_pos: None,
            agent_presence: None,
            agent_ops_history: Vec::new(),
            transcript: None,
            selected_transcript_words: HashSet::new(),
            audio_peak_l: 0.0,
            audio_peak_r: 0.0,
            dropped_frames: 0,
            last_frame_time: std::time::Instant::now(),
            status_text: "Ready".to_string(),
            status_is_error: false,
            status_time: Some(std::time::Instant::now()),
            media_active_bin: "All".to_string(),
            media_custom_bins: vec!["B-Roll".to_string(), "Graphics".to_string(), "Interviews".to_string()],
            media_search_query: String::new(),
            media_list_view: false,
            request_file_browser_import: false,
            export_state: ExportDialogState::default(),
        }
    }

    pub fn set_project(&mut self, mut project: slate_core::model::Project) {
        if let Some(tl) = project.active_timeline_mut() {
            if tl.tracks.is_empty() {
                tl.tracks.push(slate_core::model::Track::new("V1", slate_core::model::TrackType::Video));
                tl.tracks.push(slate_core::model::Track::new("A1", slate_core::model::TrackType::Audio));
            }
        }
        let timebase = project.active_timeline()
            .map(|t| t.settings.timebase)
            .unwrap_or(30);
        let engine = Engine::new(project, 200);
        self.engine = Arc::new(Mutex::new(engine));
        self.playback_clock = Arc::new(PlaybackClock::new(timebase));
        self.playhead = RationalTime::from_frames(0, timebase);
        self.is_playing = false;
        self.selected_clip_ids.clear();
        self.active_track_id = None;
        self.transcript = None;
        self.selected_transcript_words.clear();
        self.set_status("✓ Project active", false);
    }

    pub fn set_status(&mut self, text: impl Into<String>, is_error: bool) {
        self.status_text = text.into();
        self.status_is_error = is_error;
        self.status_time = Some(std::time::Instant::now());
    }

    /// Process incoming engine events to keep UI in live synchronization.
    pub fn handle_event(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::PlayheadMoved(time) => {
                self.playhead = time;
                self.playback_clock.seek(time);
            }
            EngineEvent::PlaybackStateChanged(state) => {
                let playing = state == PlaybackState::Playing;
                self.is_playing = playing;
                if playing {
                    self.playback_clock.play();
                } else {
                    self.playback_clock.pause();
                }
            }
            _ => {}
        }
    }

    /// Apply an operation with source attribution and record in agent history if applicable.
    pub fn apply_op(&mut self, op: Op, source: OpSource) -> Result<(), String> {
        let op_name = format!("{:?}", op);
        let op_clone = op.clone();
        let summary = format_op_summary(&op_clone);

        {
            let mut engine = self.engine.lock();
            engine.apply_op(op, source.clone(), summary.clone())
                .map_err(|e| e.to_string())?;
        }

        self.set_status(format!("✓ {}", summary), false);

        // If from agent, record entry for the live Agent panel
        if let OpSource::Agent { session_id } = source {
            let entry = AgentOpEntry {
                op_id: Uuid::new_v4(),
                session_id,
                op_name,
                summary,
                timestamp_secs: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
                affected_clip_ids: extract_affected_clips(&op_clone),
            };
            self.agent_ops_history.push(entry);
        }

        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool, String> {
        let success = {
            let mut engine = self.engine.lock();
            if engine.history().can_undo() {
                engine.undo().map_err(|e| e.to_string())?;
                true
            } else {
                false
            }
        };
        if success {
            self.set_status("↶ Undid last action", false);
            Ok(true)
        } else {
            self.set_status("Nothing to undo", false);
            Ok(false)
        }
    }

    pub fn redo(&mut self) -> Result<bool, String> {
        let success = {
            let mut engine = self.engine.lock();
            if engine.history().can_redo() {
                engine.redo().map_err(|e| e.to_string())?;
                true
            } else {
                false
            }
        };
        if success {
            self.set_status("↷ Redid last action", false);
            Ok(true)
        } else {
            self.set_status("Nothing to redo", false);
            Ok(false)
        }
    }

    pub fn toggle_play(&mut self) {
        if self.is_playing {
            self.is_playing = false;
            self.shuttle_speed = 0;
            self.playback_clock.pause();
        } else {
            self.is_playing = true;
            self.shuttle_speed = 1;
            self.playback_clock.seek(self.playhead);
            self.playback_clock.play();
        }
    }

    pub fn shuttle_forward(&mut self) {
        self.shuttle_speed = match self.shuttle_speed {
            s if s <= 0 => 1,
            1 => 2,
            2 => 4,
            4 => 8,
            _ => 8,
        };
        self.is_playing = true;
        self.playback_clock.seek(self.playhead);
        self.playback_clock.play();
    }

    pub fn shuttle_backward(&mut self) {
        self.shuttle_speed = match self.shuttle_speed {
            s if s >= 0 => -1,
            -1 => -2,
            -2 => -4,
            -4 => -8,
            _ => -8,
        };
        self.is_playing = true;
        self.playback_clock.seek(self.playhead);
        self.playback_clock.play();
    }

    pub fn shuttle_pause(&mut self) {
        self.is_playing = false;
        self.shuttle_speed = 0;
        self.playback_clock.pause();
    }

    pub fn step_frame(&mut self, delta_frames: i64) {
        self.shuttle_pause();
        let timebase = {
            let engine = self.engine.lock();
            engine.project().active_timeline().map(|t| t.settings.timebase).unwrap_or(30)
        };
        let current_frames = self.playhead.to_frames();
        let new_frames = (current_frames + delta_frames).max(0);
        let new_time = RationalTime::from_frames(new_frames, timebase);
        self.playhead = new_time;
        self.playback_clock.seek(new_time);
    }

    pub fn seek(&mut self, time: RationalTime) {
        self.playhead = time;
        self.playback_clock.seek(time);
    }

    /// Split a specific clip at a given timestamp.
    pub fn split_clip_at(&mut self, clip_id: ClipId, at: RationalTime) -> Result<(), String> {
        let (track_locked, clip_range, clip_name) = {
            let engine = self.engine.lock();
            let tl = engine.project().active_timeline().ok_or_else(|| "No active timeline".to_string())?;
            let (track, clip) = tl.find_clip(clip_id).ok_or_else(|| "Clip not found".to_string())?;
            (track.locked, clip.timeline_range, clip.name.clone())
        };

        if track_locked {
            let err = "Cannot split clip on a locked track".to_string();
            self.set_status(&err, true);
            return Err(err);
        }

        let at_rescaled = at.rescale(clip_range.start.rate);
        if at_rescaled <= clip_range.start || at_rescaled >= clip_range.end_time() {
            let err = format!(
                "Cut point ({}) must be strictly inside clip range",
                Timecode::format(at, false),
            );
            self.set_status(&err, true);
            return Err(err);
        }

        let new_clip_id = ClipId::new();
        let res = self.apply_op(
            Op::SplitClip {
                clip_id,
                at: at_rescaled,
                new_clip_id,
            },
            OpSource::Human,
        );

        match res {
            Ok(_) => {
                self.selected_clip_ids.clear();
                self.selected_clip_ids.insert(new_clip_id);
                self.set_status(format!("✓ Split clip '{}' at {}", clip_name, Timecode::format(at, false)), false);
                Ok(())
            }
            Err(e) => {
                self.set_status(format!("⚠ Split failed: {}", e), true);
                Err(e)
            }
        }
    }

    /// Split the selected clip(s) or any clip under playhead at the current playhead position.
    pub fn split_at_playhead(&mut self) -> Result<(), String> {
        let candidates = {
            let engine = self.engine.lock();
            let mut list = Vec::new();
            if let Some(tl) = engine.project().active_timeline() {
                for track in &tl.tracks {
                    if track.locked {
                        continue;
                    }
                    for item in &track.items {
                        if let TrackItem::Clip(clip) = item {
                            let at_rescaled = self.playhead.rescale(clip.timeline_range.start.rate);
                            if clip.timeline_range.contains_time(at_rescaled) && at_rescaled > clip.timeline_range.start {
                                list.push((clip.id, track.id, clip.name.clone()));
                            }
                        }
                    }
                }
            }
            list
        };

        if candidates.is_empty() {
            let err = format!("No clip under playhead to split at {}", Timecode::format(self.playhead, false));
            self.set_status(&err, true);
            return Err(err);
        }

        // If user has specific clips selected, filter to those; otherwise split active track or first candidate
        let target = if !self.selected_clip_ids.is_empty() {
            candidates.iter().find(|(cid, _, _)| self.selected_clip_ids.contains(cid)).cloned()
        } else if let Some(active_tid) = self.active_track_id {
            candidates.iter().find(|(_, tid, _)| *tid == active_tid).cloned().or_else(|| candidates.first().cloned())
        } else {
            candidates.first().cloned()
        };

        let (clip_id, _, name) = target.unwrap_or_else(|| candidates[0].clone());
        let new_clip_id = ClipId::new();
        let res = self.apply_op(
            Op::SplitClip {
                clip_id,
                at: self.playhead,
                new_clip_id,
            },
            OpSource::Human,
        );
        match res {
            Ok(_) => {
                self.selected_clip_ids.clear();
                self.selected_clip_ids.insert(new_clip_id);
                self.set_status(format!("✓ Split clip '{}' at {}", name, Timecode::format(self.playhead, false)), false);
                Ok(())
            }
            Err(e) => {
                self.set_status(format!("⚠ Split failed: {}", e), true);
                Err(e)
            }
        }
    }

    /// Split selected clip at playhead, or if playhead is outside selected clip, split at clip midpoint.
    pub fn split_selected_or_at_playhead(&mut self) -> Result<(), String> {
        let first_selected = self.selected_clip_ids.iter().next().copied();
        let clip_range = if let Some(clip_id) = first_selected {
            let engine = self.engine.lock();
            engine.project().active_timeline().and_then(|tl| {
                tl.find_clip(clip_id).map(|(_, clip)| clip.timeline_range)
            })
        } else {
            None
        };

        if let (Some(clip_id), Some(range)) = (first_selected, clip_range) {
            let at_rescaled = self.playhead.rescale(range.start.rate);
            if range.contains_time(at_rescaled) && at_rescaled > range.start {
                self.split_clip_at(clip_id, self.playhead)
            } else {
                // Split at clip midpoint and move playhead there
                let mid_time = range.start + range.duration / 2;
                if mid_time > range.start && mid_time < range.end_time() {
                    self.seek(mid_time);
                    self.split_clip_at(clip_id, mid_time)
                } else {
                    self.split_at_playhead()
                }
            }
        } else {
            self.split_at_playhead()
        }
    }

    /// Ripple delete all selected clips.
    pub fn ripple_delete_selected(&mut self) -> Result<(), String> {
        if self.selected_clip_ids.is_empty() {
            return Ok(());
        }

        let ranges_to_delete: Vec<TimeRange> = {
            let engine = self.engine.lock();
            let mut ranges = Vec::new();
            if let Some(tl) = engine.project().active_timeline() {
                for track in &tl.tracks {
                    for item in &track.items {
                        if let TrackItem::Clip(clip) = item {
                            if self.selected_clip_ids.contains(&clip.id) {
                                ranges.push(clip.timeline_range);
                            }
                        }
                    }
                }
            }
            ranges
        };

        if ranges_to_delete.is_empty() {
            return Ok(());
        }

        self.apply_op(
            Op::RippleDelete {
                ranges: ranges_to_delete,
                track_ids: None,
            },
            OpSource::Human,
        )?;

        self.selected_clip_ids.clear();
        Ok(())
    }

    /// Update playback clock progression on UI update frame tick.
    pub fn tick_playback(&mut self) {
        if self.is_playing {
            let timebase = {
                let engine = self.engine.lock();
                engine.project().active_timeline().map(|t| t.settings.timebase).unwrap_or(30)
            };

            let now = std::time::Instant::now();
            let dt = (now - self.last_frame_time).as_secs_f64();
            self.last_frame_time = now;

            // Multiply dt by shuttle speed
            let effective_dt = dt * (self.shuttle_speed as f64);
            let delta_ticks = (effective_dt * (timebase as f64)).round() as i64;
            let current_ticks = self.playhead.ticks;
            let new_ticks = (current_ticks + delta_ticks).max(0);
            self.playhead = RationalTime::new(new_ticks, timebase);
            self.playback_clock.seek(self.playhead);
        } else {
            self.last_frame_time = std::time::Instant::now();
        }
    }
}

fn format_op_summary(op: &Op) -> String {
    match op {
        Op::AddClip { clip, .. } => format!("Added clip '{}'", clip.name),
        Op::RemoveClip { .. } => "Removed clip".to_string(),
        Op::SplitClip { .. } => "Split clip at cut point".to_string(),
        Op::TrimClip { .. } => "Trimmed clip in/out boundaries".to_string(),
        Op::MoveClip { .. } => "Moved clip on timeline".to_string(),
        Op::RippleDelete { ranges, .. } => format!("Ripple deleted {} ranges", ranges.len()),
        Op::Batch { ops, label } => format!("Batch: {} ({} ops)", label, ops.len()),
        _ => "Timeline edit operation".to_string(),
    }
}

fn extract_affected_clips(op: &Op) -> Vec<ClipId> {
    match op {
        Op::RemoveClip { clip_id, .. } => vec![*clip_id],
        Op::SplitClip { clip_id, new_clip_id, .. } => vec![*clip_id, *new_clip_id],
        Op::TrimClip { clip_id, .. } => vec![*clip_id],
        Op::MoveClip { clip_id, .. } => vec![*clip_id],
        Op::Batch { ops, .. } => ops.iter().flat_map(extract_affected_clips).collect(),
        _ => Vec::new(),
    }
}
