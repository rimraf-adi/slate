//! Clip rendering, hit testing, and visual styling.

use egui::{pos2, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke};
use slate_core::model::{Clip, TrackType};
use crate::theme::SlateTheme;

pub const TRIM_HANDLE_WIDTH: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipHitArea {
    TrimInHandle,
    TrimOutHandle,
    Body,
}

pub struct ClipGeometry {
    pub rect: Rect,
    pub trim_in_rect: Rect,
    pub trim_out_rect: Rect,
}

impl ClipGeometry {
    pub fn new(
        clip: &Clip,
        track_y: f32,
        track_height: f32,
        track_header_w: f32,
        zoom_px_per_sec: f32,
        scroll_x: f32,
    ) -> Self {
        let start_x = track_header_w + (clip.timeline_range.start.to_seconds() as f32 * zoom_px_per_sec) - scroll_x;
        let duration_px = (clip.timeline_range.duration.to_seconds() as f32 * zoom_px_per_sec).max(4.0);
        let end_x = start_x + duration_px;

        let padding_y = 2.0;
        let rect = Rect::from_min_max(
            pos2(start_x, track_y + padding_y),
            pos2(end_x, track_y + track_height - padding_y),
        );

        let handle_w = TRIM_HANDLE_WIDTH.min(duration_px / 3.0);
        let trim_in_rect = Rect::from_min_max(
            pos2(rect.left(), rect.top()),
            pos2(rect.left() + handle_w, rect.bottom()),
        );
        let trim_out_rect = Rect::from_min_max(
            pos2(rect.right() - handle_w, rect.top()),
            pos2(rect.right(), rect.bottom()),
        );

        Self {
            rect,
            trim_in_rect,
            trim_out_rect,
        }
    }

    pub fn hit_test(&self, pos: Pos2) -> Option<ClipHitArea> {
        if !self.rect.contains(pos) {
            return None;
        }
        if self.trim_in_rect.contains(pos) {
            Some(ClipHitArea::TrimInHandle)
        } else if self.trim_out_rect.contains(pos) {
            Some(ClipHitArea::TrimOutHandle)
        } else {
            Some(ClipHitArea::Body)
        }
    }
}

pub fn draw_clip(
    painter: &Painter,
    geom: &ClipGeometry,
    clip: &Clip,
    track_type: TrackType,
    is_selected: bool,
    is_agent_highlighted: bool,
) {
    let (bg_color, border_color) = match track_type {
        TrackType::Video => (SlateTheme::CLIP_VIDEO, SlateTheme::CLIP_VIDEO_BORDER),
        TrackType::Audio => (SlateTheme::CLIP_AUDIO, SlateTheme::CLIP_AUDIO_BORDER),
        TrackType::Caption => (SlateTheme::CLIP_TITLE, SlateTheme::CLIP_TITLE_BORDER),
    };

    // Draw main clip body
    painter.rect_filled(geom.rect, 4.0, bg_color);

    // If audio track, draw synthetic audio waveform peaks inside clip
    if track_type == TrackType::Audio && geom.rect.width() > 20.0 {
        draw_audio_waveform(painter, geom.rect);
    }

    // Border: Selection > Agent > Normal
    let stroke = if is_selected {
        Stroke::new(2.0, SlateTheme::CLIP_SELECTED_BORDER)
    } else if is_agent_highlighted {
        Stroke::new(2.0, SlateTheme::CLIP_AGENT_BORDER)
    } else {
        Stroke::new(1.0, border_color)
    };
    painter.rect_stroke(geom.rect, 4.0, stroke);

    // Clip Name Label
    if geom.rect.width() > 30.0 {
        let label_pos = pos2(geom.rect.left() + 8.0, geom.rect.top() + 4.0);
        painter.text(
            label_pos,
            Align2::LEFT_TOP,
            &clip.name,
            FontId::proportional(11.0),
            SlateTheme::TEXT_PRIMARY,
        );
    }

    // Trim handle affordances (subtle edge bars)
    if is_selected && geom.rect.width() > 20.0 {
        painter.rect_filled(
            geom.trim_in_rect,
            2.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 60),
        );
        painter.rect_filled(
            geom.trim_out_rect,
            2.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 60),
        );
    }
}

fn draw_audio_waveform(painter: &Painter, rect: Rect) {
    let center_y = rect.center().y;
    let wave_h = (rect.height() - 16.0) / 2.0;
    let step = 4.0;
    let num_bars = ((rect.width() - 16.0) / step).max(1.0) as usize;

    let wave_color = Color32::from_rgba_unmultiplied(16, 185, 129, 90);

    for i in 0..num_bars {
        let x = rect.left() + 8.0 + (i as f32 * step);
        // Harmonic waveform envelope
        let phase = (i as f32 * 0.25).sin().abs() * 0.7 + (i as f32 * 0.08).cos().abs() * 0.3;
        let bar_h = wave_h * phase;
        painter.line_segment(
            [pos2(x, center_y - bar_h), pos2(x, center_y + bar_h)],
            Stroke::new(2.0, wave_color),
        );
    }
}
