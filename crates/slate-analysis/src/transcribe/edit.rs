use slate_core::id::{ClipId, TrackId};
use slate_core::model::timeline::Timeline;
use slate_core::op::types::Op;
use slate_core::time::{RationalTime, TimeRange};

use crate::error::AnalysisError;
use crate::types::{TranscriptWord, WordRange};

#[derive(Debug, Clone)]
pub struct EditByTranscript {
    pub keep_ranges: Vec<WordRange>,
    pub lead_in_padding: RationalTime,
    pub lead_out_padding: RationalTime,
    pub merge_threshold: RationalTime,
}

impl EditByTranscript {
    pub fn new(keep_ranges: Vec<WordRange>) -> Self {
        Self {
            keep_ranges,
            lead_in_padding: RationalTime::from_seconds(0.060, 1000),
            lead_out_padding: RationalTime::from_seconds(0.080, 1000),
            merge_threshold: RationalTime::from_seconds(0.150, 1000),
        }
    }

    /// Converts word index selections into an atomic batch of timeline operations.
    pub fn build_timeline_ops(
        &self,
        words: &[TranscriptWord],
        timeline: &Timeline,
        track_id: TrackId,
        clip_id: ClipId,
    ) -> Result<Op, AnalysisError> {
        let (_track, clip) = timeline
            .find_clip(clip_id)
            .ok_or_else(|| AnalysisError::TimelineMapping(format!("Clip {:?} not found", clip_id)))?;

        if words.is_empty() || self.keep_ranges.is_empty() {
            return Ok(Op::Batch {
                ops: Vec::new(),
                label: "Edit by transcript".to_string(),
            });
        }

        // 1. Convert Word Ranges to Continuous Media Source Time Ranges
        let mut source_keep_ranges: Vec<TimeRange> = Vec::new();
        for range in &self.keep_ranges {
            if range.start_word_idx >= words.len() || range.end_word_idx >= words.len() {
                return Err(AnalysisError::TimelineMapping(
                    "Word index out of bounds".into(),
                ));
            }
            if range.start_word_idx > range.end_word_idx {
                return Err(AnalysisError::TimelineMapping(
                    "Word range start > end".into(),
                ));
            }

            let raw_start = words[range.start_word_idx].start;
            let raw_end = words[range.end_word_idx].end;

            // Apply acoustic padding
            let padded_start = if raw_start > self.lead_in_padding {
                raw_start - self.lead_in_padding
            } else {
                RationalTime::ZERO
            };
            let padded_end = raw_end + self.lead_out_padding;

            let duration = if padded_end > padded_start {
                padded_end - padded_start
            } else {
                RationalTime::ZERO
            };

            source_keep_ranges.push(TimeRange::new(padded_start, duration));
        }

        // 2. Sort and merge overlapping or near-adjacent intervals (< merge_threshold)
        source_keep_ranges.sort_by_key(|r| r.start);
        let mut merged_keep: Vec<TimeRange> = Vec::new();

        for r in source_keep_ranges {
            if let Some(last) = merged_keep.last_mut() {
                if r.start <= last.end_time() + self.merge_threshold {
                    let new_end = last.end_time().max(r.end_time());
                    *last = TimeRange::new(last.start, new_end - last.start);
                    continue;
                }
            }
            merged_keep.push(r);
        }

        // 3. Compute the complement: intervals to DELETE within the clip's source range
        let clip_source_range = clip.source_range;
        let mut delete_source_ranges: Vec<TimeRange> = Vec::new();
        let mut cursor = clip_source_range.start;

        for keep in &merged_keep {
            // Only consider intervals overlapping with the clip's source range
            let clamped_keep_start = keep
                .start
                .max(clip_source_range.start)
                .min(clip_source_range.end_time());
            let clamped_keep_end = keep
                .end_time()
                .max(clip_source_range.start)
                .min(clip_source_range.end_time());

            if clamped_keep_start > cursor {
                delete_source_ranges.push(TimeRange::new(cursor, clamped_keep_start - cursor));
            }
            cursor = cursor.max(clamped_keep_end);
        }

        if cursor < clip_source_range.end_time() {
            delete_source_ranges
                .push(TimeRange::new(cursor, clip_source_range.end_time() - cursor));
        }

        // 4. Map Source Time Ranges to Timeline Time Ranges
        let mut timeline_deletes: Vec<TimeRange> = Vec::new();
        for src_del in delete_source_ranges {
            if src_del.duration == RationalTime::ZERO {
                continue;
            }
            let offset_start = src_del.start - clip_source_range.start;
            let timeline_start = clip.timeline_range.start + offset_start;
            timeline_deletes.push(TimeRange::new(timeline_start, src_del.duration));
        }

        // 5. Construct atomic Op::Batch of RippleDelete operations
        // Deleting from end to start keeps earlier timeline coordinates stable
        timeline_deletes.sort_by_key(|r| std::cmp::Reverse(r.start));
        let mut ops = Vec::new();

        for del in timeline_deletes {
            ops.push(Op::RippleDelete {
                ranges: vec![del],
                track_ids: Some(vec![track_id]),
            });
        }

        Ok(Op::Batch {
            ops,
            label: "Edit by transcript".to_string(),
        })
    }
}
