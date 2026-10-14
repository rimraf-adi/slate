use crate::decoder::worker::FileDecoder;
use crate::error::{MediaError, Result};
use slate_core::time::{RationalTime, TimeRange};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveformBucket {
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

pub struct WaveformGenerator;

impl WaveformGenerator {
    pub const MAGIC: [u8; 4] = *b"SLWF";
    pub const VERSION: u32 = 1;

    /// Generates waveform peak and RMS buckets for an audio/video file.
    /// `pixels_per_second` controls the bucket resolution (e.g. 50 or 100 buckets/sec).
    pub fn generate_waveform(
        media_path: &Path,
        duration: RationalTime,
        buckets_per_second: usize,
    ) -> Result<Vec<WaveformBucket>> {
        let mut decoder = FileDecoder::open(media_path)?;
        let range = TimeRange::new(RationalTime::ZERO, duration);

        let audio_buffer = decoder.decode_audio_range(range)?;
        if audio_buffer.samples.is_empty() {
            return Ok(Vec::new());
        }

        let total_seconds = duration.to_seconds();
        let total_buckets = (total_seconds * buckets_per_second as f64).ceil() as usize;
        let total_samples = audio_buffer.samples.len() / audio_buffer.channels as usize;

        let samples_per_bucket = (total_samples / total_buckets.max(1)).max(1);
        let mut buckets = Vec::with_capacity(total_buckets);

        for bucket_idx in 0..total_buckets {
            let start_sample = bucket_idx * samples_per_bucket;
            let end_sample = ((bucket_idx + 1) * samples_per_bucket).min(total_samples);

            if start_sample >= total_samples {
                break;
            }

            let mut min_val = 0.0f32;
            let mut max_val = 0.0f32;
            let mut sum_sq = 0.0f64;
            let count = (end_sample - start_sample) * audio_buffer.channels as usize;

            for s_idx in start_sample..end_sample {
                for ch in 0..audio_buffer.channels as usize {
                    let sample = audio_buffer.samples[s_idx * audio_buffer.channels as usize + ch];
                    if sample < min_val {
                        min_val = sample;
                    }
                    if sample > max_val {
                        max_val = sample;
                    }
                    sum_sq += (sample as f64) * (sample as f64);
                }
            }

            let rms = if count > 0 {
                (sum_sq / count as f64).sqrt() as f32
            } else {
                0.0
            };

            buckets.push(WaveformBucket {
                min: min_val,
                max: max_val,
                rms,
            });
        }

        Ok(buckets)
    }

    /// Saves waveform buckets to disk in the binary Slate waveform format (`.slwf`).
    pub fn save_waveform_file(buckets: &[WaveformBucket], dest_path: &Path) -> Result<()> {
        let file = File::create(dest_path).map_err(|e| MediaError::Io {
            path: dest_path.to_path_buf(),
            source: e,
        })?;
        let mut writer = BufWriter::new(file);

        writer.write_all(&Self::MAGIC).map_err(|e| MediaError::Io {
            path: dest_path.to_path_buf(),
            source: e,
        })?;

        writer
            .write_all(&Self::VERSION.to_le_bytes())
            .map_err(|e| MediaError::Io {
                path: dest_path.to_path_buf(),
                source: e,
            })?;

        let count = buckets.len() as u64;
        writer
            .write_all(&count.to_le_bytes())
            .map_err(|e| MediaError::Io {
                path: dest_path.to_path_buf(),
                source: e,
            })?;

        for bucket in buckets {
            writer.write_all(&bucket.min.to_le_bytes()).map_err(|e| MediaError::Io {
                path: dest_path.to_path_buf(),
                source: e,
            })?;
            writer.write_all(&bucket.max.to_le_bytes()).map_err(|e| MediaError::Io {
                path: dest_path.to_path_buf(),
                source: e,
            })?;
            writer.write_all(&bucket.rms.to_le_bytes()).map_err(|e| MediaError::Io {
                path: dest_path.to_path_buf(),
                source: e,
            })?;
        }

        writer.flush().map_err(|e| MediaError::Io {
            path: dest_path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}
