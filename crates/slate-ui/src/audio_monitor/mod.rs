//! Stereo peak & RMS volume meter bars.

use egui::{pos2, vec2, Rect, Stroke, Ui};
use crate::theme::SlateTheme;

pub struct AudioMonitor;

impl AudioMonitor {
    pub fn show(ui: &mut Ui, peak_l: f32, peak_r: f32) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            draw_channel_bar(ui, peak_l);
            draw_channel_bar(ui, peak_r);
        });
    }
}

fn draw_channel_bar(ui: &mut Ui, level: f32) {
    let (rect, _resp) = ui.allocate_exact_size(vec2(12.0, 100.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Background channel track
    painter.rect_filled(rect, 2.0, SlateTheme::BG_DARKEST);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, SlateTheme::BORDER));

    // Clamp level 0.0 to 1.0
    let clamped_level = level.clamp(0.0, 1.0);
    let fill_h = rect.height() * clamped_level;

    if fill_h > 0.0 {
        let fill_rect = Rect::from_min_max(
            pos2(rect.left(), rect.bottom() - fill_h),
            rect.right_bottom(),
        );

        let color = if clamped_level > 0.95 {
            SlateTheme::ACCENT_RED
        } else if clamped_level > 0.8 {
            SlateTheme::ACCENT_AMBER
        } else {
            SlateTheme::ACCENT_GREEN
        };

        painter.rect_filled(fill_rect, 1.0, color);
    }
}
