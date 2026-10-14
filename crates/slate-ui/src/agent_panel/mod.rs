//! Live Agent Activity Panel with diff summaries, per-op revert, and collaboration status.

use egui::{FontId, ScrollArea, Ui};
use uuid::Uuid;
use slate_core::id::ClipId;
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct AgentActivityPanel;

impl AgentActivityPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            ui.heading("Agent Activity");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(ref presence) = state.agent_presence {
                    ui.label(
                        egui::RichText::new(format!("● Active: {}", presence.session_id))
                            .color(SlateTheme::ACCENT_AMBER)
                            .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("○ Idle / Listening")
                            .color(SlateTheme::TEXT_MUTED),
                    );
                }
            });
        });

        ui.separator();

        if state.agent_ops_history.is_empty() {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No agent edits yet.\nWhen an AI agent interacts via MCP or IPC, changes appear here live.")
                        .color(SlateTheme::TEXT_MUTED)
                        .italics(),
                );
            });
            return;
        }

        let mut revert_target: Option<Uuid> = None;
        let mut inspect_clip: Option<ClipId> = None;

        ScrollArea::vertical().show(ui, |ui| {
            for entry in state.agent_ops_history.iter().rev() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Badge
                        ui.label(
                            egui::RichText::new("AGENT")
                                .font(FontId::monospace(10.0))
                                .background_color(SlateTheme::ACCENT_AMBER)
                                .color(SlateTheme::BG_DARKEST)
                                .strong(),
                        );

                        ui.label(
                            egui::RichText::new(&entry.session_id)
                                .font(FontId::monospace(11.0))
                                .color(SlateTheme::TEXT_SECONDARY),
                        );
                    });

                    ui.label(
                        egui::RichText::new(&entry.summary)
                            .font(FontId::proportional(12.0))
                            .color(SlateTheme::TEXT_PRIMARY)
                            .strong(),
                    );

                    ui.horizontal(|ui| {
                        if !entry.affected_clip_ids.is_empty() && ui.button("🔍 Highlight Clip").clicked() {
                            inspect_clip = Some(entry.affected_clip_ids[0]);
                        }

                        if ui.button("↩ Revert").clicked() {
                            revert_target = Some(entry.op_id);
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });

        if let Some(clip_id) = inspect_clip {
            state.selected_clip_ids.clear();
            state.selected_clip_ids.insert(clip_id);
        }

        if let Some(_op_id) = revert_target {
            // Revert last op via undo
            let _ = state.undo();
        }
    }
}
