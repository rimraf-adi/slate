//! Main eframe::App implementation for the Slate video editor.

use egui::{CentralPanel, TopBottomPanel, SidePanel};
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

        // 1. Top Menu & Toolbar
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Save (Cmd+S)").clicked() {
                        let engine = self.state.engine.lock();
                        let _ = slate_core::project::ProjectBundle::save_to_bundle(
                            std::path::Path::new("/tmp/slate_autosave.slateproj"),
                            engine.project(),
                        );
                        ui.close_menu();
                    }
                    if ui.button("Export...").clicked() {
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

                ui.menu_button("Agent", |ui| {
                    if ui.button("View MCP Status").clicked() {
                        self.right_tab = RightTab::AgentActivity;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🚀 Export").clicked() {
                        self.state.export_state.is_open = true;
                    }

                    // Magnetic Snapping toggle
                    let snap_label = if self.state.snap_enabled { "🧲 Snap: ON" } else { "🧲 Snap: OFF" };
                    if ui.selectable_label(self.state.snap_enabled, snap_label).clicked() {
                        self.state.snap_enabled = !self.state.snap_enabled;
                    }

                    // Undo/Redo quick buttons
                    if ui.button("↪").clicked() {
                        let _ = self.state.redo();
                    }
                    if ui.button("↩").clicked() {
                        let _ = self.state.undo();
                    }

                    render_tools_bar(ui, &mut self.state.active_tool);
                });
            });
        });

        // 2. Left Side Panel: Media Pool & Transcript
        SidePanel::left("left_panel")
            .resizable(true)
            .default_width(280.0)
            .width_range(200.0..=450.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.left_tab, LeftTab::MediaPool, "🎬 Media Pool");
                    ui.selectable_value(&mut self.left_tab, LeftTab::Transcript, "📝 Transcript");
                });
                ui.separator();

                match self.left_tab {
                    LeftTab::MediaPool => MediaPoolPanel::show(ui, &mut self.state),
                    LeftTab::Transcript => TranscriptPanel::show(ui, &mut self.state),
                }
            });

        // 3. Right Side Panel: Inspector & Agent Activity
        SidePanel::right("right_panel")
            .resizable(true)
            .default_width(300.0)
            .width_range(220.0..=450.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.right_tab, RightTab::Inspector, "⚙ Inspector");
                    ui.selectable_value(&mut self.right_tab, RightTab::AgentActivity, "🤖 Agent Activity");
                });
                ui.separator();

                match self.right_tab {
                    RightTab::Inspector => InspectorPanel::show(ui, &mut self.state),
                    RightTab::AgentActivity => AgentActivityPanel::show(ui, &mut self.state),
                }
            });

        // 4. Bottom Panel: Multi-track Timeline
        TopBottomPanel::bottom("bottom_timeline")
            .resizable(true)
            .default_height(280.0)
            .height_range(160.0..=600.0)
            .show(ctx, |ui| {
                TimelineWidget::show(ui, &mut self.state);
            });

        // 5. Central Viewport & Audio Meters
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ViewportWidget::show(ui, &mut self.state);
                });
                ui.add_space(8.0);
                AudioMonitor::show(ui, self.state.audio_peak_l, self.state.audio_peak_r);
            });
        });

        // 6. Export Modal
        ExportModal::show(ctx, &mut self.state);
    }
}
