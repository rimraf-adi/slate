//! Timeline editing tools bar and keyboard shortcut mappings.

use egui::{Response, Ui};
use crate::state::ToolMode;

pub fn render_tools_bar(ui: &mut Ui, active_tool: &mut ToolMode) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        let tools = [
            (ToolMode::Select, "↖ Select (V)", 'V'),
            (ToolMode::Blade, "✂ Blade (C)", 'C'),
            (ToolMode::TrimRipple, "⇥ Ripple (B)", 'B'),
            (ToolMode::Slip, "⇄ Slip (Y)", 'Y'),
            (ToolMode::Slide, "↔ Slide (U)", 'U'),
        ];

        for (mode, label, shortcut) in tools {
            let is_selected = *active_tool == mode;
            if ui.selectable_label(is_selected, label).clicked() {
                *active_tool = mode;
            }

            // Global shortcut handler
            if ui.input(|i| i.key_pressed(key_from_char(shortcut))) {
                *active_tool = mode;
            }
        }
    })
    .response
}

fn key_from_char(c: char) -> egui::Key {
    match c {
        'V' => egui::Key::V,
        'C' => egui::Key::C,
        'B' => egui::Key::B,
        'Y' => egui::Key::Y,
        'U' => egui::Key::U,
        _ => egui::Key::V,
    }
}
