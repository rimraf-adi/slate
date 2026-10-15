//! Track header rendering and controls.

use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Stroke};
use slate_core::model::{Track, TrackType};
use crate::theme::SlateTheme;

pub const TRACK_HEADER_WIDTH: f32 = 148.0;
pub const DEFAULT_TRACK_HEIGHT: f32 = 54.0;

/// Render the track header on the left side of the timeline lane.
pub fn draw_track_header(
    painter: &Painter,
    rect: Rect,
    track: &Track,
    is_active: bool,
) {
    // Background fill with subtle hover/active gradient
    let bg = if is_active {
        Color32::from_rgb(34, 36, 44)
    } else {
        SlateTheme::BG_DARK
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

    // Track Kind Indicator Accent Bar (4px wide)
    let (bar_color, icon, type_label) = match track.track_type {
        TrackType::Video => (SlateTheme::ACCENT_CYAN, "🎥", "Video Track"),
        TrackType::Audio => (SlateTheme::ACCENT_EMERALD, "🔊", "Audio Track"),
        TrackType::Caption => (SlateTheme::ACCENT_PURPLE, "💬", "Subtitles"),
    };
    let accent_rect = Rect::from_min_size(rect.min, vec2(4.0, rect.height()));
    painter.rect_filled(accent_rect, 0.0, bar_color);

    // Track Icon + Name / Label
    painter.text(
        pos2(rect.left() + 12.0, rect.top() + 8.0),
        Align2::LEFT_TOP,
        format!("{} {}", icon, track.name),
        FontId::proportional(12.5),
        if is_active { SlateTheme::TEXT_PRIMARY } else { SlateTheme::TEXT_SECONDARY },
    );

    // Track Sub-label (type / channels)
    painter.text(
        pos2(rect.left() + 12.0, rect.top() + 25.0),
        Align2::LEFT_TOP,
        type_label,
        FontId::proportional(9.5),
        SlateTheme::TEXT_MUTED,
    );

    // Track Status Badges (Mute, Solo, Lock) on the right side of header
    let badges_x = rect.right() - 8.0;
    let badges_y = rect.center().y;

    let mute_color = if track.muted { SlateTheme::ACCENT_RED } else { Color32::from_rgb(50, 54, 66) };
    let mute_rect = Rect::from_center_size(pos2(badges_x - 36.0, badges_y), vec2(16.0, 16.0));
    painter.rect_filled(mute_rect, 3.0, mute_color);
    painter.text(
        mute_rect.center(),
        Align2::CENTER_CENTER,
        "M",
        FontId::monospace(9.0),
        if track.muted { Color32::WHITE } else { SlateTheme::TEXT_MUTED },
    );

    let lock_color = if track.locked { SlateTheme::ACCENT_AMBER } else { Color32::from_rgb(50, 54, 66) };
    let lock_rect = Rect::from_center_size(pos2(badges_x - 14.0, badges_y), vec2(16.0, 16.0));
    painter.rect_filled(lock_rect, 3.0, lock_color);
    painter.text(
        lock_rect.center(),
        Align2::CENTER_CENTER,
        if track.locked { "🔒" } else { "🔓" },
        FontId::proportional(8.0),
        SlateTheme::TEXT_MUTED,
    );
}
