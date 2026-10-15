//! Selected clip inspector panel for transform, crop, pan, audio sound effects, and text titles.

use egui::{Color32, DragValue, FontId, ScrollArea, Slider, Ui};
use slate_core::id::{CaptionId, ClipId, KeyframeId, TrackId};
use slate_core::model::caption::{Caption, CaptionStyle};
use slate_core::model::keyframe::{Interpolation, Keyframe};
use slate_core::model::{Clip, TrackItem};
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::{RationalTime, TimeRange, Timecode};
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct InspectorPanel;

impl InspectorPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        let active_clip_id = state.selected_clip_ids.iter().next().copied();

        let clip_data: Option<(TrackId, ClipId, Clip)> = active_clip_id.and_then(|cid| {
            let engine = state.engine.lock();
            if let Some(tl) = engine.project().active_timeline() {
                for track in &tl.tracks {
                    for item in &track.items {
                        if let TrackItem::Clip(clip) = item {
                            if clip.id == cid {
                                return Some((track.id, clip.id, clip.clone()));
                            }
                        }
                    }
                }
            }
            None
        });

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if let Some((_track_id, clip_id, clip)) = clip_data {
                // Header
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("⚙ Clip Inspector")
                            .font(FontId::proportional(15.0))
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("SELECTED")
                                .font(FontId::monospace(9.5))
                                .background_color(SlateTheme::ACCENT_BLUE)
                                .color(Color32::WHITE)
                                .strong(),
                        );
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Clip Summary Badge
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Clip:").font(FontId::proportional(11.0)).color(SlateTheme::TEXT_MUTED));
                        ui.label(egui::RichText::new(&clip.name).font(FontId::proportional(12.5)).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Duration:").font(FontId::proportional(11.0)).color(SlateTheme::TEXT_MUTED));
                        ui.label(
                            egui::RichText::new(Timecode::format(clip.timeline_range.duration, false))
                                .font(FontId::monospace(11.0))
                                .color(SlateTheme::ACCENT_CYAN),
                        );
                    });
                });

                ui.add_space(6.0);

                // 1. Pan, Zoom & Transform Group
                egui::CollapsingHeader::new("🎬 Pan, Zoom & Transform")
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut pos_x = clip.transform.position_x.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut pos_y = clip.transform.position_y.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut scale_x = clip.transform.scale_x.evaluate_at(RationalTime::ZERO, 1.0);
                        let mut scale_y = clip.transform.scale_y.evaluate_at(RationalTime::ZERO, 1.0);
                        let mut rot = clip.transform.rotation_degrees.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut opacity = clip.transform.opacity.evaluate_at(RationalTime::ZERO, 1.0);

                        let mut changed = false;

                        ui.horizontal(|ui| {
                            ui.label("Pan X:");
                            if ui.add(DragValue::new(&mut pos_x).speed(0.01).range(-2.0..=2.0)).changed() {
                                changed = true;
                            }
                            ui.label("Y:");
                            if ui.add(DragValue::new(&mut pos_y).speed(0.01).range(-2.0..=2.0)).changed() {
                                changed = true;
                            }
                            if ui.button("Center").clicked() {
                                pos_x = 0.0;
                                pos_y = 0.0;
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Zoom / Scale:");
                            if ui.add(DragValue::new(&mut scale_x).speed(0.01).range(0.1..=5.0)).changed() {
                                scale_y = scale_x;
                                changed = true;
                            }
                            if ui.button("100%").clicked() {
                                scale_x = 1.0;
                                scale_y = 1.0;
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Rotation:");
                            if ui.add(DragValue::new(&mut rot).speed(0.5).suffix("°")).changed() {
                                changed = true;
                            }
                            if ui.button("0°").clicked() {
                                rot = 0.0;
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Opacity:");
                            if ui.add(Slider::new(&mut opacity, 0.0..=1.0)).changed() {
                                changed = true;
                            }
                        });

                        // Transform Quick Presets
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            if ui.button("Ken Burns Zoom").on_hover_text("Slow zoom-in").clicked() {
                                scale_x = 1.15;
                                scale_y = 1.15;
                                changed = true;
                            }
                            if ui.button("PiP Corner").on_hover_text("Picture-in-picture top-right").clicked() {
                                scale_x = 0.35;
                                scale_y = 0.35;
                                pos_x = 0.55;
                                pos_y = -0.55;
                                changed = true;
                            }
                            if ui.button("Reset").clicked() {
                                pos_x = 0.0;
                                pos_y = 0.0;
                                scale_x = 1.0;
                                scale_y = 1.0;
                                rot = 0.0;
                                opacity = 1.0;
                                changed = true;
                            }
                        });

                        if changed {
                            let mut new_transform = clip.transform.clone();
                            new_transform.position_x.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: pos_x,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.position_y.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: pos_y,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.scale_x.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: scale_x,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.scale_y.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: scale_y,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.rotation_degrees.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: rot,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.opacity.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: opacity,
                                interpolation: Interpolation::Linear,
                            }];

                            let op = Op::SetTransform {
                                clip_id,
                                transform: new_transform,
                            };
                            let _ = state.apply_op(op, OpSource::Human);
                        }
                    });

                ui.add_space(4.0);

                // 2. Crop & Reframe Group
                egui::CollapsingHeader::new("📐 Crop & Reframe")
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut crop_l = clip.transform.crop_left.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut crop_r = clip.transform.crop_right.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut crop_t = clip.transform.crop_top.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut crop_b = clip.transform.crop_bottom.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut changed = false;

                        ui.horizontal(|ui| {
                            ui.label("Left:");
                            if ui.add(Slider::new(&mut crop_l, 0.0..=0.5)).changed() {
                                changed = true;
                            }
                            ui.label("Right:");
                            if ui.add(Slider::new(&mut crop_r, 0.0..=0.5)).changed() {
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Top:");
                            if ui.add(Slider::new(&mut crop_t, 0.0..=0.5)).changed() {
                                changed = true;
                            }
                            ui.label("Bottom:");
                            if ui.add(Slider::new(&mut crop_b, 0.0..=0.5)).changed() {
                                changed = true;
                            }
                        });

                        // Crop Presets
                        ui.horizontal(|ui| {
                            if ui.button("9:16 Shorts").on_hover_text("Reframe 16:9 to vertical 9:16 crop").clicked() {
                                crop_l = 0.28;
                                crop_r = 0.28;
                                crop_t = 0.0;
                                crop_b = 0.0;
                                changed = true;
                            }
                            if ui.button("2.39:1 Cinema").on_hover_text("Cinematic scope letterbox").clicked() {
                                crop_l = 0.0;
                                crop_r = 0.0;
                                crop_t = 0.13;
                                crop_b = 0.13;
                                changed = true;
                            }
                            if ui.button("1:1 Square").on_hover_text("Square Instagram crop").clicked() {
                                crop_l = 0.22;
                                crop_r = 0.22;
                                crop_t = 0.0;
                                crop_b = 0.0;
                                changed = true;
                            }
                            if ui.button("Reset Crop").clicked() {
                                crop_l = 0.0;
                                crop_r = 0.0;
                                crop_t = 0.0;
                                crop_b = 0.0;
                                changed = true;
                            }
                        });

                        if changed {
                            let mut new_transform = clip.transform.clone();
                            new_transform.crop_left.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: crop_l,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.crop_right.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: crop_r,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.crop_top.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: crop_t,
                                interpolation: Interpolation::Linear,
                            }];
                            new_transform.crop_bottom.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: crop_b,
                                interpolation: Interpolation::Linear,
                            }];

                            let op = Op::SetTransform {
                                clip_id,
                                transform: new_transform,
                            };
                            let _ = state.apply_op(op, OpSource::Human);
                        }
                    });

                ui.add_space(4.0);

                // 3. Audio, Pan & Sound Effects Group
                egui::CollapsingHeader::new("🔊 Audio, Pan & Sounds")
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut vol_db = clip.audio_params.volume_db.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut pan = clip.audio_params.pan.evaluate_at(RationalTime::ZERO, 0.0);
                        let mut mute = clip.audio_params.mute;
                        let mut changed = false;

                        ui.horizontal(|ui| {
                            ui.label("Volume:");
                            if ui.add(Slider::new(&mut vol_db, -60.0..=12.0).suffix(" dB")).changed() {
                                changed = true;
                            }
                            if ui.checkbox(&mut mute, "Mute").changed() {
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Stereo Pan:");
                            if ui.add(Slider::new(&mut pan, -1.0..=1.0)).changed() {
                                changed = true;
                            }
                            if ui.button("Center").clicked() {
                                pan = 0.0;
                                changed = true;
                            }
                        });

                        // Sound Presets
                        ui.horizontal(|ui| {
                            if ui.button("Dialogue (+3dB)").clicked() {
                                vol_db = 3.0;
                                pan = 0.0;
                                changed = true;
                            }
                            if ui.button("Music Bed (-14dB)").clicked() {
                                vol_db = -14.0;
                                changed = true;
                            }
                            if ui.button("SFX Punch (+2dB)").clicked() {
                                vol_db = 2.0;
                                changed = true;
                            }
                            if ui.button("Reset Audio").clicked() {
                                vol_db = 0.0;
                                pan = 0.0;
                                mute = false;
                                changed = true;
                            }
                        });

                        if changed {
                            let mut new_audio = clip.audio_params.clone();
                            new_audio.volume_db.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: vol_db,
                                interpolation: Interpolation::Linear,
                            }];
                            new_audio.pan.keyframes = vec![Keyframe {
                                id: KeyframeId::new(),
                                time: RationalTime::ZERO,
                                value: pan,
                                interpolation: Interpolation::Linear,
                            }];
                            new_audio.mute = mute;

                            let op = Op::SetAudioParams {
                                clip_id,
                                params: new_audio,
                            };
                            let _ = state.apply_op(op, OpSource::Human);
                        }
                    });

                ui.add_space(4.0);

                // 4. Quick Actions
                ui.horizontal(|ui| {
                    if ui.button("✂ Split (C)").on_hover_text("Split clip at current playhead (or in half)").clicked() {
                        let _ = state.split_selected_or_at_playhead();
                    }
                    if ui.button("🗑 Ripple Delete").on_hover_text("Delete clip and ripple close the gap (Del)").clicked() {
                        let _ = state.ripple_delete_selected();
                    }
                });

                ui.add_space(8.0);
            }

            // 5. Text Effects, Titles & Captions Section (Always accessible)
            egui::CollapsingHeader::new("📝 Text Effects & Titles")
                .default_open(true)
                .show(ui, |ui| {
                    let playhead = state.playhead;
                    let existing_captions = {
                        let engine = state.engine.lock();
                        engine.project().active_timeline().map(|t| t.captions.clone()).unwrap_or_default()
                    };

                    let active_caption = existing_captions.iter().find(|c| c.time_range.contains_time(playhead)).cloned();

                    if let Some(caption) = active_caption {
                        ui.label(egui::RichText::new("ACTIVE TITLE AT PLAYHEAD:").font(FontId::monospace(9.5)).color(SlateTheme::ACCENT_CYAN).strong());
                        ui.add_space(2.0);

                        let mut text = caption.text.clone();
                        let mut style = caption.style.clone();
                        let mut changed = false;

                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            if ui.add(egui::TextEdit::singleline(&mut text).desired_width(180.0)).changed() {
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            if ui.add(Slider::new(&mut style.font_size, 14.0..=96.0)).changed() {
                                changed = true;
                            }
                            ui.label("Outline:");
                            if ui.add(Slider::new(&mut style.outline_width, 0.0..=10.0)).changed() {
                                changed = true;
                            }
                        });

                        // Text Color & Style Presets
                        ui.horizontal(|ui| {
                            if ui.button("🟡 TikTok Yellow").clicked() {
                                style.text_color_rgba = [1.0, 0.9, 0.1, 1.0];
                                style.bg_color_rgba = [0.0, 0.0, 0.0, 0.8];
                                style.font_size = 48.0;
                                style.outline_width = 3.0;
                                changed = true;
                            }
                            if ui.button("⚪ Cinema White").clicked() {
                                style.text_color_rgba = [1.0, 1.0, 1.0, 1.0];
                                style.bg_color_rgba = [0.0, 0.0, 0.0, 0.6];
                                style.font_size = 36.0;
                                style.outline_width = 1.5;
                                changed = true;
                            }
                            if ui.button("🔷 Neon Cyan").clicked() {
                                style.text_color_rgba = [0.1, 0.9, 1.0, 1.0];
                                style.bg_color_rgba = [0.05, 0.05, 0.15, 0.9];
                                style.font_size = 40.0;
                                style.outline_width = 2.0;
                                changed = true;
                            }
                        });

                        if changed {
                            let op = Op::UpdateCaption {
                                caption_id: caption.id,
                                text: Some(text),
                                style: Some(style),
                            };
                            let _ = state.apply_op(op, OpSource::Human);
                        }

                        if ui.button("🗑 Remove Title").clicked() {
                            let op = Op::RemoveCaption {
                                caption_id: caption.id,
                            };
                            let _ = state.apply_op(op, OpSource::Human);
                        }
                    } else {
                        ui.label(egui::RichText::new("No title overlay at current playhead").font(FontId::proportional(11.0)).color(SlateTheme::TEXT_MUTED));
                        ui.add_space(4.0);

                        if ui.button("➕ Add Title at Playhead (3s)").on_hover_text("Insert styled title card or lower third").clicked() {
                            let cap_id = CaptionId::new();
                            let range = TimeRange::new(playhead, RationalTime::from_seconds(3.0, playhead.rate));
                            let new_cap = Caption {
                                id: cap_id,
                                time_range: range,
                                text: "New Title Overlay".to_string(),
                                style: CaptionStyle {
                                    font_family: "Inter".to_string(),
                                    font_size: 42.0,
                                    font_weight: 700,
                                    text_color_rgba: [1.0, 0.95, 0.2, 1.0], // Eye-catching TikTok/Reels Yellow
                                    bg_color_rgba: [0.0, 0.0, 0.0, 0.75],
                                    outline_color_rgba: [0.0, 0.0, 0.0, 1.0],
                                    outline_width: 2.5,
                                    vertical_alignment: "bottom".to_string(),
                                    margin_bottom_percent: 0.12,
                                },
                            };
                            let op = Op::AddCaption { caption: new_cap };
                            let _ = state.apply_op(op, OpSource::Human);
                        }
                    }
                });

            ui.add_space(8.0);

            // 6. Project & Sequence Overview (if no clip selected)
            if active_clip_id.is_none() {
                let (proj_name, seq_name, width, height, fps, track_count, total_dur) = {
                    let engine = state.engine.lock();
                    let p = engine.project();
                    let tl = p.active_timeline();
                    (
                        p.name.clone(),
                        tl.map(|t| t.name.clone()).unwrap_or_else(|| "Main Sequence".into()),
                        tl.map(|t| t.settings.width).unwrap_or(1920),
                        tl.map(|t| t.settings.height).unwrap_or(1080),
                        tl.map(|t| t.settings.timebase).unwrap_or(30),
                        tl.map(|t| t.tracks.len()).unwrap_or(0),
                        tl.map(|t| t.duration()).unwrap_or(RationalTime::ZERO),
                    )
                };

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new("SEQUENCE PROPERTIES")
                            .font(FontId::monospace(9.5))
                            .color(SlateTheme::TEXT_MUTED)
                            .strong(),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Project:");
                        ui.label(egui::RichText::new(&proj_name).strong());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sequence:");
                        ui.label(egui::RichText::new(&seq_name).color(SlateTheme::TEXT_PRIMARY));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Canvas:");
                        ui.label(
                            egui::RichText::new(format!("{} × {} (16:9)", width, height))
                                .color(SlateTheme::ACCENT_CYAN)
                                .strong(),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Timebase:");
                        ui.label(format!("{}.00 fps", fps));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Duration:");
                        ui.label(
                            egui::RichText::new(Timecode::format(total_dur, false))
                                .font(FontId::monospace(11.0))
                                .color(SlateTheme::TEXT_PRIMARY),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Active Tracks:");
                        ui.label(format!("{} tracks", track_count));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Audio Rate:");
                        ui.label("48,000 Hz Stereo (Equal-Power Pan)");
                    });
                });
            }
        });
    }
}
