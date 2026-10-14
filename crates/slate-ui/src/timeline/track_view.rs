//! Track header rendering and controls.

use egui::{pos2, vec2, Align2, FontId, Painter, Rect, Stroke};
use slate_core::model::{Track, TrackType};
use crate::theme::SlateTheme;

pub const TRACK_HEADER_WIDTH: f32 = 120.0;
pub const DEFAULT_TRACK_HEIGHT: f32 = 48.0;

/// Render the track header on the left side of the timeline lane.
pub fn draw_track_header(
    painter: &Painter,
    rect: Rect,
    track: &Track,
    is_active: bool,
) {
    // Background fill
    let bg = if is_active {
        SlateTheme::BG_PANEL
    } else {
        SlateTheme::BG_TRACK
    };
    painter.rect_filled(rect, 0.0, bg);

    // Border line right and bottom
    painter.line_segment(
        [rect.right_top(), rect.right_bottom()],
        Stroke::new(1.0, SlateTheme::BORDER),
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, SlateTheme::BORDER),
    );

    // Track Kind Indicator Accent Bar
    let bar_color = match track.track_type {
        TrackType::Video => SlateTheme::ACCENT_BLUE,
        TrackType::Audio => SlateTheme::ACCENT_GREEN,
        TrackType::Caption => SlateTheme::ACCENT_PURPLE,
    };
    let accent_rect = Rect::from_min_size(rect.min, vec2(4.0, rect.height()));
    painter.rect_filled(accent_rect, 0.0, bar_color);

    // Track Name / Label
    painter.text(
        pos2(rect.left() + 10.0, rect.top() + 8.0),
        Align2::LEFT_TOP,
        &track.name,
        FontId::proportional(12.0),
        SlateTheme::TEXT_PRIMARY,
    );

    // Track Controls Status (Mute / Solo / Lock)
    let status_str = format!(
        "{}{}",
        if track.muted { "M " } else { "" },
        if track.locked { "🔒" } else { "" }
    );
    painter.text(
        pos2(rect.left() + 10.0, rect.bottom() - 16.0),
        Align2::LEFT_TOP,
        status_str,
        FontId::proportional(10.0),
        SlateTheme::TEXT_MUTED,
    );
}
