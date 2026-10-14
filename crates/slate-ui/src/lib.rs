//! Slate UI - High-performance desktop non-linear video editor UI built with egui & eframe.

pub mod agent_panel;
pub mod app;
pub mod audio_monitor;
pub mod export;
pub mod inspector;
pub mod media_pool;
pub mod state;
pub mod theme;
pub mod timeline;
pub mod transcript;
pub mod viewport;

pub use app::SlateApp;
pub use state::EditorState;

use slate_core::model::Project;

/// Launch the desktop Slate Video Editor window with an open project.
pub fn run_desktop_app(project: Project) -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Slate - AI-Native Non-Linear Video Editor")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Slate Video Editor",
        native_options,
        Box::new(|_cc| Ok(Box::new(SlateApp::new_with_project(project)))),
    )
}
