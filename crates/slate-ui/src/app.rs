//! Main eframe::App implementation for the Slate video editor.

use egui::{vec2, CentralPanel, FontId, SidePanel, TopBottomPanel};
use slate_core::engine::Engine;
use slate_core::model::Project;

use crate::state::EditorState;
use crate::theme::SlateTheme;

use crate::agent_panel::AgentActivityPanel;
use crate::audio_monitor::AudioMonitor;
use crate::export::ExportModal;
use crate::inspector::InspectorPanel;
use crate::media_pool::MediaPoolPanel;
use crate::timeline::tools::render_tools_bar;
use crate::timeline::TimelineWidget;
use crate::transcript::TranscriptPanel;
use crate::viewport::ViewportWidget;

#[derive(Debug, PartialEq, Eq)]
enum LeftTab {
    MediaPool,
    Transcript,
}

#[derive(Debug, PartialEq, Eq)]
enum RightTab {
    Inspector,
    AgentActivity,
}

pub struct SlateApp {
    pub state: EditorState,
    left_tab: LeftTab,
    right_tab: RightTab,
}

impl SlateApp {
    pub fn new(engine: Engine) -> Self {
        Self {
            state: EditorState::new(engine),
            left_tab: LeftTab::MediaPool,
            right_tab: RightTab::Inspector,
        }
    }

    pub fn new_with_project(mut project: Project) -> Self {
        if let Some(tl) = project.active_timeline_mut() {
            if tl.tracks.is_empty() {
                tl.tracks.push(slate_core::model::Track::new("V1", slate_core::model::TrackType::Video));
                tl.tracks.push(slate_core::model::Track::new("A1", slate_core::model::TrackType::Audio));
            }
        }
        let engine = Engine::new(project, 200);
        Self::new(engine)
    }
}

impl eframe::App for SlateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply custom dark theme
        SlateTheme::apply(ctx);

        // Update real-time playback clock
        self.state.tick_playback();
        if self.state.is_playing {
            ctx.request_repaint();
        }

        // Global Keyboard Shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            if ctx.input(|i| i.modifiers.shift) {
                let _ = self.state.redo();
            } else {
                let _ = self.state.undo();
            }
        }

        // 1. Top Studio Application Header Bar
        TopBottomPanel::top("top_panel")
            .exact_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    // Brand Identity Mark
                    ui.label(
                        egui::RichText::new("● SLATE")
                            .font(FontId::proportional(15.0))
                            .color(SlateTheme::ACCENT_CYAN)
                            .strong(),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Menu Bar
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("New Project").clicked() {
                                ui.close_menu();
                            }
                            if ui.button("Save Project (Cmd+S)").clicked() {
                                let engine = self.state.engine.lock();
                                let _ = slate_core::project::ProjectBundle::save_to_bundle(
                                    std::path::Path::new("/tmp/slate_autosave.slateproj"),
                                    engine.project(),
                                );
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Export Video...").clicked() {
                                self.state.export_state.is_open = true;
                                ui.close_menu();
                            }
                        });

                        ui.menu_button("Edit", |ui| {
                            if ui.button("Undo (Cmd+Z)").clicked() {
                                let _ = self.state.undo();
                                ui.close_menu();
                            }
                            if ui.button("Redo (Cmd+Shift+Z)").clicked() {
                                let _ = self.state.redo();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Split at Playhead (C)").clicked() {
                                let _ = self.state.split_at_playhead();
                                ui.close_menu();
                            }
                            if ui.button("Ripple Delete (Del)").clicked() {
                                let _ = self.state.ripple_delete_selected();
                                ui.close_menu();
                            }
                        });

                        ui.menu_button("Timeline", |ui| {
                            if ui.button("Add Video Track").clicked() {
                                let op = slate_core::op::Op::AddTrack {
                                    name: "V2".into(),
                                    track_type: slate_core::model::TrackType::Video,
                                    index: None,
                                };
                                let _ = self.state.apply_op(op, slate_core::OpSource::Human);
                                ui.close_menu();
                            }
                            if ui.button("Add Audio Track").clicked() {
                                let op = slate_core::op::Op::AddTrack {
                                    name: "A2".into(),
                                    track_type: slate_core::model::TrackType::Audio,
                                    index: None,
                                };
                                let _ = self.state.apply_op(op, slate_core::OpSource::Human);
                                ui.close_menu();
                            }
                        });

                        ui.menu_button("Agent", |ui| {
                            if ui.button("View MCP Activity").clicked() {
                                self.right_tab = RightTab::AgentActivity;
                                ui.close_menu();
                            }
                        });
                    });

                    // Right Side: Editing Tools, Undo/Redo & Primary Export
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);

                        // Export Button
                        let export_btn = egui::Button::new(
                            egui::RichText::new("🚀 Export Video")
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .min_size(vec2(100.0, 26.0))
                        .fill(SlateTheme::ACCENT_BLUE);

                        if ui.add(export_btn).clicked() {
                            self.state.export_state.is_open = true;
                        }

                        ui.separator();

                        // Undo / Redo
                        if ui.button("↪").on_hover_text("Redo (Cmd+Shift+Z)").clicked() {
                            let _ = self.state.redo();
                        }
                        if ui.button("↩").on_hover_text("Undo (Cmd+Z)").clicked() {
                            let _ = self.state.undo();
                        }

                        ui.separator();

                        // Editing Tools Segmented Bar
                        render_tools_bar(ui, &mut self.state.active_tool);
                    });
                });
            });

        // 2. Left Side Panel: Media Pool & Transcript
        SidePanel::left("left_panel")
            .resizable(true)
            .default_width(290.0)
            .width_range(220.0..=480.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.left_tab, LeftTab::MediaPool, "🎬 Media Pool");
                    ui.selectable_value(&mut self.left_tab, LeftTab::Transcript, "📝 Transcript");
                });
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(4.0);

                match self.left_tab {
                    LeftTab::MediaPool => MediaPoolPanel::show(ui, &mut self.state),
                    LeftTab::Transcript => TranscriptPanel::show(ui, &mut self.state),
                }
            });

        // 3. Right Side Panel: Inspector & Agent Activity
        SidePanel::right("right_panel")
            .resizable(true)
            .default_width(310.0)
            .width_range(240.0..=480.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.right_tab, RightTab::Inspector, "⚙ Inspector");
                    ui.selectable_value(&mut self.right_tab, RightTab::AgentActivity, "🤖 AI Agent");
                });
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(4.0);

                match self.right_tab {
                    RightTab::Inspector => InspectorPanel::show(ui, &mut self.state),
                    RightTab::AgentActivity => AgentActivityPanel::show(ui, &mut self.state),
                }
            });

        // 4. Bottom Panel: Multi-track Timeline
        TopBottomPanel::bottom("bottom_timeline")
            .resizable(true)
            .default_height(310.0)
            .height_range(180.0..=650.0)
            .show(ctx, |ui| {
                TimelineWidget::show(ui, &mut self.state);
            });

        // 5. Central Panel: Viewport Monitor + Audio VU Meter
        CentralPanel::default().show(ctx, |ui| {
            let total_avail = ui.available_size();
            let audio_meter_width = 54.0;
            let spacing = 8.0;
            let viewport_width = (total_avail.x - audio_meter_width - spacing).max(100.0);

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(viewport_width, total_avail.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ViewportWidget::show(ui, &mut self.state);
                    },
                );

                ui.add_space(spacing);

                ui.allocate_ui_with_layout(
                    vec2(audio_meter_width, total_avail.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        AudioMonitor::show(ui, self.state.audio_peak_l, self.state.audio_peak_r);
                    },
                );
            });
        });

        // 6. Export Modal
        ExportModal::show(ctx, &mut self.state);
    }
}
