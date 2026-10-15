//! Project Media Pool asset browser and file importer.

use egui::{vec2, Color32, FontId, ScrollArea, Stroke, Ui};
use std::path::PathBuf;

use slate_core::id::MediaId;
use slate_core::model::{Clip, MediaMetadata, MediaReference, TrackItem, TrackType};
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::{RationalTime, TimeRange, Timecode};
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct MediaPoolPanel;

impl MediaPoolPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        // Header
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🎬 Media Pool")
                    .font(FontId::proportional(15.0))
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ Import Media").on_hover_text("Import video, audio, or image files").clicked() {
                    let mut engine = state.engine.lock();
                    let asset_id = MediaId::new();
                    let count = engine.project().media_pool.len() + 1;
                    let asset = MediaReference {
                        id: asset_id,
                        original_path: PathBuf::from(format!("scene_{:02}_raw.mp4", count)),
                        relative_path: PathBuf::from(format!("scene_{:02}_raw.mp4", count)),
                        metadata: MediaMetadata {
                            file_size_bytes: 1024 * 1024 * 45,
                            duration: RationalTime::from_seconds(8.0 + (count as f64 * 2.0), 30),
                            video_codec: Some("h264".into()),
                            width: Some(1920),
                            height: Some(1080),
                            frame_rate: Some(30.0),
                            timebase: 30,
                            audio_codec: Some("aac".into()),
                            audio_channels: Some(2),
                            sample_rate: Some(48000),
                        },
                        proxy_path: None,
                        waveform_path: None,
                    };
                    engine.project_mut().media_pool.push(asset);
                }
            });
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        let assets: Vec<MediaReference> = {
            let engine = state.engine.lock();
            engine.project().media_pool.clone()
        };

        if assets.is_empty() {
            // Elegant modern dropzone card
            ui.add_space(16.0);
            let dropzone_h = 160.0;
            let (rect, _resp) = ui.allocate_exact_size(vec2(ui.available_width(), dropzone_h), egui::Sense::hover());
            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 6.0, SlateTheme::BG_DARKEST);
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.5, Color32::from_rgb(60, 65, 80)),
            );

            painter.text(
                egui::pos2(rect.center().x, rect.center().y - 25.0),
                egui::Align2::CENTER_CENTER,
                "📁",
                FontId::proportional(28.0),
                Color32::WHITE,
            );
            painter.text(
                egui::pos2(rect.center().x, rect.center().y + 8.0),
                egui::Align2::CENTER_CENTER,
                "No Footage Imported",
                FontId::proportional(13.0),
                SlateTheme::TEXT_PRIMARY,
            );
            painter.text(
                egui::pos2(rect.center().x, rect.center().y + 26.0),
                egui::Align2::CENTER_CENTER,
                "Drag & drop media or click below",
                FontId::proportional(11.0),
                SlateTheme::TEXT_MUTED,
            );

            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                if ui.button("🎬 Add Demo 1080p Clip").clicked() {
                    let asset_id = MediaId::new();
                    let asset = MediaReference {
                        id: asset_id,
                        original_path: PathBuf::from("A-Roll_Interview_01.mp4"),
                        relative_path: PathBuf::from("A-Roll_Interview_01.mp4"),
                        metadata: MediaMetadata {
                            file_size_bytes: 1024 * 1024 * 85,
                            duration: RationalTime::from_seconds(12.0, 30),
                            video_codec: Some("h264".into()),
                            width: Some(1920),
                            height: Some(1080),
                            frame_rate: Some(30.0),
                            timebase: 30,
                            audio_codec: Some("aac".into()),
                            audio_channels: Some(2),
                            sample_rate: Some(48000),
                        },
                        proxy_path: None,
                        waveform_path: None,
                    };
                    {
                        let mut engine = state.engine.lock();
                        engine.project_mut().media_pool.push(asset.clone());
                    }
                    insert_asset_onto_timeline(state, &asset);
                }

                if ui.button("🎵 Add Audio B-Roll Track").clicked() {
                    let asset_id = MediaId::new();
                    let asset = MediaReference {
                        id: asset_id,
                        original_path: PathBuf::from("Cinematic_Soundtrack.wav"),
                        relative_path: PathBuf::from("Cinematic_Soundtrack.wav"),
                        metadata: MediaMetadata {
                            file_size_bytes: 1024 * 1024 * 18,
                            duration: RationalTime::from_seconds(24.0, 48000),
                            video_codec: None,
                            width: None,
                            height: None,
                            frame_rate: None,
                            timebase: 48000,
                            audio_codec: Some("pcm_s16le".into()),
                            audio_channels: Some(2),
                            sample_rate: Some(48000),
                        },
                        proxy_path: None,
                        waveform_path: None,
                    };
                    {
                        let mut engine = state.engine.lock();
                        engine.project_mut().media_pool.push(asset.clone());
                    }
                    insert_asset_onto_timeline(state, &asset);
                }
            });
            return;
        }

        // Summary bar
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} items in pool", assets.len()))
                    .font(FontId::proportional(11.0))
                    .color(SlateTheme::TEXT_MUTED),
            );
        });
        ui.add_space(4.0);

        let mut add_to_timeline_asset: Option<MediaReference> = None;

        ScrollArea::vertical().show(ui, |ui| {
            for asset in &assets {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());

                    let has_v = asset.metadata.video_codec.is_some();
                    let has_a = asset.metadata.audio_codec.is_some();

                    ui.horizontal(|ui| {
                        let (badge_text, badge_color) = match (has_v, has_a) {
                            (true, true) => ("AV 1080p", SlateTheme::ACCENT_CYAN),
                            (true, false) => ("VIDEO", SlateTheme::ACCENT_BLUE_LIGHT),
                            (false, true) => ("AUDIO", SlateTheme::ACCENT_EMERALD),
                            _ => ("FILE", SlateTheme::TEXT_MUTED),
                        };

                        ui.label(
                            egui::RichText::new(badge_text)
                                .font(FontId::monospace(9.5))
                                .background_color(Color32::from_black_alpha(140))
                                .color(badge_color)
                                .strong(),
                        );

                        let file_name = asset.original_path.file_name()
                            .and_then(|f| f.to_str())
                            .unwrap_or("media_file");
                        ui.label(
                            egui::RichText::new(file_name)
                                .font(FontId::proportional(12.0))
                                .strong(),
                        );
                    });

                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        let dur_secs = asset.metadata.duration.to_seconds();
                        let size_mb = asset.metadata.file_size_bytes as f64 / (1024.0 * 1024.0);
                        ui.label(
                            egui::RichText::new(format!("{:.1}s  •  {:.1} MB", dur_secs, size_mb))
                                .font(FontId::monospace(10.5))
                                .color(SlateTheme::TEXT_MUTED),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("➕ Insert").on_hover_text("Insert clip onto timeline").clicked() {
                                add_to_timeline_asset = Some(asset.clone());
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });

        if let Some(asset) = add_to_timeline_asset {
            insert_asset_onto_timeline(state, &asset);
        }
    }
}

pub fn insert_asset_onto_timeline(state: &mut EditorState, asset: &MediaReference) {
    let has_video = asset.metadata.video_codec.is_some();
    let duration = asset.metadata.duration;
    let file_name = asset.original_path.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("Clip")
        .to_string();

    // 1. Find or create appropriate track
    let track_id = {
        let mut engine = state.engine.lock();
        let active_tl = engine.project_mut().active_timeline_mut();
        if let Some(tl) = active_tl {
            let needed_type = if has_video { TrackType::Video } else { TrackType::Audio };
            if let Some(t) = tl.tracks.iter().find(|t| t.track_type == needed_type && !t.locked) {
                t.id
            } else {
                let name = if has_video {
                    format!("V{}", tl.tracks.iter().filter(|t| t.track_type == TrackType::Video).count() + 1)
                } else {
                    format!("A{}", tl.tracks.iter().filter(|t| t.track_type == TrackType::Audio).count() + 1)
                };
                let track = slate_core::model::Track::new(name, needed_type);
                let id = track.id;
                tl.tracks.push(track);
                id
            }
        } else {
            return;
        }
    };

    // 2. Find next available time (playhead if non-overlapping, or end of track)
    let target_time = {
        let engine = state.engine.lock();
        if let Some(tl) = engine.project().active_timeline() {
            if let Some(t) = tl.find_track(track_id) {
                let test_range = TimeRange::new(state.playhead, duration);
                let overlaps = t.items.iter().any(|item| {
                    if let TrackItem::Clip(c) = item {
                        c.timeline_range.overlaps(&test_range)
                    } else {
                        false
                    }
                });
                if !overlaps {
                    state.playhead
                } else {
                    let last_end = t.items.iter().filter_map(|item| {
                        if let TrackItem::Clip(c) = item {
                            Some(c.timeline_range.end_time())
                        } else {
                            None
                        }
                    }).max().unwrap_or(RationalTime::ZERO);
                    last_end
                }
            } else {
                state.playhead
            }
        } else {
            state.playhead
        }
    };

    let clip = Clip::new(
        file_name.clone(),
        asset.id,
        TimeRange::new(RationalTime::from_frames(0, duration.rate), duration),
        TimeRange::new(target_time, duration),
    );
    let clip_id = clip.id;

    let op = Op::AddClip {
        track_id,
        clip,
        position: target_time,
    };
    if state.apply_op(op, OpSource::Human).is_ok() {
        state.selected_clip_ids.clear();
        state.selected_clip_ids.insert(clip_id);
        state.active_track_id = Some(track_id);
        state.seek(target_time);
        state.set_status(format!("✓ Inserted clip '{}' at {}", file_name, Timecode::format(target_time, false)), false);
    }
}
