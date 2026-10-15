//! In-app filesystem browser for opening, saving, and importing projects & media.

use egui::{vec2, Color32, FontId, ScrollArea};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileBrowserMode {
    OpenProject,
    SaveProjectAs,
    ImportMedia,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_bundle: bool,
    pub size_bytes: u64,
}

pub struct FileBrowserModal {
    pub is_open: bool,
    pub mode: FileBrowserMode,
    pub current_dir: PathBuf,
    pub selected_path: Option<PathBuf>,
    pub save_as_name: String,
    pub entries: Vec<FileEntry>,
    pub error_msg: Option<String>,
}

impl Default for FileBrowserModal {
    fn default() -> Self {
        let initial_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut browser = Self {
            is_open: false,
            mode: FileBrowserMode::OpenProject,
            current_dir: initial_dir,
            selected_path: None,
            save_as_name: "My_New_Project.slateproj".to_string(),
            entries: Vec::new(),
            error_msg: None,
        };
        browser.refresh_entries();
        browser
    }
}

impl FileBrowserModal {
    pub fn open(&mut self, mode: FileBrowserMode) {
        self.mode = mode;
        self.is_open = true;
        self.selected_path = None;
        self.error_msg = None;
        if self.current_dir.as_os_str().is_empty() {
            self.current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        }
        self.refresh_entries();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.selected_path = None;
    }

    pub fn navigate_to(&mut self, dir: PathBuf) {
        if dir.is_dir() {
            self.current_dir = dir;
            self.selected_path = None;
            self.error_msg = None;
            self.refresh_entries();
        }
    }

    pub fn refresh_entries(&mut self) {
        self.entries.clear();
        match fs::read_dir(&self.current_dir) {
            Ok(read_dir) => {
                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in read_dir.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Hide hidden dotfiles by default
                    if name.starts_with('.') && name != ".slateproj" {
                        continue;
                    }

                    let is_dir = path.is_dir();
                    let is_bundle = is_dir && (name.ends_with(".slateproj") || path.join("project.json").exists());
                    let meta = entry.metadata().ok();
                    let size_bytes = meta.map(|m| m.len()).unwrap_or(0);

                    let item = FileEntry {
                        name: name.clone(),
                        path: path.clone(),
                        is_dir,
                        is_bundle,
                        size_bytes,
                    };

                    match self.mode {
                        FileBrowserMode::OpenProject => {
                            if is_bundle || (is_dir && !name.ends_with(".slateproj")) {
                                dirs.push(item);
                            } else if name.ends_with(".json") || name.ends_with(".slateproj") {
                                files.push(item);
                            }
                        }
                        FileBrowserMode::SaveProjectAs => {
                            if is_dir {
                                dirs.push(item);
                            }
                        }
                        FileBrowserMode::ImportMedia => {
                            if is_dir {
                                dirs.push(item);
                            } else {
                                let lower = name.to_lowercase();
                                if lower.ends_with(".mp4")
                                    || lower.ends_with(".mov")
                                    || lower.ends_with(".mkv")
                                    || lower.ends_with(".wav")
                                    || lower.ends_with(".mp3")
                                    || lower.ends_with(".aac")
                                    || lower.ends_with(".png")
                                    || lower.ends_with(".jpg")
                                    || lower.ends_with(".jpeg")
                                {
                                    files.push(item);
                                }
                            }
                        }
                    }
                }

                dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                self.entries.extend(dirs);
                self.entries.extend(files);
            }
            Err(e) => {
                self.error_msg = Some(format!("Could not read directory: {}", e));
            }
        }
    }

    /// Renders the modal window. Returns Some(action_path) if user confirmed an action.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<(FileBrowserMode, PathBuf)> {
        if !self.is_open {
            return None;
        }

        let mut confirmed_action = None;
        let mut should_close = false;

        let title = match self.mode {
            FileBrowserMode::OpenProject => "📂 Open Slate Project",
            FileBrowserMode::SaveProjectAs => "💾 Save Project As Bundle",
            FileBrowserMode::ImportMedia => "🎬 Import Media Files",
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(760.0, 520.0))
            .min_size(vec2(600.0, 400.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                // Top Bar: Navigation & Breadcrumbs
                ui.horizontal(|ui| {
                    if ui.button("⬆ Up").on_hover_text("Go to parent directory").clicked() {
                        if let Some(parent) = self.current_dir.parent() {
                            self.navigate_to(parent.to_path_buf());
                        }
                    }

                    if ui.button("⟳ Refresh").clicked() {
                        self.refresh_entries();
                    }

                    ui.separator();

                    // Breadcrumb segments
                    ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut accum = PathBuf::new();
                            for component in self.current_dir.components() {
                                accum.push(component);
                                let name = component.as_os_str().to_string_lossy();
                                let label = if name == "/" { "root" } else { &name };
                                if ui.button(label).clicked() {
                                    self.navigate_to(accum.clone());
                                    break;
                                }
                                ui.label("/");
                            }
                        });
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Body: Bookmarks Sidebar + File Explorer List
                let total_h = ui.available_height() - 50.0;
                ui.horizontal(|ui| {
                    // Left Bookmarks Sidebar
                    ui.allocate_ui_with_layout(
                        vec2(160.0, total_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(egui::RichText::new("FAVORITES").font(FontId::proportional(11.0)).strong().color(Color32::from_rgb(140, 150, 170)));
                            ui.add_space(4.0);

                            if let Ok(home) = std::env::var("HOME") {
                                let home_path = PathBuf::from(home);
                                if ui.selectable_label(self.current_dir == home_path, "🏠 Home").clicked() {
                                    self.navigate_to(home_path.clone());
                                }
                                let desktop = home_path.join("Desktop");
                                if desktop.exists() && ui.selectable_label(self.current_dir == desktop, "🖥 Desktop").clicked() {
                                    self.navigate_to(desktop);
                                }
                                let docs = home_path.join("Documents");
                                if docs.exists() && ui.selectable_label(self.current_dir == docs, "📁 Documents").clicked() {
                                    self.navigate_to(docs);
                                }
                                let downloads = home_path.join("Downloads");
                                if downloads.exists() && ui.selectable_label(self.current_dir == downloads, "⬇ Downloads").clicked() {
                                    self.navigate_to(downloads);
                                }
                                let movies = home_path.join("Movies");
                                if movies.exists() && ui.selectable_label(self.current_dir == movies, "🎬 Movies").clicked() {
                                    self.navigate_to(movies);
                                }
                            }

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("WORKSPACE").font(FontId::proportional(11.0)).strong().color(Color32::from_rgb(140, 150, 170)));
                            ui.add_space(4.0);

                            if let Ok(cwd) = std::env::current_dir() {
                                if ui.selectable_label(self.current_dir == cwd, "📦 Current Workdir").clicked() {
                                    self.navigate_to(cwd);
                                }
                            }
                            let tmp_samples = PathBuf::from("/tmp/slate_samples");
                            if tmp_samples.exists() && ui.selectable_label(self.current_dir == tmp_samples, "🧪 Sample Projects").clicked() {
                                self.navigate_to(tmp_samples);
                            }
                        },
                    );

                    ui.separator();

                    // Main Files List
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), total_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            if let Some(err) = &self.error_msg {
                                ui.colored_label(Color32::from_rgb(239, 68, 68), err);
                            }

                            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                                if self.entries.is_empty() {
                                    ui.add_space(20.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new("Folder is empty").color(Color32::from_rgb(140, 150, 160)));
                                    });
                                }

                                for entry in &self.entries {
                                    let is_selected = self.selected_path.as_ref() == Some(&entry.path);

                                    let icon = if entry.is_bundle {
                                        "📽"
                                    } else if entry.is_dir {
                                        "📁"
                                    } else if entry.name.ends_with(".mp4") || entry.name.ends_with(".mov") {
                                        "🎬"
                                    } else if entry.name.ends_with(".wav") || entry.name.ends_with(".mp3") {
                                        "🎵"
                                    } else if entry.name.ends_with(".png") || entry.name.ends_with(".jpg") {
                                        "🖼"
                                    } else {
                                        "📄"
                                    };

                                    let label_text = format!("{}  {}", icon, entry.name);
                                    let resp = ui.selectable_label(is_selected, label_text);

                                    if resp.clicked() {
                                        self.selected_path = Some(entry.path.clone());
                                        if self.mode == FileBrowserMode::SaveProjectAs && !entry.is_dir {
                                            self.save_as_name = entry.name.clone();
                                        }
                                    }

                                    // Double click: open folder or open project bundle
                                    if resp.double_clicked() {
                                        if entry.is_bundle && self.mode == FileBrowserMode::OpenProject {
                                            confirmed_action = Some((self.mode, entry.path.clone()));
                                            should_close = true;
                                        } else if entry.is_dir {
                                            self.navigate_to(entry.path.clone());
                                            break;
                                        } else {
                                            confirmed_action = Some((self.mode, entry.path.clone()));
                                            should_close = true;
                                        }
                                    }
                                }
                            });
                        },
                    );
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Bottom Action Row
                ui.horizontal(|ui| {
                    match self.mode {
                        FileBrowserMode::OpenProject => {
                            if let Some(sel) = &self.selected_path {
                                ui.label(format!("Selected: {}", sel.file_name().unwrap_or_default().to_string_lossy()));
                            } else {
                                ui.label(egui::RichText::new("Choose a .slateproj bundle or project.json").color(Color32::from_rgb(140, 150, 160)));
                            }
                        }
                        FileBrowserMode::SaveProjectAs => {
                            ui.label("Bundle Name:");
                            ui.text_edit_singleline(&mut self.save_as_name);
                        }
                        FileBrowserMode::ImportMedia => {
                            if let Some(sel) = &self.selected_path {
                                ui.label(format!("Media: {}", sel.file_name().unwrap_or_default().to_string_lossy()));
                            } else {
                                ui.label(egui::RichText::new("Choose audio or video file").color(Color32::from_rgb(140, 150, 160)));
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let confirm_label = match self.mode {
                            FileBrowserMode::OpenProject => "Open Project",
                            FileBrowserMode::SaveProjectAs => "Save Bundle",
                            FileBrowserMode::ImportMedia => "Import Media",
                        };

                        let can_confirm = match self.mode {
                            FileBrowserMode::OpenProject => self.selected_path.is_some(),
                            FileBrowserMode::SaveProjectAs => !self.save_as_name.trim().is_empty(),
                            FileBrowserMode::ImportMedia => self.selected_path.is_some(),
                        };

                        let confirm_btn = egui::Button::new(
                            egui::RichText::new(confirm_label).strong().color(Color32::WHITE),
                        )
                        .fill(if can_confirm { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(60, 65, 80) });

                        if ui.add_enabled(can_confirm, confirm_btn).clicked() {
                            let action_path = match self.mode {
                                FileBrowserMode::SaveProjectAs => {
                                    let mut p = self.current_dir.join(self.save_as_name.trim());
                                    if !p.to_string_lossy().ends_with(".slateproj") {
                                        p = PathBuf::from(format!("{}.slateproj", p.display()));
                                    }
                                    p
                                }
                                _ => self.selected_path.clone().unwrap_or_default(),
                            };
                            confirmed_action = Some((self.mode, action_path));
                            should_close = true;
                        }

                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.close();
        }

        confirmed_action
    }
}
