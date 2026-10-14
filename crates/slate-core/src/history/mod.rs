pub mod entry;

pub use entry::{HistoryError, HistoryEntry, OpSource};

use crate::model::{MediaReference, Timeline, TrackItem};
use crate::op::{Op, OpApplicable, OpInverse};
use crate::time::RationalTime;

pub struct History {
    entries: Vec<HistoryEntry>,
    cursor: usize,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            capacity,
        }
    }

    /// Pushes a newly applied operation into the history stack, truncating any redo branches.
    pub fn push(
        &mut self,
        op: Op,
        inverse: OpInverse,
        source: OpSource,
        label: impl Into<String>,
    ) {
        // Truncate undone branch if cursor is behind
        self.entries.truncate(self.cursor);

        let entry = HistoryEntry::new(op, inverse, source, label);
        self.entries.push(entry);

        if self.entries.len() > self.capacity {
            self.entries.remove(0);
        } else {
            self.cursor += 1;
        }
    }

    /// Undoes the last operation.
    pub fn undo(
        &mut self,
        timeline: &mut Timeline,
        media_pool: &mut Vec<MediaReference>,
    ) -> Result<&HistoryEntry, HistoryError> {
        if self.cursor == 0 {
            return Err(HistoryError::UndoStackEmpty);
        }

        self.cursor -= 1;
        let entry = &self.entries[self.cursor];
        apply_inverse(&entry.inverse, timeline, media_pool);

        Ok(entry)
    }

    /// Redoes the previously undone operation.
    pub fn redo(
        &mut self,
        timeline: &mut Timeline,
        media_pool: &mut Vec<MediaReference>,
    ) -> Result<&HistoryEntry, HistoryError> {
        if self.cursor >= self.entries.len() {
            return Err(HistoryError::RedoStackEmpty);
        }

        let entry = &self.entries[self.cursor];
        let _ = entry.op.apply(timeline, media_pool);
        self.cursor += 1;

        Ok(entry)
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor < self.entries.len()
    }
}

pub fn apply_inverse(
    inverse: &OpInverse,
    timeline: &mut Timeline,
    media_pool: &mut Vec<MediaReference>,
) {
    match inverse {
        OpInverse::UnimportMedia { media_id } => {
            media_pool.retain(|m| m.id != *media_id);
        }
        OpInverse::ReimportMedia { reference } => {
            media_pool.push(reference.clone());
        }
        OpInverse::RemoveAddedTrack { track_id } => {
            timeline.tracks.retain(|t| t.id != *track_id);
        }
        OpInverse::RestoreRemovedTrack { index, track } => {
            let pos = (*index).min(timeline.tracks.len());
            timeline.tracks.insert(pos, track.clone());
        }
        OpInverse::RevertReorderTrack {
            track_id,
            original_index,
        } => {
            if let Some(pos) = timeline.tracks.iter().position(|t| t.id == *track_id) {
                let track = timeline.tracks.remove(pos);
                let target = (*original_index).min(timeline.tracks.len());
                timeline.tracks.insert(target, track);
            }
        }
        OpInverse::RevertTrackState {
            track_id,
            muted,
            solo,
            locked,
        } => {
            if let Some(track) = timeline.find_track_mut(*track_id) {
                track.muted = *muted;
                track.solo = *solo;
                track.locked = *locked;
            }
        }
        OpInverse::RemoveAddedClip { clip_id } => {
            for track in &mut timeline.tracks {
                track.items.retain(|item| match item {
                    TrackItem::Clip(c) => c.id != *clip_id,
                    _ => true,
                });
            }
        }
        OpInverse::RestoreRemovedClip {
            track_id,
            index,
            clip,
        } => {
            if let Some(track) = timeline.find_track_mut(*track_id) {
                let pos = (*index).min(track.items.len());
                track.items.insert(pos, TrackItem::Clip(clip.clone()));
            }
        }
        OpInverse::RevertMoveClip {
            clip_id,
            original_track,
            original_time,
        } => {
            let mut extracted = None;
            for track in &mut timeline.tracks {
                if let Some(pos) = track.items.iter().position(|i| match i {
                    TrackItem::Clip(c) => c.id == *clip_id,
                    _ => false,
                }) {
                    if let TrackItem::Clip(c) = track.items.remove(pos) {
                        extracted = Some(c);
                        break;
                    }
                }
            }
            if let Some(mut clip) = extracted {
                clip.timeline_range.start = *original_time;
                if let Some(track) = timeline.find_track_mut(*original_track) {
                    track.items.push(TrackItem::Clip(clip));
                    track.items.sort_by_key(|i| i.timeline_range().start);
                }
            }
        }
        OpInverse::RevertTrimClip {
            clip_id,
            original_source,
            original_timeline,
        } => {
            for track in &mut timeline.tracks {
                if let Some(clip) = track.find_clip_mut(*clip_id) {
                    clip.source_range = *original_source;
                    clip.timeline_range = *original_timeline;
                    break;
                }
            }
        }
        OpInverse::UnsplitClip {
            kept_clip_id,
            removed_clip_id,
            original_source_duration,
        } => {
            for track in &mut timeline.tracks {
                if let Some(pos) = track.items.iter().position(|i| match i {
                    TrackItem::Clip(c) => c.id == *kept_clip_id,
                    _ => false,
                }) {
                    track.items.retain(|i| match i {
                        TrackItem::Clip(c) => c.id != *removed_clip_id,
                        _ => true,
                    });
                    if let TrackItem::Clip(c) = &mut track.items[pos] {
                        c.source_range.duration = *original_source_duration;
                        c.timeline_range.duration = RationalTime::from_seconds(
                            original_source_duration.to_seconds() / c.speed.abs(),
                            c.timeline_range.duration.rate,
                        );
                    }
                    break;
                }
            }
        }
        OpInverse::RevertClipSpeed {
            clip_id,
            original_speed,
        } => {
            for track in &mut timeline.tracks {
                if let Some(clip) = track.find_clip_mut(*clip_id) {
                    clip.speed = *original_speed;
                    let duration_secs =
                        clip.source_range.duration.to_seconds() / original_speed.abs();
                    clip.timeline_range.duration = RationalTime::from_seconds(
                        duration_secs,
                        clip.timeline_range.duration.rate,
                    );
                    break;
                }
            }
        }
        OpInverse::RestoreRipple { restored_tracks } => {
            for (track_id, items) in restored_tracks {
                if let Some(t) = timeline.tracks.iter_mut().find(|t| t.id == *track_id) {
                    t.items = items.clone();
                }
            }
        }
        OpInverse::RevertTransform {
            clip_id,
            original_transform,
        } => {
            for track in &mut timeline.tracks {
                if let Some(clip) = track.find_clip_mut(*clip_id) {
                    clip.transform = original_transform.clone();
                    break;
                }
            }
        }
        OpInverse::RemoveAppliedEffect { clip_id, effect_id } => {
            for track in &mut timeline.tracks {
                if let Some(clip) = track.find_clip_mut(*clip_id) {
                    clip.effects.retain(|e| e.id != *effect_id);
                    break;
                }
            }
        }
        OpInverse::RevertAudioParams {
            clip_id,
            original_params,
        } => {
            for track in &mut timeline.tracks {
                if let Some(clip) = track.find_clip_mut(*clip_id) {
                    clip.audio_params = original_params.clone();
                    break;
                }
            }
        }
        OpInverse::RemoveAddedMarker { marker_id } => {
            timeline.markers.retain(|m| m.id != *marker_id);
        }
        OpInverse::RestoreRemovedMarker { marker } => {
            timeline.markers.push(marker.clone());
        }
        OpInverse::RemoveAddedCaption { caption_id } => {
            timeline.captions.retain(|c| c.id != *caption_id);
        }
        OpInverse::RestoreRemovedCaption { caption } => {
            timeline.captions.push(caption.clone());
        }
        OpInverse::BatchInverse { inverses, .. } => {
            for inv in inverses {
                apply_inverse(inv, timeline, media_pool);
            }
        }
        _ => {}
    }
}
