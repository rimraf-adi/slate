//! Playback transport controls and J/K/L shuttle shortcuts.

use egui::{Response, Ui};
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub fn render_transport_controls(ui: &mut Ui, state: &mut EditorState) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // J - Shuttle Reverse
        if ui.button("⏪ J").clicked() || ui.input(|i| i.key_pressed(egui::Key::J)) {
            state.shuttle_backward();
        }

        // Step 1 frame back (Left arrow)
        if ui.button("⏮").clicked() || ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            state.step_frame(-1);
        }

        // Play / Pause (Space)
        let play_label = if state.is_playing { "⏸ Pause" } else { "▶ Play" };
        if ui.button(play_label).clicked() || ui.input(|i| i.key_pressed(egui::Key::Space)) {
            state.toggle_play();
        }

        // Step 1 frame forward (Right arrow)
        if ui.button("⏭").clicked() || ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            state.step_frame(1);
        }

        // L - Shuttle Forward
        if ui.button("L ⏩").clicked() || ui.input(|i| i.key_pressed(egui::Key::L)) {
            state.shuttle_forward();
        }

        // K - Shuttle Stop / Pause
        if ui.input(|i| i.key_pressed(egui::Key::K)) {
            state.shuttle_pause();
        }

        // Shuttle speed indicator
        if state.is_playing {
            let speed_badge = format!("{}x", state.shuttle_speed);
            ui.label(
                egui::RichText::new(speed_badge)
                    .color(SlateTheme::ACCENT_BLUE_LIGHT)
                    .strong(),
            );
        }
    })
    .response
}
