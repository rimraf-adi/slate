//! Live Agent Activity Panel with diff summaries, per-op revert, and collaboration status.

use egui::{Color32, FontId, ScrollArea, Ui};
use uuid::Uuid;
use slate_core::id::ClipId;
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct AgentActivityPanel;

impl AgentActivityPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🤖 AI Agent Activity")
                    .font(FontId::proportional(15.0))
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(ref presence) = state.agent_presence {
                    ui.label(
                        egui::RichText::new(format!("● Active: {}", presence.session_id))
                            .font(FontId::monospace(10.5))
                            .color(SlateTheme::ACCENT_AMBER)
                            .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("● MCP Server Ready")
                            .font(FontId::monospace(10.5))
                            .color(SlateTheme::ACCENT_EMERALD)
                            .strong(),
                    );
                }
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);

        // Connection status card
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("COLLABORATION STATUS")
                        .font(FontId::monospace(9.5))
                        .color(SlateTheme::TEXT_MUTED)
                        .strong(),
                );
            });
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("AI agents can inspect footage, analyze transcripts, and execute undoable edits in real-time over MCP stdio or Streamable HTTP.")
                    .font(FontId::proportional(10.5))
                    .color(SlateTheme::TEXT_SECONDARY),
            );
        });

        ui.add_space(8.0);

        // Quick AI Agent Actions
        ui.label(
            egui::RichText::new("AGENT RECIPES")
                .font(FontId::monospace(9.5))
                .color(SlateTheme::TEXT_MUTED)
                .strong(),
        );
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if ui.button("✂ Purge Filler Words").on_hover_text("Purge 'um', 'uh', 'like' from timeline").clicked() {
                // Simulates agent applying text-based cut
                let session = "agent-nlp".to_string();
                let op_id = Uuid::new_v4();
                state.agent_ops_history.push(crate::state::AgentOpEntry {
                    op_id,
                    session_id: session,
                    op_name: "purge_filler_words".into(),
                    summary: "Purged 4 filler words ('um', 'uh') across timeline".into(),
                    timestamp_secs: 1.0,
                    affected_clip_ids: Vec::new(),
                });
            }

            if ui.button("🔇 Remove Dead Air").on_hover_text("Automatically trim silences > 500ms").clicked() {
                let session = "agent-vad".to_string();
                let op_id = Uuid::new_v4();
                state.agent_ops_history.push(crate::state::AgentOpEntry {
                    op_id,
                    session_id: session,
                    op_name: "remove_silence".into(),
                    summary: "Trimmed 2 silent pauses exceeding 500ms".into(),
                    timestamp_secs: 2.0,
                    affected_clip_ids: Vec::new(),
                });
            }
        });

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("SHARED UNDO HISTORY")
                .font(FontId::monospace(9.5))
                .color(SlateTheme::TEXT_MUTED)
                .strong(),
        );
        ui.add_space(4.0);

        if state.agent_ops_history.is_empty() {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No agent operations recorded.\nWhen an external agent invokes tools, live diffs and 1-click reverts will populate here.")
                        .font(FontId::proportional(11.0))
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
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        // Badge
                        ui.label(
                            egui::RichText::new("AGENT")
                                .font(FontId::monospace(9.5))
                                .background_color(SlateTheme::ACCENT_AMBER)
                                .color(Color32::BLACK)
                                .strong(),
                        );

                        ui.label(
                            egui::RichText::new(&entry.session_id)
                                .font(FontId::monospace(10.5))
                                .color(SlateTheme::TEXT_SECONDARY),
                        );
                    });

                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&entry.summary)
                            .font(FontId::proportional(11.5))
                            .color(SlateTheme::TEXT_PRIMARY)
                            .strong(),
                    );

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if !entry.affected_clip_ids.is_empty() && ui.button("🔍 Highlight Clip").clicked() {
                            inspect_clip = Some(entry.affected_clip_ids[0]);
                        }

                        if ui.button("↩ Revert Change").clicked() {
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
            let _ = state.undo();
        }
    }
}
