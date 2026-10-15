//! Playback transport controls and J/K/L shuttle shortcuts.

use egui::{vec2, Button, Response, Ui};
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub fn render_transport_controls(ui: &mut Ui, state: &mut EditorState) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // Home / Start of timeline
        if ui.button("⏮").on_hover_text("Go to Start (Home)").clicked() {
            state.seek(slate_core::time::RationalTime::ZERO);
        }

        // J - Shuttle Reverse
        if ui.button("⏪ J").on_hover_text("Shuttle Reverse (J)").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::J))
        {
            state.shuttle_backward();
        }

        // Step 1 frame back (Left arrow)
        if ui.button("◀").on_hover_text("Step 1 Frame Back (←)").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::ArrowLeft))
        {
            state.step_frame(-1);
        }

        // Play / Pause (Space) with prominent styling
        let (play_label, play_icon) = if state.is_playing {
            (" Pause", "⏸")
        } else {
            (" Play", "▶")
        };
        let play_btn = Button::new(
            egui::RichText::new(format!("{}{}", play_icon, play_label))
                .strong()
                .color(if state.is_playing { SlateTheme::TEXT_PRIMARY } else { egui::Color32::WHITE }),
        )
        .min_size(vec2(80.0, 26.0))
        .fill(if state.is_playing { SlateTheme::ACCENT_AMBER } else { SlateTheme::ACCENT_BLUE });

        if ui.add(play_btn).on_hover_text("Play / Pause (Space)").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::Space))
        {
            state.toggle_play();
        }

        // Step 1 frame forward (Right arrow)
        if ui.button("▶").on_hover_text("Step 1 Frame Forward (→)").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::ArrowRight))
        {
            state.step_frame(1);
        }

        // L - Shuttle Forward
        if ui.button("L ⏩").on_hover_text("Shuttle Forward (L)").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::L))
        {
            state.shuttle_forward();
        }

        // K - Shuttle Stop / Pause
        if ui.input(|i| i.key_pressed(egui::Key::K)) {
            state.shuttle_pause();
        }

        // Shuttle speed indicator badge
        if state.is_playing && state.shuttle_speed != 1 {
            let speed_badge = format!("{}x", state.shuttle_speed);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(speed_badge)
                    .font(egui::FontId::monospace(11.0))
                    .background_color(SlateTheme::ACCENT_BLUE)
                    .color(egui::Color32::WHITE)
                    .strong(),
            );
        }
    })
    .response
}
