//! Project Media Pool asset browser and file importer.

use egui::{FontId, ScrollArea, Ui};
use std::path::PathBuf;

use slate_core::id::{MediaId, TrackId};
use slate_core::model::{Clip, MediaMetadata, MediaReference};
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::{RationalTime, TimeRange};
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct MediaPoolPanel;

impl MediaPoolPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            ui.heading("Media Pool");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ Import Media").clicked() {
                    // Quick import sample asset into media pool
                    let mut engine = state.engine.lock();
                    let asset_id = MediaId::new();
                    let asset = MediaReference {
                        id: asset_id,
                        original_path: PathBuf::from("sample_footage.mp4"),
                        relative_path: PathBuf::from("sample_footage.mp4"),
                        metadata: MediaMetadata {
                            file_size_bytes: 1024 * 1024 * 25,
                            duration: RationalTime::from_seconds(10.0, 30),
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

        ui.separator();

        let assets: Vec<MediaReference> = {
            let engine = state.engine.lock();
            engine.project().media_pool.clone()
        };

        if assets.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No media imported.\nClick 'Import Media' or drag & drop files here.")
                        .color(SlateTheme::TEXT_MUTED)
                        .italics(),
                );
            });
            return;
        }

        let mut add_to_timeline_media: Option<(MediaId, String, RationalTime)> = None;

        ScrollArea::vertical().show(ui, |ui| {
            for asset in &assets {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let has_v = asset.metadata.video_codec.is_some();
                        let has_a = asset.metadata.audio_codec.is_some();
                        let icon = match (has_v, has_a) {
                            (true, true) => "🎬 [A/V]",
                            (true, false) => "🎥 [V]",
                            (false, true) => "🎵 [A]",
                            _ => "📄",
                        };

                        ui.label(
                            egui::RichText::new(icon)
                                .font(FontId::monospace(12.0))
                                .color(SlateTheme::ACCENT_BLUE_LIGHT),
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

                    ui.horizontal(|ui| {
                        let dur_secs = asset.metadata.duration.to_seconds();
                        ui.label(
                            egui::RichText::new(format!("{:.1}s", dur_secs))
                                .font(FontId::monospace(10.0))
                                .color(SlateTheme::TEXT_MUTED),
                        );

                        if ui.button("➕ Add to Timeline").clicked() {
                            let file_name = asset.original_path.file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or("Clip")
                                .to_string();
                            add_to_timeline_media = Some((asset.id, file_name, asset.metadata.duration));
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });

        if let Some((media_id, name, duration)) = add_to_timeline_media {
            let target_track_id: Option<TrackId> = {
                let engine = state.engine.lock();
                engine.project().active_timeline().and_then(|tl| tl.tracks.first().map(|t| t.id))
            };

            if let Some(track_id) = target_track_id {
                let timebase = duration.rate;
                let clip = Clip::new(
                    name,
                    media_id,
                    TimeRange::new(RationalTime::from_frames(0, timebase), duration),
                    TimeRange::new(state.playhead, duration),
                );

                let op = Op::AddClip {
                    track_id,
                    clip,
                    position: state.playhead,
                };
                let _ = state.apply_op(op, OpSource::Human);
            }
        }
    }
}
