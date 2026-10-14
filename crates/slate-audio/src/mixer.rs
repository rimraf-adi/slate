use crate::dsp::{db_to_linear, equal_power_pan, soft_clip};
use slate_core::traits::decoder::AudioBuffer;
use slate_core::traits::mixer::{AudioError, AudioMixer, MixRequest};

pub struct SoftwareAudioMixer {
    target_sample_rate: u32,
    target_channels: u16,
}

impl SoftwareAudioMixer {
    pub fn new() -> Self {
        Self {
            target_sample_rate: 48000,
            target_channels: 2,
        }
    }

    pub fn with_format(sample_rate: u32, channels: u16) -> Self {
        Self {
            target_sample_rate: sample_rate,
            target_channels: channels,
        }
    }
}

impl Default for SoftwareAudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioMixer for SoftwareAudioMixer {
    fn mix(&self, request: &MixRequest) -> Result<AudioBuffer, AudioError> {
        let total_frames = (request.range.duration.to_seconds() * self.target_sample_rate as f64).round() as usize;
        let total_samples = total_frames * self.target_channels as usize;

        let mut master_buffer = vec![0.0f32; total_samples];
        let master_gain = db_to_linear(request.master_volume_db);

        for (_track_id, track_buffer, volume_db, pan) in &request.track_streams {
            let track_gain = db_to_linear(*volume_db);
            let (pan_l, pan_r) = equal_power_pan(*pan);

            let gain_l = track_gain * pan_l;
            let gain_r = track_gain * pan_r;

            let num_frames = (track_buffer.samples.len() / track_buffer.channels as usize).min(total_frames);

            if track_buffer.channels == 1 {
                // Mono track -> panned to stereo
                for f in 0..num_frames {
                    let mono_sample = track_buffer.samples[f];
                    master_buffer[f * 2] += mono_sample * gain_l;
                    master_buffer[f * 2 + 1] += mono_sample * gain_r;
                }
            } else if track_buffer.channels >= 2 {
                // Stereo track
                for f in 0..num_frames {
                    let left = track_buffer.samples[f * track_buffer.channels as usize];
                    let right = track_buffer.samples[f * track_buffer.channels as usize + 1];

                    master_buffer[f * 2] += left * gain_l;
                    master_buffer[f * 2 + 1] += right * gain_r;
                }
            }
        }

        // Apply master volume and soft-clipping
        for sample in &mut master_buffer {
            let scaled = *sample * master_gain;
            *sample = soft_clip(scaled);
        }

        Ok(AudioBuffer {
            sample_rate: self.target_sample_rate,
            channels: self.target_channels,
            samples: master_buffer,
        })
    }
}
