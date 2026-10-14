//! SMPTE timecode display and dropped frame diagnostics.

use egui::{FontId, Ui};
use slate_core::time::{RationalTime, Timecode};
use crate::theme::SlateTheme;

pub fn render_timecode_badge(
    ui: &mut Ui,
    current_time: RationalTime,
    duration: RationalTime,
    dropped_frames: u64,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        // Current timecode
        let tc_current = Timecode::format(current_time, false);
        ui.label(
            egui::RichText::new(tc_current)
                .font(FontId::monospace(18.0))
                .color(SlateTheme::TEXT_PRIMARY)
                .strong(),
        );

        ui.label(
            egui::RichText::new("/")
                .font(FontId::monospace(14.0))
                .color(SlateTheme::TEXT_MUTED),
        );

        // Total duration
        let tc_duration = Timecode::format(duration, false);
        ui.label(
            egui::RichText::new(tc_duration)
                .font(FontId::monospace(14.0))
                .color(SlateTheme::TEXT_SECONDARY),
        );

        // Dropped frame reporting - Goal E5: "Dropped frames are reported, never hidden"
        if dropped_frames > 0 {
            ui.label(
                egui::RichText::new(format!("⚠ {} dropped", dropped_frames))
                    .font(FontId::proportional(11.0))
                    .color(SlateTheme::ACCENT_AMBER),
            );
        } else {
            ui.label(
                egui::RichText::new("● 0 dropped")
                    .font(FontId::proportional(11.0))
                    .color(SlateTheme::ACCENT_GREEN),
            );
        }
    });
}
