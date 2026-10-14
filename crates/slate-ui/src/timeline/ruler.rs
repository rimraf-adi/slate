//! Timecode ruler and playhead scrubber rendering.

use egui::{pos2, Align2, Color32, FontId, Painter, Rect, Stroke};
use slate_core::time::{RationalTime, Timecode};
use crate::theme::SlateTheme;

pub const RULER_HEIGHT: f32 = 28.0;

/// Render the timecode ruler with adaptive interval ticks and playhead handle.
pub fn draw_ruler(
    painter: &Painter,
    rect: Rect,
    timebase: u32,
    zoom_px_per_sec: f32,
    scroll_x: f32,
    playhead: RationalTime,
    total_duration_secs: f64,
) {
    // Background
    painter.rect_filled(rect, 0.0, SlateTheme::BG_DARKEST);
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, SlateTheme::BORDER),
    );

    // Adaptive tick interval based on zoom level
    let interval_secs = if zoom_px_per_sec > 200.0 {
        0.5 // Half seconds
    } else if zoom_px_per_sec > 80.0 {
        1.0 // 1 second
    } else if zoom_px_per_sec > 30.0 {
        5.0 // 5 seconds
    } else if zoom_px_per_sec > 10.0 {
        10.0 // 10 seconds
    } else {
        30.0 // 30 seconds
    };

    let start_time_secs = (scroll_x / zoom_px_per_sec).max(0.0) as f64;
    let visible_duration = (rect.width() / zoom_px_per_sec) as f64;
    let end_time_secs = (start_time_secs + visible_duration).min(total_duration_secs.max(60.0));

    let first_tick = (start_time_secs / interval_secs).floor() * interval_secs;
    let mut current_sec = first_tick;

    while current_sec <= end_time_secs + interval_secs {
        let x = rect.left() + (current_sec as f32 * zoom_px_per_sec) - scroll_x;
        if x >= rect.left() && x <= rect.right() {
            // Major tick
            painter.line_segment(
                [pos2(x, rect.bottom() - 8.0), pos2(x, rect.bottom())],
                Stroke::new(1.0, SlateTheme::TEXT_MUTED),
            );

            // Timecode label
            let time_val = RationalTime::from_seconds(current_sec, timebase);
            let tc_str = Timecode::format(time_val, false);
            painter.text(
                pos2(x + 4.0, rect.top() + 4.0),
                Align2::LEFT_TOP,
                tc_str,
                FontId::monospace(10.0),
                SlateTheme::TEXT_SECONDARY,
            );

            // Minor ticks between intervals
            let minor_interval = interval_secs / 4.0;
            for i in 1..4 {
                let minor_x = x + (i as f32 * minor_interval as f32 * zoom_px_per_sec);
                if minor_x >= rect.left() && minor_x <= rect.right() {
                    painter.line_segment(
                        [pos2(minor_x, rect.bottom() - 4.0), pos2(minor_x, rect.bottom())],
                        Stroke::new(1.0, Color32::from_rgb(60, 63, 75)),
                    );
                }
            }
        }
        current_sec += interval_secs;
    }

    // Draw playhead head (red triangle/badge)
    let playhead_x = rect.left() + (playhead.to_seconds() as f32 * zoom_px_per_sec) - scroll_x;
    if playhead_x >= rect.left() - 10.0 && playhead_x <= rect.right() + 10.0 {
        draw_playhead_handle(painter, playhead_x, rect.top(), rect.bottom());
    }
}

pub fn draw_playhead_handle(painter: &Painter, x: f32, top: f32, bottom: f32) {
    let half_w = 6.0;
    // Playhead inverted pentagon handle
    let points = vec![
        pos2(x - half_w, top),
        pos2(x + half_w, top),
        pos2(x + half_w, bottom - 6.0),
        pos2(x, bottom),
        pos2(x - half_w, bottom - 6.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        SlateTheme::PLAYHEAD_LINE,
        Stroke::new(1.0, Color32::WHITE),
    ));
}
