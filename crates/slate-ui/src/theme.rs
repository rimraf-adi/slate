//! Theme and visual styling for Slate video editor.
//! Crafted with modern high-end studio aesthetics (DaVinci / Premiere style dark neutral tones).

use egui::{Color32, Stroke};

pub struct SlateTheme;

impl SlateTheme {
    // Backgrounds & Surfaces
    pub const BG_APP: Color32 = Color32::from_rgb(14, 15, 18);          // Deepest canvas
    pub const BG_DARKEST: Color32 = Color32::from_rgb(17, 18, 22);      // Viewport / timeline bed
    pub const BG_DARK: Color32 = Color32::from_rgb(22, 23, 28);         // Background beneath panels
    pub const BG_PANEL: Color32 = Color32::from_rgb(28, 29, 35);        // Sidebars & toolbar
    pub const BG_HEADER: Color32 = Color32::from_rgb(32, 33, 40);       // Window header & tab strip
    pub const BG_CARD: Color32 = Color32::from_rgb(36, 38, 46);         // Floating cards / clips / inputs
    pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(44, 46, 56);   // Hovered card
    pub const BG_TIMELINE: Color32 = Color32::from_rgb(18, 19, 23);     // Timeline ruler & lanes
    pub const BG_TRACK: Color32 = Color32::from_rgb(24, 25, 31);        // Track lane
    pub const BG_TRACK_ALT: Color32 = Color32::from_rgb(21, 22, 27);    // Track lane alt

    // Accents & Studio Palette
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);        // Primary brand accent
    pub const ACCENT_BLUE_LIGHT: Color32 = Color32::from_rgb(96, 165, 250);  // Active selection
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(6, 182, 212);         // Video tracks / timecode
    pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(16, 185, 129);     // Audio tracks / success
    pub const ACCENT_GREEN: Color32 = Self::ACCENT_EMERALD;
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);       // AI Agent presence / warning
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);      // Titles / Captions
    pub const ACCENT_RED: Color32 = Color32::from_rgb(239, 68, 68);          // Playhead / Recording / Clip peak

    // Clips Styling
    pub const CLIP_VIDEO: Color32 = Color32::from_rgb(30, 58, 110);
    pub const CLIP_VIDEO_BORDER: Color32 = Color32::from_rgb(59, 130, 246);
    pub const CLIP_AUDIO: Color32 = Color32::from_rgb(16, 75, 55);
    pub const CLIP_AUDIO_BORDER: Color32 = Color32::from_rgb(16, 185, 129);
    pub const CLIP_TITLE: Color32 = Color32::from_rgb(76, 29, 115);
    pub const CLIP_TITLE_BORDER: Color32 = Color32::from_rgb(168, 85, 247);
    pub const CLIP_SELECTED_BORDER: Color32 = Color32::from_rgb(255, 255, 255);
    pub const CLIP_AGENT_BORDER: Color32 = Color32::from_rgb(251, 191, 36);

    // Typography & Dividers
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(248, 250, 252);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 166, 178);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 106, 120);
    pub const BORDER: Color32 = Color32::from_rgb(42, 45, 56);
    pub const BORDER_LIGHT: Color32 = Color32::from_rgb(60, 64, 78);
    pub const PLAYHEAD_LINE: Color32 = Color32::from_rgb(239, 68, 68);
    pub const SNAP_LINE: Color32 = Color32::from_rgb(255, 255, 255);

    pub fn apply(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Self::BG_PANEL;
        visuals.window_fill = Self::BG_DARK;
        visuals.override_text_color = Some(Self::TEXT_PRIMARY);

        // Buttons & interactive widgets
        visuals.widgets.noninteractive.bg_fill = Self::BG_DARK;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(5.0);

        visuals.widgets.inactive.bg_fill = Self::BG_CARD;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        visuals.widgets.inactive.rounding = egui::Rounding::same(5.0);

        visuals.widgets.hovered.bg_fill = Self::BG_CARD_HOVER;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::ACCENT_BLUE_LIGHT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.hovered.rounding = egui::Rounding::same(5.0);

        visuals.widgets.active.bg_fill = Self::ACCENT_BLUE;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.active.rounding = egui::Rounding::same(5.0);

        visuals.selection.bg_fill = Self::ACCENT_BLUE;
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

        ctx.set_visuals(visuals);
    }
}
