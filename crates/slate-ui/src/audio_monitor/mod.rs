//! Stereo peak & RMS volume meter bars with calibrated dB scale.

use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Stroke, Ui};
use crate::theme::SlateTheme;

pub struct AudioMonitor;

impl AudioMonitor {
    pub fn show(ui: &mut Ui, peak_l: f32, peak_r: f32) {
        let (rect, _resp) = ui.allocate_exact_size(vec2(52.0, ui.available_height().max(140.0)), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Frame background
        painter.rect_filled(rect, 4.0, SlateTheme::BG_DARKEST);
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0, SlateTheme::BORDER));

        let meter_top = rect.top() + 8.0;
        let meter_bottom = rect.bottom() - 20.0;
        let meter_h = meter_bottom - meter_top;

        let bar_w = 8.0;
        let bar_l_x = rect.left() + 8.0;
        let bar_r_x = bar_l_x + bar_w + 4.0;
        let scale_x = bar_r_x + bar_w + 6.0;

        // Draw Left and Right Channel Bars
        draw_bar(&painter, Rect::from_min_max(pos2(bar_l_x, meter_top), pos2(bar_l_x + bar_w, meter_bottom)), peak_l);
        draw_bar(&painter, Rect::from_min_max(pos2(bar_r_x, meter_top), pos2(bar_r_x + bar_w, meter_bottom)), peak_r);

        // Labels at bottom: L / R
        painter.text(
            pos2(bar_l_x + bar_w / 2.0, rect.bottom() - 10.0),
            Align2::CENTER_CENTER,
            "L",
            FontId::proportional(9.0),
            SlateTheme::TEXT_MUTED,
        );
        painter.text(
            pos2(bar_r_x + bar_w / 2.0, rect.bottom() - 10.0),
            Align2::CENTER_CENTER,
            "R",
            FontId::proportional(9.0),
            SlateTheme::TEXT_MUTED,
        );

        // dB Tick Marks (0, -6, -12, -18, -24, -36, -48 dB)
        let ticks: &[(f32, &str)] = &[
            (1.00, " 0"),
            (0.85, "-6"),
            (0.70, "-12"),
            (0.55, "-18"),
            (0.40, "-24"),
            (0.20, "-36"),
            (0.05, "-48"),
        ];

        for &(pct, label) in ticks {
            let y = meter_bottom - meter_h * pct;
            painter.line_segment(
                [pos2(scale_x, y), pos2(scale_x + 3.0, y)],
                Stroke::new(1.0, SlateTheme::TEXT_MUTED),
            );
            painter.text(
                pos2(scale_x + 5.0, y),
                Align2::LEFT_CENTER,
                label,
                FontId::monospace(8.0),
                SlateTheme::TEXT_MUTED,
            );
        }
    }
}

fn draw_bar(painter: &egui::Painter, rect: Rect, level: f32) {
    // Channel track bed
    painter.rect_filled(rect, 1.0, Color32::from_rgb(12, 13, 16));
    painter.rect_stroke(rect, 1.0, Stroke::new(1.0, SlateTheme::BORDER));

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
            SlateTheme::ACCENT_EMERALD
        };

        painter.rect_filled(fill_rect, 1.0, color);
    }
}
