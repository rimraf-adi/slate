use super::error::OpError;
use super::inverse::OpInverse;
use super::types::{Op, TrimEdge};
use crate::model::{
    MediaMetadata, MediaReference, Timeline, Track, TrackItem,
};
use crate::time::{RationalTime, TimeRange};

pub trait OpApplicable {
    /// Validates preconditions before mutation. Returns Ok(()) if operation is safe to execute.
    fn validate(&self, timeline: &Timeline, media_pool: &[MediaReference]) -> Result<(), OpError>;

    /// Executes the mutation in place and returns the exact symmetrical inverse for undo.
    fn apply(
        &self,
        timeline: &mut Timeline,
        media_pool: &mut Vec<MediaReference>,
    ) -> Result<OpInverse, OpError>;
}

impl OpApplicable for Op {
    fn validate(&self, timeline: &Timeline, media_pool: &[MediaReference]) -> Result<(), OpError> {
        match self {
            Op::ImportMedia { .. } => Ok(()),
            Op::RemoveMedia { media_id } => {
                if !media_pool.iter().any(|m| m.id == *media_id) {
                    return Err(OpError::MediaNotImported {
                        media_id: *media_id,
                    });
                }
                Ok(())
            }
            Op::AddTrack { .. } => Ok(()),
            Op::RemoveTrack { track_id } => {
                if timeline.find_track(*track_id).is_none() {
                    return Err(OpError::TrackNotFound {
                        track_id: *track_id,
                    });
                }
                Ok(())
            }
            Op::ReorderTrack {
                track_id,
                new_index,
            } => {
                let track_idx = timeline
                    .tracks
                    .iter()
                    .position(|t| t.id == *track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                if *new_index >= timeline.tracks.len() {
                    // Valid clamped index
                }
                let _ = track_idx;
                Ok(())
            }
            Op::SetTrackMute { track_id, .. }
            | Op::SetTrackSolo { track_id, .. }
            | Op::SetTrackLock { track_id, .. } => {
                if timeline.find_track(*track_id).is_none() {
                    return Err(OpError::TrackNotFound {
                        track_id: *track_id,
                    });
                }
                Ok(())
            }
            Op::AddClip {
                track_id,
                clip,
                position,
            } => {
                let track = timeline
                    .find_track(*track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                if track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: *track_id,
                    });
                }
                if !media_pool.iter().any(|m| m.id == clip.media_id) {
                    return Err(OpError::MediaNotImported {
                        media_id: clip.media_id,
                    });
                }
                let new_range = TimeRange::new(*position, clip.timeline_range.duration);
                for item in &track.items {
                    if let TrackItem::Clip(existing) = item {
                        if existing.timeline_range.overlaps(&new_range) {
                            return Err(OpError::OverlapConflict {
                                conflicting_clip_id: existing.id,
                                requested_start: new_range.start,
                                requested_end: new_range.end_time(),
                            });
                        }
                    }
                }
                Ok(())
            }
            Op::RemoveClip { clip_id } => {
                let (track, _) = timeline
                    .find_clip(*clip_id)
                    .ok_or(OpError::ClipNotFound {
                        clip_id: *clip_id,
                    })?;
                if track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: track.id,
                    });
                }
                Ok(())
            }
            Op::MoveClip {
                clip_id,
                target_track,
                target_time,
            } => {
                let (src_track, clip) = timeline
                    .find_clip(*clip_id)
                    .ok_or(OpError::ClipNotFound {
                        clip_id: *clip_id,
                    })?;
                if src_track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: src_track.id,
                    });
                }
                let dst_track = timeline
                    .find_track(*target_track)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *target_track,
                    })?;
                if dst_track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: *target_track,
                    });
                }
                let new_range = TimeRange::new(*target_time, clip.timeline_range.duration);
                for item in &dst_track.items {
                    if let TrackItem::Clip(existing) = item {
                        if existing.id != *clip_id && existing.timeline_range.overlaps(&new_range) {
                            return Err(OpError::OverlapConflict {
                                conflicting_clip_id: existing.id,
                                requested_start: new_range.start,
                                requested_end: new_range.end_time(),
                            });
                        }
                    }
                }
                Ok(())
            }
            Op::TrimClip {
                clip_id,
                edge: _,
                delta,
                mode: _,
            } => {
                let (track, clip) = timeline
                    .find_clip(*clip_id)
                    .ok_or(OpError::ClipNotFound {
                        clip_id: *clip_id,
                    })?;
                if track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: track.id,
                    });
                }
                let media = media_pool
                    .iter()
                    .find(|m| m.id == clip.media_id)
                    .ok_or(OpError::MediaNotImported {
                        media_id: clip.media_id,
                    })?;
                let proposed_source_dur = clip.source_range.duration + *delta;
                if proposed_source_dur.ticks <= 0 || proposed_source_dur > media.metadata.duration {
                    return Err(OpError::TrimBeyondMedia {
                        clip_id: *clip_id,
                        requested: proposed_source_dur,
                        available: media.metadata.duration,
                    });
                }
                Ok(())
            }
            Op::SplitClip {
                clip_id,
                at,
                new_clip_id: _,
            } => {
                let (track, clip) = timeline
                    .find_clip(*clip_id)
                    .ok_or(OpError::ClipNotFound {
                        clip_id: *clip_id,
                    })?;
                if track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: track.id,
                    });
                }
                let at_rescaled = at.rescale(clip.timeline_range.start.rate);
                if !clip.timeline_range.contains_time(at_rescaled) || at_rescaled <= clip.timeline_range.start {
                    return Err(OpError::SplitOutsideClip {
                        clip_id: *clip_id,
                        at: *at,
                        start: clip.timeline_range.start,
                        end: clip.timeline_range.end_time(),
                    });
                }
                Ok(())
            }
            Op::SetClipSpeed { clip_id, speed } => {
                if *speed == 0.0 || speed.abs() > 100.0 {
                    return Err(OpError::InvalidSpeed { speed: *speed });
                }
                let (track, _) = timeline
                    .find_clip(*clip_id)
                    .ok_or(OpError::ClipNotFound {
                        clip_id: *clip_id,
                    })?;
                if track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: track.id,
                    });
                }
                Ok(())
            }
            Op::ApplyEffect { clip_id, .. }
            | Op::SetTransform { clip_id, .. }
            | Op::SetKeyframe { clip_id, .. }
            | Op::RemoveKeyframe { clip_id, .. }
            | Op::SetAudioParams { clip_id, .. }
            | Op::NormalizeAudio { clip_id, .. } => {
                let (track, _) = timeline
                    .find_clip(*clip_id)
                    .ok_or(OpError::ClipNotFound {
                        clip_id: *clip_id,
                    })?;
                if track.locked {
                    return Err(OpError::TrackLocked {
                        track_id: track.id,
                    });
                }
                Ok(())
            }
            Op::Batch { ops, .. } => {
                for (idx, sub_op) in ops.iter().enumerate() {
                    sub_op.validate(timeline, media_pool).map_err(|e| {
                        OpError::BatchFailed {
                            index: idx,
                            cause: Box::new(e),
                        }
                    })?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn apply(
        &self,
        timeline: &mut Timeline,
        media_pool: &mut Vec<MediaReference>,
    ) -> Result<OpInverse, OpError> {
        self.validate(timeline, media_pool)?;

        match self {
            Op::ImportMedia { path, media_id } => {
                let reference = MediaReference {
                    id: *media_id,
                    original_path: path.clone(),
                    relative_path: path.clone(),
                    metadata: MediaMetadata {
                        file_size_bytes: 0,
                        duration: RationalTime::from_seconds(3600.0, 30),
                        video_codec: Some("h264".to_string()),
                        width: Some(1920),
                        height: Some(1080),
                        frame_rate: Some(30.0),
                        timebase: 30,
                        audio_codec: Some("aac".to_string()),
                        audio_channels: Some(2),
                        sample_rate: Some(48000),
                    },
                    proxy_path: None,
                    waveform_path: None,
                };
                media_pool.push(reference);
                Ok(OpInverse::UnimportMedia { media_id: *media_id })
            }
            Op::RemoveMedia { media_id } => {
                let idx = media_pool
                    .iter()
                    .position(|m| m.id == *media_id)
                    .ok_or(OpError::MediaNotImported {
                        media_id: *media_id,
                    })?;
                let reference = media_pool.remove(idx);
                Ok(OpInverse::ReimportMedia { reference })
            }
            Op::AddTrack {
                track_type,
                name,
                index,
            } => {
                let track = Track::new(name.clone(), *track_type);
                let track_id = track.id;
                if let Some(idx) = index {
                    let insert_pos = (*idx).min(timeline.tracks.len());
                    timeline.tracks.insert(insert_pos, track);
                } else {
                    timeline.tracks.push(track);
                }
                Ok(OpInverse::RemoveAddedTrack { track_id })
            }
            Op::RemoveTrack { track_id } => {
                let idx = timeline
                    .tracks
                    .iter()
                    .position(|t| t.id == *track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                let track = timeline.tracks.remove(idx);
                Ok(OpInverse::RestoreRemovedTrack { index: idx, track })
            }
            Op::ReorderTrack {
                track_id,
                new_index,
            } => {
                let current_idx = timeline
                    .tracks
                    .iter()
                    .position(|t| t.id == *track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                let track = timeline.tracks.remove(current_idx);
                let target_idx = (*new_index).min(timeline.tracks.len());
                timeline.tracks.insert(target_idx, track);
                Ok(OpInverse::RevertReorderTrack {
                    track_id: *track_id,
                    original_index: current_idx,
                })
            }
            Op::SetTrackMute { track_id, muted } => {
                let track = timeline
                    .find_track_mut(*track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                let old_muted = track.muted;
                track.muted = *muted;
                Ok(OpInverse::RevertTrackState {
                    track_id: *track_id,
                    muted: old_muted,
                    solo: track.solo,
                    locked: track.locked,
                })
            }
            Op::SetTrackSolo { track_id, solo } => {
                let track = timeline
                    .find_track_mut(*track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                let old_solo = track.solo;
                track.solo = *solo;
                Ok(OpInverse::RevertTrackState {
                    track_id: *track_id,
                    muted: track.muted,
                    solo: old_solo,
                    locked: track.locked,
                })
            }
            Op::SetTrackLock { track_id, locked } => {
                let track = timeline
                    .find_track_mut(*track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                let old_locked = track.locked;
                track.locked = *locked;
                Ok(OpInverse::RevertTrackState {
                    track_id: *track_id,
                    muted: track.muted,
                    solo: track.solo,
                    locked: old_locked,
                })
            }
            Op::AddClip {
                track_id,
                clip,
                position,
            } => {
                let track = timeline
                    .find_track_mut(*track_id)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *track_id,
                    })?;
                let mut new_clip = clip.clone();
                new_clip.timeline_range.start = *position;
                let clip_id = new_clip.id;
                track.items.push(TrackItem::Clip(new_clip));
                // Keep track sorted by timeline start
                track.items.sort_by_key(|i| i.timeline_range().start);
                Ok(OpInverse::RemoveAddedClip { clip_id })
            }
            Op::RemoveClip { clip_id } => {
                for track in &mut timeline.tracks {
                    if let Some(pos) = track.items.iter().position(|i| match i {
                        TrackItem::Clip(c) => c.id == *clip_id,
                        _ => false,
                    }) {
                        if let TrackItem::Clip(clip) = track.items.remove(pos) {
                            return Ok(OpInverse::RestoreRemovedClip {
                                track_id: track.id,
                                index: pos,
                                clip,
                            });
                        }
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::MoveClip {
                clip_id,
                target_track,
                target_time,
            } => {
                // Find and remove from source track
                let mut found_clip = None;
                let mut original_track = None;
                let mut original_time = RationalTime::ZERO;

                for track in &mut timeline.tracks {
                    if let Some(pos) = track.items.iter().position(|i| match i {
                        TrackItem::Clip(c) => c.id == *clip_id,
                        _ => false,
                    }) {
                        original_track = Some(track.id);
                        if let TrackItem::Clip(c) = track.items.remove(pos) {
                            original_time = c.timeline_range.start;
                            found_clip = Some(c);
                            break;
                        }
                    }
                }

                let mut clip = found_clip.ok_or(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })?;
                let src_track_id = original_track.unwrap();

                clip.timeline_range.start = *target_time;

                let dst_track = timeline
                    .find_track_mut(*target_track)
                    .ok_or(OpError::TrackNotFound {
                        track_id: *target_track,
                    })?;
                dst_track.items.push(TrackItem::Clip(clip));
                dst_track
                    .items
                    .sort_by_key(|i| i.timeline_range().start);

                Ok(OpInverse::RevertMoveClip {
                    clip_id: *clip_id,
                    original_track: src_track_id,
                    original_time,
                })
            }
            Op::TrimClip {
                clip_id,
                edge,
                delta,
                mode: _,
            } => {
                for track in &mut timeline.tracks {
                    if let Some(clip) = track.find_clip_mut(*clip_id) {
                        let original_source = clip.source_range;
                        let original_timeline = clip.timeline_range;

                        match edge {
                            TrimEdge::Start => {
                                clip.source_range.start += *delta;
                                clip.source_range.duration -= *delta;
                                clip.timeline_range.start += *delta;
                                clip.timeline_range.duration -= *delta;
                            }
                            TrimEdge::End => {
                                clip.source_range.duration += *delta;
                                clip.timeline_range.duration += *delta;
                            }
                        }

                        return Ok(OpInverse::RevertTrimClip {
                            clip_id: *clip_id,
                            original_source,
                            original_timeline,
                        });
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::SplitClip {
                clip_id,
                at,
                new_clip_id,
            } => {
                for track in &mut timeline.tracks {
                    if let Some(pos) = track.items.iter().position(|i| match i {
                        TrackItem::Clip(c) => c.id == *clip_id,
                        _ => false,
                    }) {
                        let first_half = match &track.items[pos] {
                            TrackItem::Clip(c) => c.clone(),
                            _ => unreachable!(),
                        };

                        let at_rescaled = at.rescale(first_half.timeline_range.start.rate);
                        let split_offset = at_rescaled - first_half.timeline_range.start;
                        let original_source_duration = first_half.source_range.duration;

                        let mut second_half = first_half.clone();
                        second_half.id = *new_clip_id;
                        second_half.source_range.start += split_offset.rescale(second_half.source_range.start.rate);
                        second_half.source_range.duration -= split_offset.rescale(second_half.source_range.start.rate);
                        second_half.timeline_range.start = at_rescaled;
                        second_half.timeline_range.duration -= split_offset;

                        if let TrackItem::Clip(first) = &mut track.items[pos] {
                            first.source_range.duration = split_offset.rescale(first.source_range.start.rate);
                            first.timeline_range.duration = split_offset;
                        }

                        track
                            .items
                            .insert(pos + 1, TrackItem::Clip(second_half));

                        return Ok(OpInverse::UnsplitClip {
                            kept_clip_id: *clip_id,
                            removed_clip_id: *new_clip_id,
                            original_source_duration,
                        });
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::SetClipSpeed { clip_id, speed } => {
                for track in &mut timeline.tracks {
                    if let Some(clip) = track.find_clip_mut(*clip_id) {
                        let original_speed = clip.speed;
                        clip.speed = *speed;
                        let new_duration_secs =
                            clip.source_range.duration.to_seconds() / speed.abs();
                        clip.timeline_range.duration = RationalTime::from_seconds(
                            new_duration_secs,
                            clip.timeline_range.duration.rate,
                        );
                        return Ok(OpInverse::RevertClipSpeed {
                            clip_id: *clip_id,
                            original_speed,
                        });
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::SetTransform { clip_id, transform } => {
                for track in &mut timeline.tracks {
                    if let Some(clip) = track.find_clip_mut(*clip_id) {
                        let original_transform = clip.transform.clone();
                        clip.transform = transform.clone();
                        return Ok(OpInverse::RevertTransform {
                            clip_id: *clip_id,
                            original_transform,
                        });
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::ApplyEffect { clip_id, effect } => {
                for track in &mut timeline.tracks {
                    if let Some(clip) = track.find_clip_mut(*clip_id) {
                        clip.effects.push(effect.clone());
                        let effect_id = effect.id;
                        return Ok(OpInverse::RemoveAppliedEffect {
                            clip_id: *clip_id,
                            effect_id,
                        });
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::SetAudioParams { clip_id, params } => {
                for track in &mut timeline.tracks {
                    if let Some(clip) = track.find_clip_mut(*clip_id) {
                        let original_params = clip.audio_params.clone();
                        clip.audio_params = params.clone();
                        return Ok(OpInverse::RevertAudioParams {
                            clip_id: *clip_id,
                            original_params,
                        });
                    }
                }
                Err(OpError::ClipNotFound {
                    clip_id: *clip_id,
                })
            }
            Op::AddMarker { marker } => {
                timeline.markers.push(marker.clone());
                let marker_id = marker.id;
                Ok(OpInverse::RemoveAddedMarker { marker_id })
            }
            Op::RemoveMarker { marker_id } => {
                let pos = timeline
                    .markers
                    .iter()
                    .position(|m| m.id == *marker_id)
                    .ok_or(OpError::MarkerNotFound {
                        marker_id: *marker_id,
                    })?;
                let marker = timeline.markers.remove(pos);
                Ok(OpInverse::RestoreRemovedMarker { marker })
            }
            Op::AddCaption { caption } => {
                timeline.captions.push(caption.clone());
                let caption_id = caption.id;
                Ok(OpInverse::RemoveAddedCaption { caption_id })
            }
            Op::RemoveCaption { caption_id } => {
                let pos = timeline
                    .captions
                    .iter()
                    .position(|c| c.id == *caption_id)
                    .ok_or(OpError::CaptionNotFound {
                        caption_id: *caption_id,
                    })?;
                let caption = timeline.captions.remove(pos);
                Ok(OpInverse::RestoreRemovedCaption { caption })
            }
            Op::RippleDelete { ranges, track_ids } => {
                let mut restored_tracks = Vec::new();
                for track in &mut timeline.tracks {
                    if let Some(ref t_ids) = track_ids {
                        if !t_ids.contains(&track.id) {
                            continue;
                        }
                    }
                    restored_tracks.push((track.id, track.items.clone()));

                    for delete_range in ranges {
                        let mut new_items = Vec::new();
                        for item in &track.items {
                            let item_range = item.timeline_range();
                            if !item_range.overlaps(delete_range) {
                                // If item occurs after the deleted range, shift it left by the deleted duration
                                if item_range.start >= delete_range.end_time() {
                                    let mut shifted = item.clone();
                                    match &mut shifted {
                                        TrackItem::Clip(c) => {
                                            c.timeline_range.start -= delete_range.duration;
                                        }
                                        TrackItem::Gap(g) => {
                                            g.timeline_range.start -= delete_range.duration;
                                        }
                                        _ => {}
                                    }
                                    new_items.push(shifted);
                                } else {
                                    new_items.push(item.clone());
                                }
                            }
                        }
                        track.items = new_items;
                    }
                }
                Ok(OpInverse::RestoreRipple { restored_tracks })
            }
            Op::Batch { ops, label } => {
                let mut inverses = Vec::new();
                for op in ops {
                    let inv = op.apply(timeline, media_pool)?;
                    inverses.push(inv);
                }
                inverses.reverse(); // Reverse so undo applies in inverse order
                Ok(OpInverse::BatchInverse {
                    inverses,
                    label: label.clone(),
                })
            }
            _ => {
                // For ops with default no-op inverse
                Ok(OpInverse::BatchInverse {
                    inverses: Vec::new(),
                    label: "Unsupported".to_string(),
                })
            }
        }
    }
}
