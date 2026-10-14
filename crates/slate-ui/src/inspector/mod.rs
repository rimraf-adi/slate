//! Selected clip inspector panel for transform, audio, and keyframes.

use egui::{DragValue, Slider, Ui};
use slate_core::id::{ClipId, KeyframeId, TrackId};
use slate_core::model::keyframe::{Interpolation, Keyframe};
use slate_core::model::{Clip, TrackItem};
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::RationalTime;
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct InspectorPanel;

impl InspectorPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        ui.heading("Clip Inspector");
        ui.separator();

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
            ui.label(format!("Clip: {}", clip.name));
            ui.label(
                egui::RichText::new(format!("ID: {}", clip_id))
                    .font(egui::FontId::monospace(10.0))
                    .color(SlateTheme::TEXT_MUTED),
            );
            ui.add_space(8.0);

            // Transform Group
            egui::CollapsingHeader::new("Transform")
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

            ui.add_space(8.0);

            // Audio Group
            egui::CollapsingHeader::new("Audio & Gain")
                .default_open(true)
                .show(ui, |ui| {
                    let mut vol_db = clip.audio_params.volume_db.evaluate_at(RationalTime::ZERO, 0.0);
                    let mut pan = clip.audio_params.pan.evaluate_at(RationalTime::ZERO, 0.0);
                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label("Volume (dB):");
                        if ui.add(Slider::new(&mut vol_db, -60.0..=12.0)).changed() {
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

            ui.add_space(8.0);

            // Actions
            ui.horizontal(|ui| {
                if ui.button("✂ Split at Playhead").clicked() {
                    let _ = state.split_at_playhead();
                }
                if ui.button("🗑 Delete (Ripple)").clicked() {
                    let _ = state.ripple_delete_selected();
                }
            });
        } else {
            ui.label(
                egui::RichText::new("Select a clip on the timeline to inspect its properties.")
                    .color(SlateTheme::TEXT_MUTED),
            );
        }
    }
}
