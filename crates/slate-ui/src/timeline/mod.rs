//! Custom egui timeline painter and interaction surface.

pub mod clip_view;
pub mod ruler;
pub mod snap;
pub mod tools;
pub mod track_view;

use egui::{pos2, vec2, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Stroke, Ui};
use slate_core::id::{ClipId, TrackId};
use slate_core::model::{TrackItem, TrackType};
use slate_core::op::Op;
use slate_core::time::RationalTime;
use slate_core::OpSource;
use crate::state::{EditorState, ToolMode};
use crate::theme::SlateTheme;

use clip_view::{draw_clip, ClipGeometry, ClipHitArea};
use ruler::{draw_ruler, RULER_HEIGHT};
use snap::find_snap_target;
use track_view::{draw_track_header, DEFAULT_TRACK_HEIGHT, TRACK_HEADER_WIDTH};

const TIMELINE_TOOLBAR_HEIGHT: f32 = 32.0;

pub struct TimelineWidget;

impl TimelineWidget {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        let (timeline_rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), ui.available_height().max(200.0)),
            Sense::click_and_drag(),
        );

        let painter = ui.painter_at(timeline_rect);

        // Timeline Bed Background
        painter.rect_filled(timeline_rect, 0.0, SlateTheme::BG_TIMELINE);

        let (timebase, total_duration_secs, tracks_data) = {
            let engine = state.engine.lock();
            let tl_opt = engine.project().active_timeline();
            let tb = tl_opt.map(|t| t.settings.timebase).unwrap_or(30);
            let dur = tl_opt.map(|t| t.duration().to_seconds()).unwrap_or(0.0);
            let tracks = tl_opt.map(|t| t.tracks.clone()).unwrap_or_default();
            (tb, dur, tracks)
        };

        // 1. Timeline Mini-Toolbar
        let toolbar_rect = Rect::from_min_size(
            timeline_rect.min,
            vec2(timeline_rect.width(), TIMELINE_TOOLBAR_HEIGHT),
        );
        painter.rect_filled(toolbar_rect, 0.0, SlateTheme::BG_DARK);
        painter.line_segment(
            [toolbar_rect.left_bottom(), toolbar_rect.right_bottom()],
            Stroke::new(1.0, SlateTheme::BORDER),
        );

        let mut toolbar_ui = ui.new_child(egui::UiBuilder::new().max_rect(toolbar_rect));
        toolbar_ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Add Track buttons
            if ui.button("➕ Video").on_hover_text("Add a new Video Track (V)").clicked() {
                let track_count = tracks_data.iter().filter(|t| t.track_type == TrackType::Video).count() + 1;
                let op = Op::AddTrack {
                    name: format!("V{}", track_count),
                    track_type: TrackType::Video,
                    index: None,
                };
                let _ = state.apply_op(op, OpSource::Human);
            }

            if ui.button("➕ Audio").on_hover_text("Add a new Audio Track (A)").clicked() {
                let track_count = tracks_data.iter().filter(|t| t.track_type == TrackType::Audio).count() + 1;
                let op = Op::AddTrack {
                    name: format!("A{}", track_count),
                    track_type: TrackType::Audio,
                    index: None,
                };
                let _ = state.apply_op(op, OpSource::Human);
            }

            ui.separator();

            // Quick Timeline Edit Actions
            if ui.button("✂ Split").on_hover_text("Split clip at current playhead (Cmd+K / C)").clicked() {
                let _ = state.split_selected_or_at_playhead();
            }

            if ui.button("🗑 Delete").on_hover_text("Ripple delete selected clips (Del)").clicked() {
                let _ = state.ripple_delete_selected();
            }

            ui.separator();

            // Magnetic snap toggle
            let snap_text = if state.snap_enabled { "🧲 Snapping ON (N)" } else { "🧲 Snapping OFF (N)" };
            if ui.selectable_label(state.snap_enabled, snap_text).clicked()
                || ui.input(|i| i.key_pressed(egui::Key::N))
            {
                state.snap_enabled = !state.snap_enabled;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);

                // Zoom Fit Button
                if ui.button("Fit").on_hover_text("Fit timeline to window width").clicked() {
                    let total_dur = total_duration_secs.max(10.0) as f32;
                    let avail_w = timeline_rect.width() - TRACK_HEADER_WIDTH - 40.0;
                    state.zoom_px_per_sec = (avail_w / total_dur).clamp(10.0, 300.0);
                    state.scroll_x = 0.0;
                }

                // Zoom In / Out
                if ui.button("➕").on_hover_text("Zoom In (Cmd+=)").clicked() {
                    state.zoom_px_per_sec = (state.zoom_px_per_sec * 1.25).min(500.0);
                }

                // Zoom Slider
                ui.add(
                    egui::Slider::new(&mut state.zoom_px_per_sec, 5.0..=400.0)
                        .show_value(false)
                        .text("Zoom"),
                );

                if ui.button("➖").on_hover_text("Zoom Out (Cmd+-)").clicked() {
                    state.zoom_px_per_sec = (state.zoom_px_per_sec * 0.8).max(5.0);
                }
            });
        });

        // 2. Draw Ruler
        let ruler_y = timeline_rect.top() + TIMELINE_TOOLBAR_HEIGHT;
        let ruler_rect = Rect::from_min_size(
            pos2(timeline_rect.left() + TRACK_HEADER_WIDTH, ruler_y),
            vec2(timeline_rect.width() - TRACK_HEADER_WIDTH, RULER_HEIGHT),
        );
        draw_ruler(
            &painter,
            ruler_rect,
            timebase,
            state.zoom_px_per_sec,
            state.scroll_x,
            state.playhead,
            total_duration_secs,
        );

        // Top-left corner box (above track headers, beneath toolbar)
        let corner_rect = Rect::from_min_size(
            pos2(timeline_rect.left(), ruler_y),
            vec2(TRACK_HEADER_WIDTH, RULER_HEIGHT),
        );
        painter.rect_filled(corner_rect, 0.0, SlateTheme::BG_DARK);
        painter.line_segment([corner_rect.right_top(), corner_rect.right_bottom()], Stroke::new(1.0, SlateTheme::BORDER));
        painter.line_segment([corner_rect.left_bottom(), corner_rect.right_bottom()], Stroke::new(1.0, SlateTheme::BORDER));
        painter.text(
            corner_rect.center(),
            Align2::CENTER_CENTER,
            "TRACKS",
            FontId::monospace(10.0),
            SlateTheme::TEXT_MUTED,
        );

        // 3. Draw Tracks & Clips
        let mut current_y = ruler_y + RULER_HEIGHT;
        let mut clicked_clip: Option<(TrackId, ClipId, ClipHitArea, Pos2)> = None;

        for track in &tracks_data {
            let track_h = DEFAULT_TRACK_HEIGHT;
            let header_rect = Rect::from_min_size(
                pos2(timeline_rect.left(), current_y),
                vec2(TRACK_HEADER_WIDTH, track_h),
            );
            let is_active_track = state.active_track_id == Some(track.id);
            draw_track_header(&painter, header_rect, track, is_active_track);

            // Track lane background
            let lane_rect = Rect::from_min_size(
                pos2(timeline_rect.left() + TRACK_HEADER_WIDTH, current_y),
                vec2(timeline_rect.width() - TRACK_HEADER_WIDTH, track_h),
            );
            painter.rect_filled(lane_rect, 0.0, SlateTheme::BG_TRACK_ALT);
            painter.line_segment([lane_rect.left_bottom(), lane_rect.right_bottom()], Stroke::new(1.0, SlateTheme::BORDER));

            // Subtle vertical second grid lines in track lane
            let sec_interval = 5.0;
            let start_sec = (state.scroll_x / state.zoom_px_per_sec).floor() * sec_interval;
            let end_sec = start_sec + (lane_rect.width() / state.zoom_px_per_sec);
            let mut g_sec = start_sec;
            while g_sec <= end_sec {
                let gx = lane_rect.left() + (g_sec * state.zoom_px_per_sec) - state.scroll_x;
                if gx >= lane_rect.left() && gx <= lane_rect.right() {
                    painter.line_segment(
                        [pos2(gx, lane_rect.top()), pos2(gx, lane_rect.bottom())],
                        Stroke::new(0.5, Color32::from_rgb(32, 34, 42)),
                    );
                }
                g_sec += sec_interval;
            }

            // Clips
            for item in &track.items {
                if let TrackItem::Clip(clip) = item {
                    let geom = ClipGeometry::new(
                        clip,
                        current_y,
                        track_h,
                        timeline_rect.left() + TRACK_HEADER_WIDTH,
                        state.zoom_px_per_sec,
                        state.scroll_x,
                    );

                    let is_selected = state.selected_clip_ids.contains(&clip.id);
                    let is_agent_active = state.agent_presence.as_ref().is_some_and(|p| {
                        p.active_range.is_some_and(|r| r.overlaps(&clip.timeline_range))
                    });

                    draw_clip(&painter, &geom, clip, track.track_type, is_selected, is_agent_active);

                    // Hit testing if mouse clicked inside lane
                    if let Some(hover_pos) = response.hover_pos() {
                        if let Some(area) = geom.hit_test(hover_pos) {
                            if response.clicked() {
                                clicked_clip = Some((track.id, clip.id, area, hover_pos));
                            }
                        }
                    }
                }
            }

            current_y += track_h;
        }

        // Fill remaining empty timeline space with subtle dark lane bed
        if current_y < timeline_rect.bottom() {
            let empty_lane_rect = Rect::from_min_max(
                pos2(timeline_rect.left(), current_y),
                timeline_rect.right_bottom(),
            );
            painter.rect_filled(empty_lane_rect, 0.0, SlateTheme::BG_TIMELINE);
        }

        // 4. Draw Agent Presence Range Indicator (if active)
        if let Some(ref presence) = state.agent_presence {
            if let Some(range) = presence.active_range {
                let start_x = timeline_rect.left() + TRACK_HEADER_WIDTH + (range.start.to_seconds() as f32 * state.zoom_px_per_sec) - state.scroll_x;
                let dur_px = range.duration.to_seconds() as f32 * state.zoom_px_per_sec;
                let agent_box = Rect::from_min_size(
                    pos2(start_x, ruler_y),
                    vec2(dur_px, current_y.max(timeline_rect.bottom()) - ruler_y),
                );
                painter.rect_stroke(
                    agent_box,
                    2.0,
                    Stroke::new(1.5, Color32::from_rgba_unmultiplied(245, 158, 11, 180)),
                );
            }
        }

        // 5. Draw Full Timeline Playhead Vertical Line
        let playhead_x = timeline_rect.left() + TRACK_HEADER_WIDTH + (state.playhead.to_seconds() as f32 * state.zoom_px_per_sec) - state.scroll_x;
        if playhead_x >= timeline_rect.left() + TRACK_HEADER_WIDTH && playhead_x <= timeline_rect.right() {
            painter.line_segment(
                [pos2(playhead_x, ruler_y), pos2(playhead_x, current_y.max(timeline_rect.bottom()))],
                Stroke::new(1.5, SlateTheme::PLAYHEAD_LINE),
            );
        }

        // 5b. Draw Blade Cutting Guide Line under cursor when active
        if state.active_tool == ToolMode::Blade {
            if let Some(hover_pos) = response.hover_pos() {
                if hover_pos.x >= timeline_rect.left() + TRACK_HEADER_WIDTH && hover_pos.y >= ruler_y {
                    let cut_color = Color32::from_rgb(239, 68, 68);
                    painter.line_segment(
                        [pos2(hover_pos.x, ruler_y), pos2(hover_pos.x, current_y.max(timeline_rect.bottom()))],
                        Stroke::new(1.5, cut_color),
                    );
                    painter.text(
                        pos2(hover_pos.x + 6.0, hover_pos.y - 10.0),
                        Align2::LEFT_CENTER,
                        "✂ CUT",
                        FontId::monospace(10.0),
                        cut_color,
                    );
                }
            }
        }

        // 6. Handle Mouse Interactions
        if let Some(pos) = response.interact_pointer_pos() {
            let in_ruler = ruler_rect.contains(pos);
            let in_timeline = pos.x >= timeline_rect.left() + TRACK_HEADER_WIDTH && pos.y >= ruler_y;

            if (in_ruler || in_timeline) && (response.dragged_by(PointerButton::Primary) || response.clicked()) && clicked_clip.is_none() {
                let click_offset_x = pos.x - (timeline_rect.left() + TRACK_HEADER_WIDTH) + state.scroll_x;
                let target_secs = (click_offset_x / state.zoom_px_per_sec).max(0.0) as f64;
                let target_time = RationalTime::from_seconds(target_secs, timebase);

                if state.snap_enabled {
                    let snap = {
                        let engine = state.engine.lock();
                        if let Some(tl) = engine.project().active_timeline() {
                            find_snap_target(
                                target_time,
                                tl,
                                state.playhead,
                                state.zoom_px_per_sec,
                                state.scroll_x,
                                state.snap_tolerance_px,
                            )
                        } else {
                            snap::SnapResult {
                                snapped_time: target_time,
                                snap_line_x: None,
                                did_snap: false,
                            }
                        }
                    };
                    state.seek(snap.snapped_time);
                    if let Some(snap_x) = snap.snap_line_x {
                        let snapped_screen_x = timeline_rect.left() + TRACK_HEADER_WIDTH + snap_x;
                        painter.line_segment(
                            [pos2(snapped_screen_x, ruler_y), pos2(snapped_screen_x, current_y.max(timeline_rect.bottom()))],
                            Stroke::new(1.0, SlateTheme::SNAP_LINE),
                        );
                    }
                } else {
                    state.seek(target_time);
                }
            }
        }

        // 7. Handle Clip Click Events based on Active Tool
        if let Some((track_id, clip_id, _area, click_pos)) = clicked_clip {
            match state.active_tool {
                ToolMode::Select => {
                    if ui.input(|i| i.modifiers.shift) {
                        if state.selected_clip_ids.contains(&clip_id) {
                            state.selected_clip_ids.remove(&clip_id);
                        } else {
                            state.selected_clip_ids.insert(clip_id);
                        }
                    } else {
                        state.selected_clip_ids.clear();
                        state.selected_clip_ids.insert(clip_id);
                    }
                    state.active_track_id = Some(track_id);

                    // Move playhead to click position
                    let click_offset_x = click_pos.x - (timeline_rect.left() + TRACK_HEADER_WIDTH) + state.scroll_x;
                    let target_secs = (click_offset_x / state.zoom_px_per_sec).max(0.0) as f64;
                    let target_time = RationalTime::from_seconds(target_secs, timebase);
                    state.seek(target_time);
                }
                ToolMode::Blade => {
                    let click_offset_x = click_pos.x - (timeline_rect.left() + TRACK_HEADER_WIDTH) + state.scroll_x;
                    let target_secs = (click_offset_x / state.zoom_px_per_sec).max(0.0) as f64;
                    let cut_time = RationalTime::from_seconds(target_secs, timebase);

                    if state.split_clip_at(clip_id, cut_time).is_ok() {
                        state.seek(cut_time);
                    }
                }
                _ => {}
            }
        }

        // 8. Handle Zoom and Horizontal Scroll with Mouse Wheel
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                if scroll_delta.y.abs() > 0.0 {
                    let zoom_factor = if scroll_delta.y > 0.0 { 1.15 } else { 0.85 };
                    state.zoom_px_per_sec = (state.zoom_px_per_sec * zoom_factor).clamp(5.0, 500.0);
                }
            } else if scroll_delta.x.abs() > 0.0 || scroll_delta.y.abs() > 0.0 {
                let dx = if scroll_delta.x.abs() > 0.0 { scroll_delta.x } else { scroll_delta.y };
                state.scroll_x = (state.scroll_x - dx).max(0.0);
            }
        }
    }
}
