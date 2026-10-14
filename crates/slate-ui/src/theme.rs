//! Theme and visual styling for Slate video editor.

use egui::{Color32, Stroke};

pub struct SlateTheme;

impl SlateTheme {
    // Backgrounds
    pub const BG_DARKEST: Color32 = Color32::from_rgb(18, 19, 22);
    pub const BG_DARK: Color32 = Color32::from_rgb(26, 27, 32);
    pub const BG_PANEL: Color32 = Color32::from_rgb(32, 34, 40);
    pub const BG_TIMELINE: Color32 = Color32::from_rgb(22, 23, 27);
    pub const BG_TRACK: Color32 = Color32::from_rgb(28, 30, 36);
    pub const BG_TRACK_ALT: Color32 = Color32::from_rgb(24, 25, 30);

    // Accents & Brand
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);
    pub const ACCENT_BLUE_LIGHT: Color32 = Color32::from_rgb(96, 165, 250);
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11); // Agent edits & warnings
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(16, 185, 129); // Audio clips
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(139, 92, 246); // Text/titles
    pub const ACCENT_RED: Color32 = Color32::from_rgb(239, 68, 68); // Playhead & recording

    // Clips
    pub const CLIP_VIDEO: Color32 = Color32::from_rgb(45, 85, 155);
    pub const CLIP_VIDEO_BORDER: Color32 = Color32::from_rgb(70, 120, 210);
    pub const CLIP_AUDIO: Color32 = Color32::from_rgb(20, 110, 80);
    pub const CLIP_AUDIO_BORDER: Color32 = Color32::from_rgb(34, 160, 115);
    pub const CLIP_TITLE: Color32 = Color32::from_rgb(100, 60, 150);
    pub const CLIP_TITLE_BORDER: Color32 = Color32::from_rgb(140, 90, 210);
    pub const CLIP_SELECTED_BORDER: Color32 = Color32::from_rgb(255, 255, 255);
    pub const CLIP_AGENT_BORDER: Color32 = Color32::from_rgb(251, 191, 36);

    // UI elements
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 244, 246);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(156, 163, 175);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(107, 114, 128);
    pub const BORDER: Color32 = Color32::from_rgb(45, 48, 58);
    pub const PLAYHEAD_LINE: Color32 = Color32::from_rgb(239, 68, 68);
    pub const SNAP_LINE: Color32 = Color32::from_rgb(255, 255, 255);

    pub fn apply(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Self::BG_PANEL;
        visuals.window_fill = Self::BG_DARK;
        visuals.override_text_color = Some(Self::TEXT_PRIMARY);
        visuals.widgets.noninteractive.bg_fill = Self::BG_DARK;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        visuals.widgets.inactive.bg_fill = Self::BG_PANEL;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 52, 64);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::ACCENT_BLUE_LIGHT);
        visuals.widgets.active.bg_fill = Self::ACCENT_BLUE;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::WHITE);
        ctx.set_visuals(visuals);
    }
}
