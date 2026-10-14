//! Transcript viewer and text-based timeline editing panel.

use egui::{Color32, FontId, ScrollArea, Ui};
use slate_analysis::transcribe::purge_filler_words;
use slate_core::op::Op;
use slate_core::OpSource;
use slate_core::time::TimeRange;
use crate::state::EditorState;
use crate::theme::SlateTheme;

pub struct TranscriptPanel;

impl TranscriptPanel {
    pub fn show(ui: &mut Ui, state: &mut EditorState) {
        ui.heading("Transcript & Text-Based Edit");
        ui.separator();

        // Action Toolbar
        ui.horizontal(|ui| {
            if ui.button("⚡ Purge Filler Words (um/uh)").clicked() {
                if let Some(ref tr) = state.transcript {
                    let keep_ranges = purge_filler_words(&tr.words, &[]);

                    let mut ranges_to_delete = Vec::new();
                    for (i, word) in tr.words.iter().enumerate() {
                        let is_kept = keep_ranges
                            .iter()
                            .any(|kr| i >= kr.start_word_idx && i <= kr.end_word_idx);
                        if !is_kept {
                            ranges_to_delete.push(TimeRange::from_start_end(word.start, word.end));
                        }
                    }

                    if !ranges_to_delete.is_empty() {
                        let _ = state.apply_op(
                            Op::RippleDelete {
                                ranges: ranges_to_delete,
                                track_ids: None,
                            },
                            OpSource::Human,
                        );
                    }
                }
            }

            if ui.button("✂ Delete Selected Words").clicked() {
                if let Some(ref tr) = state.transcript {
                    if !state.selected_transcript_words.is_empty() {
                        let mut ranges = Vec::new();
                        for &idx in &state.selected_transcript_words {
                            if let Some(word) = tr.words.get(idx) {
                                ranges.push(TimeRange::from_start_end(word.start, word.end));
                            }
                        }

                        let _ = state.apply_op(
                            Op::RippleDelete {
                                ranges,
                                track_ids: None,
                            },
                            OpSource::Human,
                        );

                        state.selected_transcript_words.clear();
                    }
                }
            }
        });

        ui.add_space(8.0);

        // Word Cloud / Flow Container
        if let Some(ref tr) = state.transcript {
            let mut word_click: Option<(usize, bool, slate_core::time::RationalTime)> = None;
            let playhead_secs = state.playhead.to_seconds();

            ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                    for (i, word) in tr.words.iter().enumerate() {
                        let is_selected = state.selected_transcript_words.contains(&i);
                        let is_under_playhead = playhead_secs >= word.start.to_seconds()
                            && playhead_secs <= word.end.to_seconds();

                        let text_color = if is_under_playhead {
                            SlateTheme::ACCENT_AMBER
                        } else if is_selected {
                            Color32::WHITE
                        } else {
                            SlateTheme::TEXT_PRIMARY
                        };

                        let label = egui::RichText::new(&word.word)
                            .font(FontId::proportional(14.0))
                            .color(text_color);

                        let resp = ui.selectable_label(is_selected, label);
                        if resp.clicked() {
                            let is_shift = ui.input(|inp| inp.modifiers.shift);
                            word_click = Some((i, is_shift, word.start));
                        }
                    }
                });
            });

            if let Some((idx, is_shift, start_time)) = word_click {
                if is_shift {
                    if state.selected_transcript_words.contains(&idx) {
                        state.selected_transcript_words.remove(&idx);
                    } else {
                        state.selected_transcript_words.insert(idx);
                    }
                } else {
                    state.selected_transcript_words.clear();
                    state.selected_transcript_words.insert(idx);
                    state.seek(start_time);
                }
            }
        } else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No transcript available.\nImport an audio or video clip to generate automatic transcripts.")
                        .color(SlateTheme::TEXT_MUTED)
                        .italics(),
                );
            });
        }
    }
}
