//! Project Manager & Welcome Screen for Slate Video Editor.

pub mod recents;

use egui::{vec2, Color32, FontId, ScrollArea, Stroke};
use std::path::PathBuf;

use crate::homescreen::recents::{RecentProject, RecentsStore};
use crate::theme::SlateTheme;

pub enum HomeScreenAction {
    OpenProject(PathBuf),
    CreateNewProject {
        name: String,
        width: u32,
        height: u32,
        fps: u32,
    },
    OpenSampleProject,
}

pub struct HomeScreenModal {
    pub is_open: bool,
    pub search_query: String,
    pub recents_store: RecentsStore,
    pub show_new_dialog: bool,
    pub new_name: String,
    pub new_preset_idx: usize,
}

impl Default for HomeScreenModal {
    fn default() -> Self {
        Self {
            is_open: false,
            search_query: String::new(),
            recents_store: RecentsStore::load(),
            show_new_dialog: false,
            new_name: "Untitled Studio Project".to_string(),
            new_preset_idx: 0,
        }
    }
}

impl HomeScreenModal {
    pub fn open(&mut self) {
        self.is_open = true;
        self.recents_store = RecentsStore::load();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.show_new_dialog = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        on_request_browse: &mut bool,
    ) -> Option<HomeScreenAction> {
        if !self.is_open {
            return None;
        }

        let mut chosen_action = None;
        let mut should_close = false;

        egui::Window::new("Project Manager")
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(880.0, 580.0))
            .min_size(vec2(720.0, 480.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                // 1. Header Bar: Brand Identity + Search + Close
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("● SLATE")
                            .font(FontId::proportional(18.0))
                            .color(SlateTheme::ACCENT_CYAN)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("PROJECT MANAGER")
                            .font(FontId::proportional(14.0))
                            .color(Color32::from_rgb(160, 170, 190))
                            .strong(),
                    );

                    ui.add_space(20.0);

                    // Search input
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("🔍 Search recent projects...")
                            .desired_width(260.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕ Close").clicked() {
                            should_close = true;
                        }
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // 2. Hero Action Buttons Row
                ui.horizontal(|ui| {
                    let new_btn = egui::Button::new(
                        egui::RichText::new("🎬 New Project")
                            .font(FontId::proportional(13.0))
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(37, 99, 235))
                    .min_size(vec2(140.0, 32.0));

                    if ui.add(new_btn).clicked() {
                        self.show_new_dialog = !self.show_new_dialog;
                    }

                    let open_btn = egui::Button::new(
                        egui::RichText::new("📂 Open Project...")
                            .font(FontId::proportional(13.0))
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(45, 52, 68))
                    .min_size(vec2(140.0, 32.0));

                    if ui.add(open_btn).clicked() {
                        *on_request_browse = true;
                    }

                    let sample_btn = egui::Button::new(
                        egui::RichText::new("🧪 Load Sample Project")
                            .font(FontId::proportional(13.0))
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(30, 41, 59))
                    .min_size(vec2(160.0, 32.0));

                    if ui.add(sample_btn).clicked() {
                        chosen_action = Some(HomeScreenAction::OpenSampleProject);
                        should_close = true;
                    }
                });

                // Inline New Project Form
                if self.show_new_dialog {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(22, 27, 34))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(56, 68, 88)))
                        .rounding(6.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Project Name:").strong());
                                ui.add(egui::TextEdit::singleline(&mut self.new_name).desired_width(220.0));

                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("Format:").strong());

                                let presets = [
                                    ("YouTube 1080p (16:9)", 1920, 1080, 30),
                                    ("Ultra HD 4K (16:9)", 3840, 2160, 24),
                                    ("TikTok / Reels (9:16)", 1080, 1920, 60),
                                    ("Instagram Square (1:1)", 1080, 1080, 30),
                                ];

                                egui::ComboBox::from_id_salt("preset_picker")
                                    .selected_text(presets[self.new_preset_idx].0)
                                    .show_ui(ui, |ui| {
                                        for (i, p) in presets.iter().enumerate() {
                                            ui.selectable_value(&mut self.new_preset_idx, i, p.0);
                                        }
                                    });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Create Project").clicked() {
                                        let sel = presets[self.new_preset_idx];
                                        chosen_action = Some(HomeScreenAction::CreateNewProject {
                                            name: self.new_name.clone(),
                                            width: sel.1,
                                            height: sel.2,
                                            fps: sel.3,
                                        });
                                        should_close = true;
                                    }
                                });
                            });
                        });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // 3. Recent Projects Grid / List
                ui.label(
                    egui::RichText::new("RECENT PROJECTS")
                        .font(FontId::proportional(12.0))
                        .color(Color32::from_rgb(140, 150, 170))
                        .strong(),
                );
                ui.add_space(6.0);

                let filtered_projects: Vec<RecentProject> = self
                    .recents_store
                    .projects
                    .iter()
                    .filter(|p| {
                        if self.search_query.trim().is_empty() {
                            true
                        } else {
                            let q = self.search_query.to_lowercase();
                            p.name.to_lowercase().contains(&q)
                                || p.path.to_string_lossy().to_lowercase().contains(&q)
                        }
                    })
                    .cloned()
                    .collect();

                let mut remove_path = None;

                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if filtered_projects.is_empty() {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No matching recent projects found").color(Color32::from_rgb(130, 140, 155)));
                        });
                    }

                    for project in filtered_projects {
                        let is_vertical = project.resolution[0] < project.resolution[1];
                        let is_4k = project.resolution[0] >= 3840;

                        let badge_text = if is_vertical {
                            "9:16 SHORTS"
                        } else if is_4k {
                            "4K UHD"
                        } else {
                            "1080p HD"
                        };

                        let badge_color = if is_vertical {
                            Color32::from_rgb(236, 72, 153)
                        } else if is_4k {
                            Color32::from_rgb(245, 158, 11)
                        } else {
                            Color32::from_rgb(59, 130, 246)
                        };

                        egui::Frame::none()
                            .fill(Color32::from_rgb(20, 24, 33))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(40, 48, 64)))
                            .rounding(6.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Visual project badge
                                    let (rect, _) = ui.allocate_exact_size(vec2(70.0, 48.0), egui::Sense::hover());
                                    let painter = ui.painter_at(rect);
                                    painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 15, 22));
                                    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, badge_color));
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        badge_text,
                                        FontId::monospace(9.5),
                                        badge_color,
                                    );

                                    ui.add_space(10.0);

                                    // Project details
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&project.name)
                                                    .font(FontId::proportional(14.0))
                                                    .color(Color32::WHITE)
                                                    .strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("• {}", project.formatted_time_ago()))
                                                    .font(FontId::proportional(11.5))
                                                    .color(Color32::from_rgb(130, 140, 160)),
                                            );
                                        });

                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}x{} @ {:.0}fps  |  Duration: {}  |  {} tracks",
                                                    project.resolution[0],
                                                    project.resolution[1],
                                                    project.fps,
                                                    project.formatted_duration(),
                                                    project.track_count
                                                ))
                                                .font(FontId::proportional(11.0))
                                                .color(Color32::from_rgb(140, 150, 170)),
                                            );
                                        });

                                        ui.label(
                                            egui::RichText::new(project.path.to_string_lossy())
                                                .font(FontId::monospace(9.5))
                                                .color(Color32::from_rgb(90, 100, 120)),
                                        );
                                    });

                                    // Action buttons
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("🗑").on_hover_text("Remove from recent list").clicked() {
                                            remove_path = Some(project.path.clone());
                                        }

                                        let open_btn = egui::Button::new(
                                            egui::RichText::new("▶ Open")
                                                .font(FontId::proportional(12.0))
                                                .strong()
                                                .color(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgb(37, 99, 235))
                                        .min_size(vec2(72.0, 26.0));

                                        if ui.add(open_btn).clicked() {
                                            chosen_action = Some(HomeScreenAction::OpenProject(project.path.clone()));
                                            should_close = true;
                                        }
                                    });
                                });
                            });

                        ui.add_space(6.0);
                    }
                });

                if let Some(path) = remove_path {
                    self.recents_store.remove(&path);
                }
            });

        if should_close {
            self.close();
        }

        chosen_action
    }
}
