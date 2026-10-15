//! Main eframe::App implementation for the Slate video editor.

use std::path::{Path, PathBuf};
use egui::{vec2, CentralPanel, Color32, FontId, SidePanel, TopBottomPanel};
use slate_core::engine::Engine;
use slate_core::model::{Clip, MediaMetadata, MediaReference, Project, TrackItem, TrackType};
use slate_core::project::ProjectBundle;
use slate_core::time::{RationalTime, TimeRange, Timecode};

use crate::file_browser::{FileBrowserModal, FileBrowserMode};
use crate::homescreen::recents::RecentProject;
use crate::homescreen::{HomeScreenAction, HomeScreenModal};
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
    pub home_screen: HomeScreenModal,
    pub file_browser: FileBrowserModal,
    pub current_project_path: Option<PathBuf>,
}

impl SlateApp {
    pub fn new(engine: Engine) -> Self {
        Self {
            state: EditorState::new(engine),
            left_tab: LeftTab::MediaPool,
            right_tab: RightTab::Inspector,
            home_screen: HomeScreenModal::default(),
            file_browser: FileBrowserModal::default(),
            current_project_path: None,
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

    pub fn record_current_project_in_recents(&mut self) {
        let engine = self.state.engine.lock();
        let proj = engine.project();
        let dur_secs = proj.active_timeline().map(|t| t.duration().to_seconds()).unwrap_or(0.0);
        let res = proj.active_timeline().map(|t| [t.settings.width, t.settings.height]).unwrap_or([1920, 1080]);
        let fps = proj.active_timeline().map(|t| t.settings.timebase as f64).unwrap_or(30.0);
        let track_count = proj.active_timeline().map(|t| t.tracks.len()).unwrap_or(0);
        let clip_count = proj.active_timeline().map(|t| t.tracks.iter().map(|tr| tr.items.len()).sum()).unwrap_or(0);

        let path = self.current_project_path.clone().unwrap_or_else(|| {
            PathBuf::from(format!("/tmp/slate_projects/{}.slateproj", proj.name.replace(' ', "_")))
        });

        let item = RecentProject::new(
            proj.name.clone(),
            path,
            dur_secs,
            res,
            fps,
            track_count,
            clip_count,
        );
        drop(engine);
        self.home_screen.recents_store.add(item);
    }

    pub fn load_project_path(&mut self, path: &Path) {
        let loaded = if path.join(ProjectBundle::PROJECT_JSON).exists() || path.is_dir() {
            ProjectBundle::load_from_bundle(path).ok()
        } else if path.exists() {
            if let Ok(data) = std::fs::read_to_string(path) {
                serde_json::from_str::<Project>(&data).ok()
            } else {
                None
            }
        } else {
            None
        };

        if let Some(project) = loaded {
            let name = project.name.clone();
            self.current_project_path = Some(path.to_path_buf());
            self.state.set_project(project);
            self.record_current_project_in_recents();
            self.state.set_status(format!("✓ Opened project: {}", name), false);
        } else {
            // If sample file doesn't exist yet on disk, create it dynamically
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Studio Project");
            let mut proj = Project::new(name);
            if let Some(tl) = proj.active_timeline_mut() {
                tl.tracks.push(slate_core::model::Track::new("V1", TrackType::Video));
                tl.tracks.push(slate_core::model::Track::new("A1", TrackType::Audio));
            }
            let _ = ProjectBundle::save_to_bundle(path, &proj);
            self.current_project_path = Some(path.to_path_buf());
            self.state.set_project(proj);
            self.record_current_project_in_recents();
            self.state.set_status(format!("✓ Initialized project: {}", name), false);
        }
    }

    pub fn save_current_project(&mut self) {
        if let Some(path) = self.current_project_path.clone() {
            let engine = self.state.engine.lock();
            if let Err(e) = ProjectBundle::save_to_bundle(&path, engine.project()) {
                drop(engine);
                self.state.set_status(format!("Failed to save: {}", e), true);
            } else {
                drop(engine);
                self.record_current_project_in_recents();
                self.state.set_status(format!("✓ Saved project to {}", path.display()), false);
            }
        } else {
            self.file_browser.open(FileBrowserMode::SaveProjectAs);
        }
    }

    pub fn create_sample_project(&mut self) {
        let mut proj = Project::new("Tokyo Cyberpunk Demo");
        let sample_path = PathBuf::from("/tmp/slate_samples/tokyo_cyberpunk.slateproj");

        if let Some(tl) = proj.active_timeline_mut() {
            tl.settings.width = 1920;
            tl.settings.height = 1080;
            tl.settings.timebase = 30;

            let mut v1 = slate_core::model::Track::new("V1", TrackType::Video);
            let mut v2 = slate_core::model::Track::new("V2", TrackType::Video);
            let mut a1 = slate_core::model::Track::new("A1", TrackType::Audio);

            let clip_v1 = Clip::new(
                "Shibuya_Crossing_Night.mp4",
                slate_core::id::MediaId::new(),
                TimeRange::new(RationalTime::from_seconds(0.0, 30), RationalTime::from_seconds(6.0, 30)),
                TimeRange::new(RationalTime::from_seconds(0.0, 30), RationalTime::from_seconds(6.0, 30)),
            );
            let clip_v2 = Clip::new(
                "Neon_Sign_B-Roll.mp4",
                slate_core::id::MediaId::new(),
                TimeRange::new(RationalTime::from_seconds(0.0, 30), RationalTime::from_seconds(4.0, 30)),
                TimeRange::new(RationalTime::from_seconds(6.0, 30), RationalTime::from_seconds(4.0, 30)),
            );
            let clip_a1 = Clip::new(
                "Synthwave_Soundtrack.wav",
                slate_core::id::MediaId::new(),
                TimeRange::new(RationalTime::from_seconds(0.0, 48000), RationalTime::from_seconds(10.0, 48000)),
                TimeRange::new(RationalTime::from_seconds(0.0, 48000), RationalTime::from_seconds(10.0, 48000)),
            );

            v1.items.push(TrackItem::Clip(clip_v1));
            v2.items.push(TrackItem::Clip(clip_v2));
            a1.items.push(TrackItem::Clip(clip_a1));

            tl.tracks.push(v1);
            tl.tracks.push(v2);
            tl.tracks.push(a1);
        }

        let _ = ProjectBundle::save_to_bundle(&sample_path, &proj);
        self.current_project_path = Some(sample_path);
        self.state.set_project(proj);
        self.record_current_project_in_recents();
        self.state.set_status("✓ Loaded Tokyo Cyberpunk Demo Project", false);
    }

    pub fn import_media_file(&mut self, path: &Path) {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let metadata = match slate_media::probe::MediaProbe::probe(path) {
            Ok(meta) => meta,
            Err(_) => {
                let lower = file_name.to_lowercase();
                let is_video = lower.ends_with(".mp4") || lower.ends_with(".mov") || lower.ends_with(".mkv");
                let is_audio = lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.ends_with(".aac");
                MediaMetadata {
                    file_size_bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(1024 * 1024 * 15),
                    duration: RationalTime::from_seconds(12.0, 30),
                    video_codec: if is_video { Some("h264".into()) } else { None },
                    width: if is_video { Some(1920) } else { None },
                    height: if is_video { Some(1080) } else { None },
                    frame_rate: if is_video { Some(30.0) } else { None },
                    timebase: 30,
                    audio_codec: if is_audio || is_video { Some("aac".into()) } else { None },
                    audio_channels: Some(2),
                    sample_rate: Some(48000),
                }
            }
        };

        let asset = MediaReference {
            id: slate_core::id::MediaId::new(),
            original_path: path.to_path_buf(),
            relative_path: PathBuf::from(&file_name),
            metadata,
            proxy_path: None,
            waveform_path: None,
        };

        {
            let mut engine = self.state.engine.lock();
            engine.project_mut().media_pool.push(asset.clone());
        }
        crate::media_pool::insert_asset_onto_timeline(&mut self.state, &asset);
        self.state.set_status(format!("✓ Imported media file: {}", file_name), false);
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

        // Check if media pool requested file browser import
        if self.state.request_file_browser_import {
            self.state.request_file_browser_import = false;
            self.file_browser.open(FileBrowserMode::ImportMedia);
        }

        // Global Keyboard Shortcuts (NLE Industry Standard)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            if ctx.input(|i| i.modifiers.shift) {
                let _ = self.state.redo();
            } else {
                let _ = self.state.undo();
            }
        }

        // Homescreen / Project Manager (Cmd+Shift+H / Cmd+H)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::H)) {
            self.home_screen.open();
        }

        // New Project (Cmd+N)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && !i.modifiers.shift && i.key_pressed(egui::Key::N)) {
            self.home_screen.open();
            self.home_screen.show_new_dialog = true;
        }

        // Open Project (Cmd+O)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && !i.modifiers.shift && i.key_pressed(egui::Key::O)) {
            self.file_browser.open(FileBrowserMode::OpenProject);
        }

        // Import Media (Cmd+I)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::I)) {
            self.file_browser.open(FileBrowserMode::ImportMedia);
        }

        // Split / Cut (Cmd+K / Ctrl+K / S)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::K))
            || ctx.input(|i| !i.modifiers.command && !i.modifiers.ctrl && i.key_pressed(egui::Key::S))
        {
            let _ = self.state.split_selected_or_at_playhead();
        }

        // Ripple Delete (Delete or Backspace)
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            let _ = self.state.ripple_delete_selected();
        }

        // Transport Controls: Play/Pause (Space)
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.state.toggle_play();
        }

        // Transport Controls: Shuttle J/K/L & Frame stepping
        if ctx.input(|i| !i.modifiers.command && !i.modifiers.ctrl) {
            if ctx.input(|i| i.key_pressed(egui::Key::J)) {
                self.state.shuttle_backward();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::K)) {
                self.state.shuttle_pause();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::L)) {
                self.state.shuttle_forward();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.state.step_frame(-1);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.state.step_frame(1);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
                self.state.seek(slate_core::time::RationalTime::ZERO);
            }
        }

        // Save Project (Cmd+S / Ctrl+S)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && !i.modifiers.shift && i.key_pressed(egui::Key::S)) {
            self.save_current_project();
        }

        // Save Project As (Cmd+Shift+S)
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.modifiers.shift && i.key_pressed(egui::Key::S)) {
            self.file_browser.open(FileBrowserMode::SaveProjectAs);
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

                    ui.add_space(6.0);

                    // Homescreen / Project Hub Button
                    let home_btn = egui::Button::new(
                        egui::RichText::new("🏠 Projects")
                            .font(FontId::proportional(12.0))
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(37, 99, 235));

                    if ui.add(home_btn).on_hover_text("Open Project Manager (Cmd+Shift+H)").clicked() {
                        self.home_screen.open();
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Menu Bar
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Project Manager (Cmd+Shift+H)").clicked() {
                                self.home_screen.open();
                                ui.close_menu();
                            }
                            if ui.button("New Project... (Cmd+N)").clicked() {
                                self.home_screen.open();
                                self.home_screen.show_new_dialog = true;
                                ui.close_menu();
                            }
                            if ui.button("Open Project... (Cmd+O)").clicked() {
                                self.file_browser.open(FileBrowserMode::OpenProject);
                                ui.close_menu();
                            }

                            // Recent Projects Submenu
                            let recents: Vec<RecentProject> = self.home_screen.recents_store.projects.iter().take(6).cloned().collect();
                            ui.menu_button("Recent Projects", |ui| {
                                if recents.is_empty() {
                                    ui.label("No recent projects");
                                } else {
                                    for p in recents {
                                        let label = format!("{} ({})", p.name, p.formatted_time_ago());
                                        if ui.button(label).clicked() {
                                            self.load_project_path(&p.path);
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });

                            ui.separator();
                            if ui.button("Save Project (Cmd+S)").clicked() {
                                self.save_current_project();
                                ui.close_menu();
                            }
                            if ui.button("Save Project As... (Cmd+Shift+S)").clicked() {
                                self.file_browser.open(FileBrowserMode::SaveProjectAs);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Import Media Files... (Cmd+I)").clicked() {
                                self.file_browser.open(FileBrowserMode::ImportMedia);
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
                            if ui.button("Split at Playhead (Cmd+K / S)").clicked() {
                                let _ = self.state.split_selected_or_at_playhead();
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

        // 4a. Studio Status Bar (At lowest screen edge)
        TopBottomPanel::bottom("bottom_status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    let status_color = if self.state.status_is_error {
                        Color32::from_rgb(239, 68, 68)
                    } else {
                        Color32::from_rgb(16, 185, 129)
                    };

                    ui.label(
                        egui::RichText::new(&self.state.status_text)
                            .font(FontId::proportional(11.5))
                            .color(status_color)
                            .strong(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);

                        ui.label(
                            egui::RichText::new("✂ Blade: C  •  Split: Cmd+K / S  •  Play: Space  •  Del: Ripple")
                                .font(FontId::proportional(11.0))
                                .color(SlateTheme::TEXT_MUTED),
                        );

                        ui.separator();

                        ui.label(
                            egui::RichText::new(format!("TC: {}", Timecode::format(self.state.playhead, false)))
                                .font(FontId::monospace(11.0))
                                .color(SlateTheme::ACCENT_CYAN),
                        );

                        ui.separator();

                        ui.label(
                            egui::RichText::new(format!("{:?}", self.state.active_tool))
                                .font(FontId::monospace(10.5))
                                .color(SlateTheme::ACCENT_AMBER),
                        );
                    });
                });
            });

        // 4b. Multi-track Timeline
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

        // 7. Homescreen / Project Hub Modal
        let mut request_browse = false;
        if let Some(action) = self.home_screen.show(ctx, &mut request_browse) {
            match action {
                HomeScreenAction::OpenProject(path) => {
                    self.load_project_path(&path);
                }
                HomeScreenAction::CreateNewProject { name, width, height, fps } => {
                    let mut proj = Project::new(&name);
                    if let Some(tl) = proj.active_timeline_mut() {
                        tl.settings.width = width;
                        tl.settings.height = height;
                        tl.settings.timebase = fps;
                        tl.tracks.push(slate_core::model::Track::new("V1", TrackType::Video));
                        tl.tracks.push(slate_core::model::Track::new("A1", TrackType::Audio));
                    }
                    let p_path = PathBuf::from(format!("/tmp/slate_projects/{}.slateproj", name.replace(' ', "_")));
                    let _ = ProjectBundle::save_to_bundle(&p_path, &proj);
                    self.current_project_path = Some(p_path);
                    self.state.set_project(proj);
                    self.record_current_project_in_recents();
                }
                HomeScreenAction::OpenSampleProject => {
                    self.create_sample_project();
                }
            }
        }

        if request_browse {
            self.file_browser.open(FileBrowserMode::OpenProject);
        }

        // 8. In-App Filesystem Browser Modal
        if let Some((mode, path)) = self.file_browser.show(ctx) {
            match mode {
                FileBrowserMode::OpenProject => {
                    self.load_project_path(&path);
                }
                FileBrowserMode::SaveProjectAs => {
                    let engine = self.state.engine.lock();
                    if ProjectBundle::save_to_bundle(&path, engine.project()).is_ok() {
                        self.current_project_path = Some(path.clone());
                        drop(engine);
                        self.record_current_project_in_recents();
                        self.state.set_status(format!("✓ Saved bundle to {}", path.display()), false);
                    }
                }
                FileBrowserMode::ImportMedia => {
                    self.import_media_file(&path);
                }
            }
        }
    }
}
