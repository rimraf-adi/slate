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
        let (project_w, project_h, duration) = {
            let engine = state.engine.lock();
            let tl_opt = engine.project().active_timeline();
            let w = tl_opt.map(|t| t.settings.width as f32).unwrap_or(1920.0);
            let h = tl_opt.map(|t| t.settings.height as f32).unwrap_or(1080.0);
            let dur = tl_opt.map(|t| t.duration()).unwrap_or(RationalTime::ZERO);
            (w, h, dur)
        };

        let aspect_ratio = if project_h > 0.0 {
            project_w / project_h
        } else {
            16.0 / 9.0
        };

        // Available area for viewport
        let avail_rect = ui.available_rect_before_wrap();
        let controls_height = 42.0;
        let preview_area_height = (avail_rect.height() - controls_height).max(100.0);

        // Letterbox calculation
        let max_w = avail_rect.width();
        let max_h = preview_area_height;

        let (fit_w, fit_h) = if max_w / max_h > aspect_ratio {
            (max_h * aspect_ratio, max_h)
        } else {
            (max_w, max_w / aspect_ratio)
        };

        let fit_x = avail_rect.left() + (max_w - fit_w) / 2.0;
        let fit_y = avail_rect.top() + (max_h - fit_h) / 2.0;
        let canvas_rect = Rect::from_min_size(pos2(fit_x, fit_y), vec2(fit_w, fit_h));

        let painter = ui.painter();

        // Viewport background
        painter.rect_filled(avail_rect, 0.0, SlateTheme::BG_DARKEST);

        // Letterboxed video canvas
        painter.rect_filled(canvas_rect, 0.0, Color32::BLACK);
        painter.rect_stroke(canvas_rect, 0.0, Stroke::new(1.0, SlateTheme::BORDER));

        // Center preview placeholder or rendered texture
        painter.text(
            canvas_rect.center(),
            Align2::CENTER_CENTER,
            format!("Preview Target ({}x{})", project_w as u32, project_h as u32),
            FontId::proportional(14.0),
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
            painter.text(
                pos2(canvas_rect.left() + 12.0, canvas_rect.bottom() - 20.0),
                Align2::LEFT_BOTTOM,
                format!("Clip: {}", name),
                FontId::proportional(12.0),
                SlateTheme::TEXT_SECONDARY,
            );
        }

        // Bottom Controls Bar
        let controls_rect = Rect::from_min_size(
            pos2(avail_rect.left(), avail_rect.top() + preview_area_height),
            vec2(avail_rect.width(), controls_height),
        );
        let mut controls_ui = ui.new_child(egui::UiBuilder::new().max_rect(controls_rect));
        controls_ui.horizontal(|ui| {
            ui.add_space(8.0);
            render_transport_controls(ui, state);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                render_timecode_badge(ui, current_time, duration, state.dropped_frames);
            });
        });
    }
}
