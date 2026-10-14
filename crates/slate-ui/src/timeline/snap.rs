//! Magnetic timeline snapping calculations.

use slate_core::model::{Timeline, TrackItem};
use slate_core::time::RationalTime;

pub struct SnapResult {
    pub snapped_time: RationalTime,
    pub snap_line_x: Option<f32>,
    pub did_snap: bool,
}

/// Calculate the closest snap target (clip start/end, playhead) within tolerance.
pub fn find_snap_target(
    target_time: RationalTime,
    timeline: &Timeline,
    playhead: RationalTime,
    pixels_per_second: f32,
    scroll_x: f32,
    tolerance_px: f32,
) -> SnapResult {
    let target_secs = target_time.to_seconds();
    let target_x = (target_secs as f32) * pixels_per_second - scroll_x;

    let mut closest_snap_x: Option<f32> = None;
    let mut closest_snap_time: Option<RationalTime> = None;
    let mut min_distance = tolerance_px;

    // 1. Check playhead
    let playhead_secs = playhead.to_seconds();
    let playhead_x = (playhead_secs as f32) * pixels_per_second - scroll_x;
    let dist = (target_x - playhead_x).abs();
    if dist < min_distance {
        min_distance = dist;
        closest_snap_x = Some(playhead_x);
        closest_snap_time = Some(playhead);
    }

    // 2. Check clip start and end points
    for track in &timeline.tracks {
        for item in &track.items {
            if let TrackItem::Clip(clip) = item {
                let start_secs = clip.timeline_range.start.to_seconds();
                let start_x = (start_secs as f32) * pixels_per_second - scroll_x;
                let d_start = (target_x - start_x).abs();
                if d_start < min_distance {
                    min_distance = d_start;
                    closest_snap_x = Some(start_x);
                    closest_snap_time = Some(clip.timeline_range.start);
                }

                let end_time = clip.timeline_range.end_time();
                let end_secs = end_time.to_seconds();
                let end_x = (end_secs as f32) * pixels_per_second - scroll_x;
                let d_end = (target_x - end_x).abs();
                if d_end < min_distance {
                    min_distance = d_end;
                    closest_snap_x = Some(end_x);
                    closest_snap_time = Some(end_time);
                }
            }
        }
    }

    if let (Some(x), Some(t)) = (closest_snap_x, closest_snap_time) {
        SnapResult {
            snapped_time: t,
            snap_line_x: Some(x),
            did_snap: true,
        }
    } else {
        SnapResult {
            snapped_time: target_time,
            snap_line_x: None,
            did_snap: false,
        }
    }
}
