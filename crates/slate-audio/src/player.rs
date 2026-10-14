use crate::clock::PlaybackClock;
use crate::error::{AudioError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use rtrb::{Producer, RingBuffer};
use std::sync::Arc;

pub struct AudioPlayer {
    clock: Arc<PlaybackClock>,
    producer: Producer<f32>,
    stream: Option<Stream>,
    config: StreamConfig,
}

impl AudioPlayer {
    /// Creates a new AudioPlayer connected to the default system audio output.
    pub fn new(clock: Arc<PlaybackClock>, ring_buffer_capacity: usize) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::DeviceError("No audio output device found".into()))?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        let (producer, mut consumer) = RingBuffer::<f32>::new(ring_buffer_capacity);

        let clock_clone = clock.clone();
        let channels = config.channels as usize;

        let err_fn = |err| {
            tracing::error!("CPAL audio stream error: {}", err);
        };

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let is_playing = clock_clone.is_playing();
                    let mut frames_read = 0;

                    for sample in data.iter_mut() {
                        if is_playing {
                            if let Ok(val) = consumer.pop() {
                                *sample = val;
                                frames_read += 1;
                            } else {
                                *sample = 0.0;
                            }
                        } else {
                            *sample = 0.0;
                        }
                    }

                    if is_playing && channels > 0 {
                        clock_clone.advance_samples(frames_read / channels);
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let is_playing = clock_clone.is_playing();
                    let mut frames_read = 0;

                    for sample in data.iter_mut() {
                        if is_playing {
                            if let Ok(val) = consumer.pop() {
                                *sample = (val.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                                frames_read += 1;
                            } else {
                                *sample = 0;
                            }
                        } else {
                            *sample = 0;
                        }
                    }

                    if is_playing && channels > 0 {
                        clock_clone.advance_samples(frames_read / channels);
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    for sample in data.iter_mut() {
                        *sample = u16::MAX / 2;
                    }
                },
                err_fn,
                None,
            ),
            _ => return Err(AudioError::StreamError("Unsupported sample format".into())),
        }
        .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        Ok(Self {
            clock,
            producer,
            stream: Some(stream),
            config,
        })
    }

    /// Pushes interleaved float samples into the lock-free ring buffer.
    pub fn push_samples(&mut self, samples: &[f32]) -> Result<usize> {
        let mut pushed = 0;
        for &sample in samples {
            if self.producer.push(sample).is_ok() {
                pushed += 1;
            } else {
                break;
            }
        }
        Ok(pushed)
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    pub fn clock(&self) -> &Arc<PlaybackClock> {
        &self.clock
    }

    pub fn pause_stream(&self) -> Result<()> {
        if let Some(ref s) = self.stream {
            s.pause().map_err(|e| AudioError::StreamError(e.to_string()))?;
        }
        Ok(())
    }

    pub fn play_stream(&self) -> Result<()> {
        if let Some(ref s) = self.stream {
            s.play().map_err(|e| AudioError::StreamError(e.to_string()))?;
        }
        Ok(())
    }
}
