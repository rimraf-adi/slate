//! Export modal dialog with presets and background job progress.

use egui::{ProgressBar, Window};
use crate::state::{EditorState, ExportPreset};
use crate::theme::SlateTheme;

pub struct ExportModal;

impl ExportModal {
    pub fn show(ctx: &egui::Context, state: &mut EditorState) {
        if !state.export_state.is_open {
            return;
        }

        let mut is_open = state.export_state.is_open;

        Window::new("Export Project")
            .open(&mut is_open)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.heading("Render & Export");
                ui.separator();

                // Preset selection
                ui.label("Format Preset:");
                egui::ComboBox::from_label("")
                    .selected_text(state.export_state.preset.name())
                    .show_ui(ui, |ui| {
                        let presets = [
                            ExportPreset::YouTube1080p,
                            ExportPreset::YouTube4K,
                            ExportPreset::ShortsTikTok9x16,
                            ExportPreset::Square1x1,
                            ExportPreset::ProRes422Hq,
                            ExportPreset::AudioOnlyAac,
                        ];

                        for p in presets {
                            let name = p.name();
                            ui.selectable_value(&mut state.export_state.preset, p, name);
                        }
                    });

                let (w, h) = state.export_state.preset.dimensions();
                if w > 0 && h > 0 {
                    ui.label(
                        egui::RichText::new(format!("Resolution: {}x{}", w, h))
                            .font(egui::FontId::monospace(11.0))
                            .color(SlateTheme::TEXT_SECONDARY),
                    );
                }

                ui.add_space(8.0);

                // Output path
                ui.label("Destination Path:");
                ui.text_edit_singleline(&mut state.export_state.output_path);

                ui.add_space(12.0);

                // Progress Bar
                if state.export_state.is_exporting {
                    ui.add(
                        ProgressBar::new(state.export_state.progress)
                            .show_percentage()
                            .text(&state.export_state.status_text),
                    );

                    ui.add_space(8.0);
                    if ui.button("Cancel Export").clicked() {
                        state.export_state.is_exporting = false;
                        state.export_state.status_text = "Cancelled".to_string();
                    }
                } else {
                    ui.horizontal(|ui| {
                        if ui.button("🚀 Start Render").clicked() {
                            state.export_state.is_exporting = true;
                            state.export_state.progress = 0.05;
                            state.export_state.status_text = "Encoding frames...".to_string();
                        }

                        if ui.button("Close").clicked() {
                            state.export_state.is_open = false;
                        }
                    });
                }
            });

        state.export_state.is_open = is_open;
    }
}
