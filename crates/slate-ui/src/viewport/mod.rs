//! Video preview viewport widget and letterboxed canvas.

pub mod controls;
pub mod timecode;

use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Stroke, Ui};
use slate_core::model::TrackItem;
use slate_core::time::RationalTime;
use crate::state::EditorState;
use crate::theme::SlateTheme;

use controls::render_transport_controls;
use timecode::render_timecode_badge;

pub struct ViewportWidget;

impl ViewportWidget {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        let (project_w, project_h, duration, fps) = {
            let engine = state.engine.lock();
            let tl_opt = engine.project().active_timeline();
            let w = tl_opt.map(|t| t.settings.width as f32).unwrap_or(1920.0);
            let h = tl_opt.map(|t| t.settings.height as f32).unwrap_or(1080.0);
            let dur = tl_opt.map(|t| t.duration()).unwrap_or(RationalTime::ZERO);
            let timebase = tl_opt.map(|t| t.settings.timebase).unwrap_or(30);
            (w, h, dur, timebase)
        };

        let aspect_ratio = if project_h > 0.0 {
            project_w / project_h
        } else {
            16.0 / 9.0
        };

        // Fill available space
        let full_size = ui.available_size();
        let (viewport_rect, _resp) = ui.allocate_exact_size(full_size, egui::Sense::hover());
        let painter = ui.painter_at(viewport_rect);

        // Dark studio background
        painter.rect_filled(viewport_rect, 0.0, SlateTheme::BG_DARKEST);

        let top_bar_height = 34.0;
        let controls_height = 46.0;
        let preview_area_rect = Rect::from_min_max(
            pos2(viewport_rect.left(), viewport_rect.top() + top_bar_height),
            pos2(viewport_rect.right(), viewport_rect.bottom() - controls_height),
        );

        // 1. Top Viewport Info Bar
        let top_bar_rect = Rect::from_min_size(viewport_rect.min, vec2(viewport_rect.width(), top_bar_height));
        painter.rect_filled(top_bar_rect, 0.0, SlateTheme::BG_DARK);
        painter.line_segment(
            [top_bar_rect.left_bottom(), top_bar_rect.right_bottom()],
            Stroke::new(1.0, SlateTheme::BORDER),
        );

        let mut top_bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(top_bar_rect));
        top_bar_ui.horizontal(|ui| {
            ui.add_space(10.0);

            // Resolution Tag
            ui.label(
                egui::RichText::new(format!("📺 {}×{} (16:9)", project_w as u32, project_h as u32))
                    .font(FontId::proportional(11.0))
                    .color(SlateTheme::TEXT_SECONDARY),
            );

            ui.separator();

            // FPS Tag
            ui.label(
                egui::RichText::new(format!("{:.2} fps", fps))
                    .font(FontId::monospace(11.0))
                    .color(SlateTheme::ACCENT_CYAN),
            );

            ui.separator();

            // Playback Mode Badge
            let mode_text = if state.is_playing { "● PLAYING" } else { "PAUSED" };
            let mode_color = if state.is_playing { SlateTheme::ACCENT_EMERALD } else { SlateTheme::TEXT_MUTED };
            ui.label(
                egui::RichText::new(mode_text)
                    .font(FontId::monospace(10.0))
                    .color(mode_color)
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Fit (100%)")
                        .font(FontId::proportional(11.0))
                        .color(SlateTheme::TEXT_MUTED),
                );
            });
        });

        // 2. Letterbox Video Canvas
        let avail_w = preview_area_rect.width() - 24.0;
        let avail_h = preview_area_rect.height() - 24.0;

        let (fit_w, fit_h) = if avail_w / avail_h > aspect_ratio {
            (avail_h * aspect_ratio, avail_h)
        } else {
            (avail_w, avail_w / aspect_ratio)
        };

        let fit_x = preview_area_rect.left() + (preview_area_rect.width() - fit_w) / 2.0;
        let fit_y = preview_area_rect.top() + (preview_area_rect.height() - fit_h) / 2.0;
        let canvas_rect = Rect::from_min_size(pos2(fit_x, fit_y), vec2(fit_w, fit_h));

        // Outer glow/shadow border
        painter.rect_filled(canvas_rect, 4.0, Color32::BLACK);
        painter.rect_stroke(canvas_rect, 4.0, Stroke::new(1.5, SlateTheme::BORDER_LIGHT));

        // Center crosshairs (subtle studio markings)
        let cx = canvas_rect.center().x;
        let cy = canvas_rect.center().y;
        let cross_size = 12.0;
        painter.line_segment(
            [pos2(cx - cross_size, cy), pos2(cx + cross_size, cy)],
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 40)),
        );
        painter.line_segment(
            [pos2(cx, cy - cross_size), pos2(cx, cy + cross_size)],
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 40)),
        );

        // Safe area rectangle (90% Action Safe)
        let safe_inset_x = canvas_rect.width() * 0.05;
        let safe_inset_y = canvas_rect.height() * 0.05;
        let safe_rect = Rect::from_min_max(
            pos2(canvas_rect.left() + safe_inset_x, canvas_rect.top() + safe_inset_y),
            pos2(canvas_rect.right() - safe_inset_x, canvas_rect.bottom() - safe_inset_y),
        );
        painter.rect_stroke(
            safe_rect,
            0.0,
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 20)),
        );

        // Center preview title & status
        painter.text(
            pos2(cx, cy - 20.0),
            Align2::CENTER_CENTER,
            "SLATE MONITOR",
            FontId::proportional(16.0),
            Color32::from_rgba_premultiplied(240, 240, 250, 180),
        );
        painter.text(
            pos2(cx, cy + 8.0),
            Align2::CENTER_CENTER,
            format!("{}×{} ProRes / H.264 Ready", project_w as u32, project_h as u32),
            FontId::monospace(11.0),
            SlateTheme::TEXT_MUTED,
        );

        // Active clip indicator in preview
        let current_time = state.playhead;
        let active_clip_name = {
            let engine = state.engine.lock();
            let mut name = None;
            if let Some(tl) = engine.project().active_timeline() {
                for track in &tl.tracks {
                    for item in &track.items {
                        if let TrackItem::Clip(clip) = item {
                            if clip.timeline_range.contains_time(current_time) {
                                name = Some(clip.name.clone());
                                break;
                            }
                        }
                    }
                    if name.is_some() {
                        break;
                    }
                }
            }
            name
        };

        if let Some(name) = active_clip_name {
            let clip_badge_rect = Rect::from_min_size(
                pos2(canvas_rect.left() + 14.0, canvas_rect.bottom() - 32.0),
                vec2(160.0, 22.0),
            );
            painter.rect_filled(clip_badge_rect, 4.0, Color32::from_black_alpha(180));
            painter.text(
                pos2(clip_badge_rect.left() + 8.0, clip_badge_rect.center().y),
                Align2::LEFT_CENTER,
                format!("▶ {}", name),
                FontId::proportional(11.0),
                SlateTheme::ACCENT_CYAN,
            );
        }

        // 3. Bottom Transport Controls Bar
        let controls_rect = Rect::from_min_size(
            pos2(viewport_rect.left(), viewport_rect.bottom() - controls_height),
            vec2(viewport_rect.width(), controls_height),
        );
        painter.rect_filled(controls_rect, 0.0, SlateTheme::BG_DARK);
        painter.line_segment(
            [controls_rect.left_top(), controls_rect.right_top()],
            Stroke::new(1.0, SlateTheme::BORDER),
        );

        let mut controls_ui = ui.new_child(egui::UiBuilder::new().max_rect(controls_rect));
        controls_ui.horizontal(|ui| {
            ui.add_space(12.0);
            render_transport_controls(ui, state);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(16.0);
                render_timecode_badge(ui, current_time, duration, state.dropped_frames);
            });
        });
    }
}
