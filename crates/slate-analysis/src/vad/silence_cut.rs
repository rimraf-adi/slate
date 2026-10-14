use slate_core::id::{ClipId, TrackId};
use slate_core::model::timeline::Timeline;
use slate_core::op::types::Op;
use slate_core::time::TimeRange;

use crate::error::AnalysisError;

/// Builds an atomic batch of ripple delete operations to remove dead air silences from a clip.
pub fn build_silence_removal_ops(
    silences: &[TimeRange],
    timeline: &Timeline,
    track_id: TrackId,
    clip_id: ClipId,
) -> Result<Op, AnalysisError> {
    let (_track, clip) = timeline
        .find_clip(clip_id)
        .ok_or_else(|| AnalysisError::TimelineMapping(format!("Clip {:?} not found", clip_id)))?;

    let clip_source = clip.source_range;
    let mut timeline_deletes = Vec::new();

    for sil in silences {
        // Intersect silence range with clip source range
        let start = sil.start.max(clip_source.start);
        let end = sil.end_time().min(clip_source.end_time());

        if end > start {
            let offset_start = start - clip_source.start;
            let dur = end - start;
            let tl_start = clip.timeline_range.start + offset_start;
            timeline_deletes.push(TimeRange::new(tl_start, dur));
        }
    }

    // Sort descending by start time so earlier timestamps don't shift during ripple
    timeline_deletes.sort_by_key(|r| std::cmp::Reverse(r.start));

    let mut ops = Vec::new();
    for range in timeline_deletes {
        ops.push(Op::RippleDelete {
            ranges: vec![range],
            track_ids: Some(vec![track_id]),
        });
    }

    Ok(Op::Batch {
        ops,
        label: "Remove silence".to_string(),
    })
}
