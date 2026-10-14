use slate_core::time::{RationalTime, TimeRange};
use crate::types::{VadParams, VadResult};

/// Voice activity detector analyzing audio PCM energy to detect speech and silences.
pub struct EnergyVad {
    params: VadParams,
    sample_rate: u32,
}

impl EnergyVad {
    pub fn new(sample_rate: u32, params: VadParams) -> Self {
        Self {
            params,
            sample_rate,
        }
    }

    /// Processes an audio channel slice (f32 normalized to [-1.0, 1.0])
    /// and extracts speech segments and silence segments.
    pub fn process(&self, samples: &[f32]) -> VadResult {
        if samples.is_empty() {
            return VadResult {
                speech_segments: Vec::new(),
                silence_segments: Vec::new(),
            };
        }

        let window_size = self.params.window_size_samples.max(128) as usize;
        let num_windows = samples.len() / window_size;
        let window_duration_sec = window_size as f64 / self.sample_rate as f64;

        // Compute RMS energy for each window
        let mut window_energies = Vec::with_capacity(num_windows);
        for i in 0..num_windows {
            let chunk = &samples[i * window_size..(i + 1) * window_size];
            let sum_sq: f32 = chunk.iter().map(|&s| s * s).sum();
            let rms = (sum_sq / chunk.len() as f32).sqrt();
            window_energies.push(rms);
        }

        // Determine energy threshold (scaled by input parameter)
        let rms_threshold = (self.params.threshold * 0.03).max(0.001);

        // State machine to cluster active frames
        let min_speech_windows = ((self.params.min_speech_duration_ms as f64 / 1000.0)
            / window_duration_sec)
            .ceil() as usize;
        let min_silence_windows = ((self.params.min_silence_duration_ms as f64 / 1000.0)
            / window_duration_sec)
            .ceil() as usize;

        let mut raw_speech_spans: Vec<(usize, usize)> = Vec::new();
        let mut in_speech = false;
        let mut speech_start = 0;
        let mut silence_count = 0;

        for (i, &energy) in window_energies.iter().enumerate() {
            let is_speech = energy >= rms_threshold;
            if is_speech {
                if !in_speech {
                    in_speech = true;
                    speech_start = i;
                }
                silence_count = 0;
            } else if in_speech {
                silence_count += 1;
                if silence_count >= min_silence_windows {
                    let speech_end = i - silence_count + 1;
                    if speech_end > speech_start
                        && (speech_end - speech_start) >= min_speech_windows
                    {
                        raw_speech_spans.push((speech_start, speech_end));
                    }
                    in_speech = false;
                }
            }
        }

        if in_speech {
            let speech_end = window_energies.len();
            if speech_end > speech_start && (speech_end - speech_start) >= min_speech_windows {
                raw_speech_spans.push((speech_start, speech_end));
            }
        }

        let pre_pad_sec = self.params.pre_padding_ms as f64 / 1000.0;
        let post_pad_sec = self.params.post_padding_ms as f64 / 1000.0;
        let total_duration_sec = samples.len() as f64 / self.sample_rate as f64;
        let total_rational = RationalTime::from_seconds(total_duration_sec, self.sample_rate);

        // Convert spans to padded TimeRange
        let mut speech_ranges: Vec<TimeRange> = Vec::new();
        for (start_win, end_win) in raw_speech_spans {
            let raw_start = start_win as f64 * window_duration_sec;
            let raw_end = end_win as f64 * window_duration_sec;

            let start_sec = (raw_start - pre_pad_sec).max(0.0);
            let end_sec = (raw_end + post_pad_sec).min(total_duration_sec);

            let start_rt = RationalTime::from_seconds(start_sec, self.sample_rate);
            let end_rt = RationalTime::from_seconds(end_sec, self.sample_rate);

            if end_rt > start_rt {
                speech_ranges.push(TimeRange::new(start_rt, end_rt - start_rt));
            }
        }

        // Merge overlapping speech ranges
        let mut merged_speech: Vec<TimeRange> = Vec::new();
        for r in speech_ranges {
            if let Some(last) = merged_speech.last_mut() {
                if r.start <= last.end_time() {
                    let new_end = last.end_time().max(r.end_time());
                    *last = TimeRange::new(last.start, new_end - last.start);
                    continue;
                }
            }
            merged_speech.push(r);
        }

        // Calculate complementary silence intervals
        let mut silence_ranges: Vec<TimeRange> = Vec::new();
        let mut cursor = RationalTime::ZERO;

        for sp in &merged_speech {
            if sp.start > cursor {
                let sil_dur = sp.start - cursor;
                let min_sil_dur = RationalTime::from_seconds(
                    self.params.min_silence_duration_ms as f64 / 1000.0,
                    1000,
                );
                if sil_dur >= min_sil_dur {
                    silence_ranges.push(TimeRange::new(cursor, sil_dur));
                }
            }
            cursor = cursor.max(sp.end_time());
        }

        if cursor < total_rational {
            let sil_dur = total_rational - cursor;
            let min_sil_dur = RationalTime::from_seconds(
                self.params.min_silence_duration_ms as f64 / 1000.0,
                1000,
            );
            if sil_dur >= min_sil_dur {
                silence_ranges.push(TimeRange::new(cursor, sil_dur));
            }
        }

        VadResult {
            speech_segments: merged_speech,
            silence_segments: silence_ranges,
        }
    }
}
