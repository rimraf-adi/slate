use crate::error::{MediaError, Result};
use slate_core::model::MediaMetadata;
use slate_core::time::RationalTime;
use std::path::Path;

pub struct MediaProbe;

impl MediaProbe {
    /// Probes the file at `path` using FFmpeg demuxer and returns parsed `MediaMetadata`.
    pub fn probe(path: &Path) -> Result<MediaMetadata> {
        if !path.exists() {
            return Err(MediaError::FileNotFound(path.to_path_buf()));
        }

        let file_size_bytes = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(0);

        let ictx = ffmpeg_next::format::input(path).map_err(|e| MediaError::Ffmpeg(e))?;

        let mut duration_secs = 0.0;
        if ictx.duration() > 0 {
            duration_secs = ictx.duration() as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64;
        }

        let mut video_codec = None;
        let mut width = None;
        let mut height = None;
        let mut frame_rate = None;
        let mut timebase = 30;

        let mut audio_codec = None;
        let mut audio_channels = None;
        let mut sample_rate = None;

        for stream in ictx.streams() {
            let codec_params = stream.parameters();
            match codec_params.medium() {
                ffmpeg_next::media::Type::Video if video_codec.is_none() => {
                    let decoder = ffmpeg_next::codec::context::Context::from_parameters(codec_params.clone())
                        .ok()
                        .and_then(|ctx| ctx.decoder().video().ok());

                    if let Some(vid) = decoder {
                        width = Some(vid.width());
                        height = Some(vid.height());
                        let avg_fps = stream.avg_frame_rate();
                        if avg_fps.1 > 0 {
                            let fps = avg_fps.0 as f64 / avg_fps.1 as f64;
                            if fps > 0.0 && fps < 1000.0 {
                                frame_rate = Some(fps);
                                timebase = (fps.round() as u32).max(1);
                            }
                        }
                    }

                    let id = codec_params.id();
                    video_codec = Some(format!("{:?}", id).to_lowercase());

                    let stream_dur = stream.duration();
                    if stream_dur > 0 && duration_secs == 0.0 {
                        let tb = stream.time_base();
                        if tb.1 > 0 {
                            duration_secs = stream_dur as f64 * (tb.0 as f64 / tb.1 as f64);
                        }
                    }
                }
                ffmpeg_next::media::Type::Audio if audio_codec.is_none() => {
                    let decoder = ffmpeg_next::codec::context::Context::from_parameters(codec_params.clone())
                        .ok()
                        .and_then(|ctx| ctx.decoder().audio().ok());

                    if let Some(aud) = decoder {
                        audio_channels = Some(aud.channels() as u32);
                        sample_rate = Some(aud.rate() as u32);
                    }

                    let id = codec_params.id();
                    audio_codec = Some(format!("{:?}", id).to_lowercase());
                }
                _ => {}
            }
        }

        let duration = RationalTime::from_seconds(duration_secs.max(0.0), timebase);

        Ok(MediaMetadata {
            file_size_bytes,
            duration,
            video_codec,
            width,
            height,
            frame_rate,
            timebase,
            audio_codec,
            audio_channels,
            sample_rate,
        })
    }
}
