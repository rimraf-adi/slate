//! Custom egui timeline painter and interaction surface.

pub mod clip_view;
pub mod ruler;
pub mod snap;
pub mod tools;
pub mod track_view;

use egui::{pos2, vec2, Color32, PointerButton, Rect, Sense, Stroke, Ui};
use slate_core::id::{ClipId, TrackId};
use slate_core::model::TrackItem;
use slate_core::time::RationalTime;
use crate::state::{EditorState, ToolMode};
use crate::theme::SlateTheme;

use clip_view::{draw_clip, ClipGeometry, ClipHitArea};
use ruler::{draw_ruler, RULER_HEIGHT};
use snap::find_snap_target;
use track_view::{draw_track_header, DEFAULT_TRACK_HEIGHT, TRACK_HEADER_WIDTH};

pub struct TimelineWidget;

impl TimelineWidget {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        let (timeline_rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), ui.available_height().max(200.0)),
            Sense::click_and_drag(),
        );

        let painter = ui.painter_at(timeline_rect);

        // Background
        painter.rect_filled(timeline_rect, 0.0, SlateTheme::BG_TIMELINE);

        let (timebase, total_duration_secs, tracks_data) = {
            let engine = state.engine.lock();
            let tl_opt = engine.project().active_timeline();
            let tb = tl_opt.map(|t| t.settings.timebase).unwrap_or(30);
            let dur = tl_opt.map(|t| t.duration().to_seconds()).unwrap_or(0.0);
            let tracks = tl_opt.map(|t| t.tracks.clone()).unwrap_or_default();
            (tb, dur, tracks)
        };

        // 1. Draw Ruler
        let ruler_rect = Rect::from_min_size(
            pos2(timeline_rect.left() + TRACK_HEADER_WIDTH, timeline_rect.top()),
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

        // Top-left corner box (above track headers)
        let corner_rect = Rect::from_min_size(
            timeline_rect.min,
            vec2(TRACK_HEADER_WIDTH, RULER_HEIGHT),
        );
        painter.rect_filled(corner_rect, 0.0, SlateTheme::BG_DARK);
        painter.line_segment([corner_rect.right_top(), corner_rect.right_bottom()], Stroke::new(1.0, SlateTheme::BORDER));
        painter.line_segment([corner_rect.left_bottom(), corner_rect.right_bottom()], Stroke::new(1.0, SlateTheme::BORDER));

        // 2. Draw Tracks & Clips
        let mut current_y = timeline_rect.top() + RULER_HEIGHT;
        let mut clicked_clip: Option<(TrackId, ClipId, ClipHitArea)> = None;

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
                                clicked_clip = Some((track.id, clip.id, area));
                            }
                        }
                    }
                }
            }

            current_y += track_h;
        }

        // 3. Draw Agent Presence Range Indicator (if active)
        if let Some(ref presence) = state.agent_presence {
            if let Some(range) = presence.active_range {
                let start_x = timeline_rect.left() + TRACK_HEADER_WIDTH + (range.start.to_seconds() as f32 * state.zoom_px_per_sec) - state.scroll_x;
                let dur_px = range.duration.to_seconds() as f32 * state.zoom_px_per_sec;
                let agent_box = Rect::from_min_size(
                    pos2(start_x, timeline_rect.top() + RULER_HEIGHT),
                    vec2(dur_px, current_y - (timeline_rect.top() + RULER_HEIGHT)),
                );
                painter.rect_stroke(
                    agent_box,
                    2.0,
                    Stroke::new(1.5, Color32::from_rgba_unmultiplied(245, 158, 11, 180)),
                );
            }
        }

        // 4. Draw Full Timeline Playhead Vertical Line
        let playhead_x = timeline_rect.left() + TRACK_HEADER_WIDTH + (state.playhead.to_seconds() as f32 * state.zoom_px_per_sec) - state.scroll_x;
        if playhead_x >= timeline_rect.left() + TRACK_HEADER_WIDTH && playhead_x <= timeline_rect.right() {
            painter.line_segment(
                [pos2(playhead_x, timeline_rect.top() + RULER_HEIGHT), pos2(playhead_x, current_y.max(timeline_rect.bottom()))],
                Stroke::new(1.5, SlateTheme::PLAYHEAD_LINE),
            );
        }

        // 5. Handle Mouse Interactions
        if let Some(pos) = response.interact_pointer_pos() {
            let in_ruler = ruler_rect.contains(pos);
            let in_timeline = pos.x >= timeline_rect.left() + TRACK_HEADER_WIDTH && pos.y >= timeline_rect.top();

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
                                [pos2(snapped_screen_x, timeline_rect.top()), pos2(snapped_screen_x, current_y)],
                                Stroke::new(1.0, SlateTheme::SNAP_LINE),
                            );
                        }
                    } else {
                        state.seek(target_time);
                    }
                }
            }

        // 6. Handle Clip Click Events based on Active Tool
        if let Some((track_id, clip_id, _area)) = clicked_clip {
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
                }
                ToolMode::Blade => {
                    // Split clip at current playhead
                    let _ = state.split_at_playhead();
                }
                _ => {}
            }
        }

        // 7. Handle Zoom and Horizontal Scroll with Mouse Wheel
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                // Zoom
                if scroll_delta.y.abs() > 0.0 {
                    let zoom_factor = if scroll_delta.y > 0.0 { 1.15 } else { 0.85 };
                    state.zoom_px_per_sec = (state.zoom_px_per_sec * zoom_factor).clamp(5.0, 500.0);
                }
            } else {
                // Horizontal scroll
                if scroll_delta.x.abs() > 0.0 || scroll_delta.y.abs() > 0.0 {
                    let dx = if scroll_delta.x.abs() > 0.0 { scroll_delta.x } else { scroll_delta.y };
                    state.scroll_x = (state.scroll_x - dx).max(0.0);
                }
            }
        }
    }
}
