use crate::error::{MediaError, Result};
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::decoder::{AudioBuffer, VideoFrame};
use std::path::{Path, PathBuf};

pub struct FileDecoder {
    path: PathBuf,
    input: ffmpeg_next::format::context::Input,
    video_stream_index: Option<usize>,
    audio_stream_index: Option<usize>,
    video_decoder: Option<ffmpeg_next::codec::decoder::Video>,
    audio_decoder: Option<ffmpeg_next::codec::decoder::Audio>,
    scaler: Option<ffmpeg_next::software::scaling::Context>,
    resampler: Option<ffmpeg_next::software::resampling::Context>,
}

unsafe impl Send for FileDecoder {}

impl FileDecoder {
    pub fn open(path: &Path) -> Result<Self> {
        let input = ffmpeg_next::format::input(path).map_err(MediaError::Ffmpeg)?;

        let mut video_stream_index = None;
        let mut audio_stream_index = None;
        let mut video_decoder = None;
        let mut audio_decoder = None;

        for stream in input.streams() {
            let params = stream.parameters();
            match params.medium() {
                ffmpeg_next::media::Type::Video if video_stream_index.is_none() => {
                    video_stream_index = Some(stream.index());
                    if let Ok(ctx) = ffmpeg_next::codec::context::Context::from_parameters(params) {
                        if let Ok(dec) = ctx.decoder().video() {
                            video_decoder = Some(dec);
                        }
                    }
                }
                ffmpeg_next::media::Type::Audio if audio_stream_index.is_none() => {
                    audio_stream_index = Some(stream.index());
                    if let Ok(ctx) = ffmpeg_next::codec::context::Context::from_parameters(params) {
                        if let Ok(dec) = ctx.decoder().audio() {
                            audio_decoder = Some(dec);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
            input,
            video_stream_index,
            audio_stream_index,
            video_decoder,
            audio_decoder,
            scaler: None,
            resampler: None,
        })
    }

    pub fn decode_video_frame(&mut self, target_time: RationalTime) -> Result<VideoFrame> {
        let stream_idx = self
            .video_stream_index
            .ok_or_else(|| MediaError::NoVideoStream(self.path.clone()))?;

        let stream = self.input.stream(stream_idx).unwrap();
        let time_base = stream.time_base();
        let fps = stream.avg_frame_rate();
        let _rate = if fps.1 > 0 { (fps.0 / fps.1) as u32 } else { 30 };

        // Convert target_time to stream PTS
        let target_pts = (target_time.to_seconds() * (time_base.1 as f64 / time_base.0 as f64)).round() as i64;

        // Keyframe seek backward from target
        let _ = self.input.seek(target_pts, ..target_pts);

        let decoder = self
            .video_decoder
            .as_mut()
            .ok_or_else(|| MediaError::NoVideoStream(self.path.clone()))?;

        decoder.flush();

        let mut decoded_frame = ffmpeg_next::frame::Video::empty();
        let mut closest_frame = None;
        let mut min_diff = i64::MAX;

        for (stream, packet) in self.input.packets() {
            if stream.index() != stream_idx {
                continue;
            }

            if decoder.send_packet(&packet).is_ok() {
                while decoder.receive_frame(&mut decoded_frame).is_ok() {
                    let frame_pts = decoded_frame.pts().unwrap_or(0);
                    let diff = (frame_pts - target_pts).abs();

                    if diff < min_diff {
                        min_diff = diff;
                        closest_frame = Some(decoded_frame.clone());
                    }

                    if frame_pts >= target_pts {
                        break;
                    }
                }
            }

            if let Some(ref cf) = closest_frame {
                if cf.pts().unwrap_or(0) >= target_pts {
                    break;
                }
            }
        }

        let frame = closest_frame.ok_or(MediaError::FrameDecodeFailed { pts: target_pts })?;

        let width = frame.width();
        let height = frame.height();

        // Convert frame to RGBA8
        let mut rgb_frame = ffmpeg_next::frame::Video::empty();
        let scaler = self.scaler.get_or_insert_with(|| {
            ffmpeg_next::software::scaling::context::Context::get(
                frame.format(),
                width,
                height,
                ffmpeg_next::format::Pixel::RGBA,
                width,
                height,
                ffmpeg_next::software::scaling::flag::Flags::BILINEAR,
            )
            .expect("failed to create scaler")
        });

        scaler
            .run(&frame, &mut rgb_frame)
            .map_err(MediaError::Ffmpeg)?;

        let data = rgb_frame.data(0).to_vec();
        let stride = rgb_frame.stride(0);

        Ok(VideoFrame {
            pts: target_time,
            width,
            height,
            data,
            stride,
        })
    }

    pub fn decode_audio_range(&mut self, range: TimeRange) -> Result<AudioBuffer> {
        let stream_idx = self
            .audio_stream_index
            .ok_or_else(|| MediaError::NoAudioStream(self.path.clone()))?;

        let stream = self.input.stream(stream_idx).unwrap();
        let time_base = stream.time_base();
        let target_pts = (range.start.to_seconds() * (time_base.1 as f64 / time_base.0 as f64)).round() as i64;

        let _ = self.input.seek(target_pts, ..target_pts);

        let decoder = self
            .audio_decoder
            .as_mut()
            .ok_or_else(|| MediaError::NoAudioStream(self.path.clone()))?;

        decoder.flush();

        let target_sample_rate = 48000;
        let target_channels = 2;

        let mut resampler = self.resampler.take().unwrap_or_else(|| {
            ffmpeg_next::software::resampling::context::Context::get(
                decoder.format(),
                decoder.channel_layout(),
                decoder.rate(),
                ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
                ffmpeg_next::channel_layout::ChannelLayout::STEREO,
                target_sample_rate,
            )
            .expect("failed to create audio resampler")
        });

        let mut decoded_frame = ffmpeg_next::frame::Audio::empty();
        let mut collected_samples = Vec::new();
        let target_sample_count = (range.duration.to_seconds() * target_sample_rate as f64 * target_channels as f64) as usize;

        for (stream, packet) in self.input.packets() {
            if stream.index() != stream_idx {
                continue;
            }

            if decoder.send_packet(&packet).is_ok() {
                while decoder.receive_frame(&mut decoded_frame).is_ok() {
                    let mut resampled_frame = ffmpeg_next::frame::Audio::empty();
                    if resampler.run(&decoded_frame, &mut resampled_frame).is_ok() {
                        let data = resampled_frame.data(0);
                        let slice: &[f32] = unsafe {
                            std::slice::from_raw_parts(
                                data.as_ptr() as *const f32,
                                data.len() / std::mem::size_of::<f32>(),
                            )
                        };
                        collected_samples.extend_from_slice(slice);
                    }

                    if collected_samples.len() >= target_sample_count {
                        break;
                    }
                }
            }

            if collected_samples.len() >= target_sample_count {
                break;
            }
        }

        self.resampler = Some(resampler);

        Ok(AudioBuffer {
            sample_rate: target_sample_rate,
            channels: target_channels as u16,
            samples: collected_samples,
        })
    }
}
