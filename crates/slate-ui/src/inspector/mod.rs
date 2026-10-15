//! Selected clip inspector panel for transform, audio, and keyframes.

use egui::{DragValue, FontId, Slider, Ui};
use slate_core::id::{ClipId, KeyframeId, TrackId};
use slate_core::model::keyframe::{Interpolation, Keyframe};
use slate_core::model::{Clip, TrackItem};
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::{RationalTime, Timecode};
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

        if let Some((_track_id, clip_id, clip)) = clip_data {
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
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Clip:")
                            .font(FontId::proportional(11.0))
                            .color(SlateTheme::TEXT_MUTED),
                    );
                    ui.label(
                        egui::RichText::new(&clip.name)
                            .font(FontId::proportional(12.5))
                            .strong(),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Duration:")
                            .font(FontId::proportional(11.0))
                            .color(SlateTheme::TEXT_MUTED),
                    );
                    ui.label(
                        egui::RichText::new(Timecode::format(clip.timeline_range.duration, false))
                            .font(FontId::monospace(11.0))
                            .color(SlateTheme::ACCENT_CYAN),
                    );
                });
            });

            ui.add_space(8.0);

            // Transform Group
            egui::CollapsingHeader::new("🎬 Video & Transform")
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
                        ui.label("Position X:");
                        if ui.add(DragValue::new(&mut pos_x).speed(0.01).range(-2.0..=2.0)).changed() {
                            changed = true;
                        }
                        ui.label("Y:");
                        if ui.add(DragValue::new(&mut pos_y).speed(0.01).range(-2.0..=2.0)).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Scale X:");
                        if ui.add(DragValue::new(&mut scale_x).speed(0.01).range(0.0..=10.0)).changed() {
                            changed = true;
                        }
                        ui.label("Y:");
                        if ui.add(DragValue::new(&mut scale_y).speed(0.01).range(0.0..=10.0)).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Rotation:");
                        if ui.add(DragValue::new(&mut rot).speed(0.5).suffix("°")).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Opacity:");
                        if ui.add(Slider::new(&mut opacity, 0.0..=1.0)).changed() {
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

            ui.add_space(6.0);

            // Audio Group
            egui::CollapsingHeader::new("🔊 Audio & Levels")
                .default_open(true)
                .show(ui, |ui| {
                    let mut vol_db = clip.audio_params.volume_db.evaluate_at(RationalTime::ZERO, 0.0);
                    let mut pan = clip.audio_params.pan.evaluate_at(RationalTime::ZERO, 0.0);
                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label("Volume:");
                        if ui.add(Slider::new(&mut vol_db, -60.0..=12.0).suffix(" dB")).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Pan:");
                        if ui.add(Slider::new(&mut pan, -1.0..=1.0)).changed() {
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

                        let op = Op::SetAudioParams {
                            clip_id,
                            params: new_audio,
                        };
                        let _ = state.apply_op(op, OpSource::Human);
                    }
                });

            ui.add_space(10.0);

            // Clip Quick Actions
            ui.horizontal(|ui| {
                if ui.button("✂ Split (C)").on_hover_text("Split clip at current playhead (or in half)").clicked() {
                    let _ = state.split_selected_or_at_playhead();
                }
                if ui.button("🗑 Ripple Delete").on_hover_text("Delete clip and ripple close the gap (Del)").clicked() {
                    let _ = state.ripple_delete_selected();
                }
            });
        } else {
            // Sequence & Project Properties (Professional default when idle)
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

            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("⚙ Project Inspector")
                        .font(FontId::proportional(15.0))
                        .strong(),
                );
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);

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
                    ui.label("48,000 Hz Stereo");
                });
            });

            ui.add_space(12.0);

            ui.label(
                egui::RichText::new("Tip: Select any clip on the timeline to inspect and keyframe position, scale, opacity, and audio gain.")
                    .font(FontId::proportional(11.0))
                    .color(SlateTheme::TEXT_MUTED)
                    .italics(),
            );
        }
    }
}
