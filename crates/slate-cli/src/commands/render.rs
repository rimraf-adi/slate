use crate::cli::RenderArgs;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use slate_core::model::{ExportPreset, TrackItem, TrackType};
use slate_core::project::ProjectBundle;
use slate_core::time::RationalTime;
use slate_core::traits::compositor::{Compositor, FrameRequest};
use slate_core::traits::decoder::{MediaDecoder, VideoFrame};
use slate_media::{ExportSession, FfmpegMediaDecoder};
use slate_render::compositor::GpuCompositor;
use slate_render::context::GpuContext;
use std::collections::HashMap;

pub fn execute(args: RenderArgs, json: bool) -> Result<()> {
    slate_media::init().context("Failed to initialize FFmpeg subsystem")?;

    let project = ProjectBundle::load_from_bundle(&args.project)
        .with_context(|| format!("Failed to open project bundle at {:?}", args.project))?;

    let timeline = project
        .timelines
        .iter()
        .find(|t| t.id == project.active_timeline_id)
        .ok_or_else(|| anyhow::anyhow!("Active timeline not found in project"))?;

    // Determine dimensions & preset
    let (width, height) = match args.preset.as_str() {
        "9:16" => (1080, 1920),
        "1:1" => (1080, 1080),
        _ => (timeline.settings.width, timeline.settings.height),
    };

    let timebase = timeline.settings.timebase.max(1);
    let fps = timebase as f64;
    let duration = timeline.duration();
    let total_frames = if duration.ticks == 0 {
        // Render 1 second minimum if empty
        timebase as u64
    } else {
        duration.rescale(timebase).to_frames().max(1) as u64
    };

    let preset = ExportPreset {
        name: format!("Export {}", args.preset),
        width,
        height,
        fps,
        video_codec: "h264".into(),
        video_bitrate_kbps: args.bitrate,
        audio_codec: "aac".into(),
        audio_bitrate_kbps: 320,
        container: "mp4".into(),
    };

    let gpu_ctx = GpuContext::new_headless()
        .map_err(|e| anyhow::anyhow!("Failed to initialize GPU context: {}", e))?;
    let compositor = GpuCompositor::new(gpu_ctx)
        .map_err(|e| anyhow::anyhow!("Failed to create GPU compositor: {}", e))?;

    let mut export_session = ExportSession::start(&args.output, &preset)
        .with_context(|| format!("Failed to start export session for {:?}", args.output))?;

    // Pre-open decoders for media pool
    let decoder_service = FfmpegMediaDecoder::new();
    let mut media_handles = HashMap::new();
    for media_ref in &project.media_pool {
        let path = if media_ref.original_path.exists() {
            &media_ref.original_path
        } else {
            let resolved = args.project.join(&media_ref.relative_path);
            if resolved.exists() {
                // Resolved relative to bundle
                &media_ref.relative_path
            } else {
                &media_ref.original_path
            }
        };

        if let Ok(handle) = decoder_service.open(path) {
            media_handles.insert(media_ref.id, handle);
        }
    }

    let progress_bar = if !json {
        let pb = ProgressBar::new(total_frames);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} frames ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let start_instant = std::time::Instant::now();

    for frame_idx in 0..total_frames {
        let timeline_time = RationalTime::new(frame_idx as i64, timebase);

        let mut visible_clips = Vec::new();

        // Collect visible video clips across video tracks
        for track in timeline.tracks.iter().filter(|t| t.track_type == TrackType::Video && !t.muted) {
            for item in &track.items {
                if let TrackItem::Clip(clip) = item {
                    if clip.timeline_range.contains_time(timeline_time) {
                        let offset = timeline_time - clip.timeline_range.start;
                        let source_time = clip.source_range.start + offset;

                        let frame = if let Some(&handle) = media_handles.get(&clip.media_id) {
                            decoder_service
                                .decode_frame(&handle, source_time)
                                .unwrap_or_else(|_| VideoFrame {
                                    pts: source_time,
                                    width,
                                    height,
                                    data: vec![0; (width * height * 4) as usize],
                                    stride: (width * 4) as usize,
                                })
                        } else {
                            VideoFrame {
                                pts: source_time,
                                width,
                                height,
                                data: vec![0; (width * height * 4) as usize],
                                stride: (width * 4) as usize,
                            }
                        };

                        visible_clips.push((clip.clone(), frame));
                    }
                }
            }
        }

        let frame_req = FrameRequest {
            timeline_time,
            visible_clips,
            active_transitions: vec![],
            canvas_width: width,
            canvas_height: height,
        };

        let rendered = compositor
            .composite_frame(&frame_req)
            .map_err(|e| anyhow::anyhow!("Compositing error at frame {}: {}", frame_idx, e))?;

        let export_frame = VideoFrame {
            pts: timeline_time,
            width: rendered.width,
            height: rendered.height,
            data: rendered.rgba_pixels,
            stride: (rendered.width * 4) as usize,
        };

        export_session
            .write_video_frame(&export_frame)
            .with_context(|| format!("Failed to write frame {} to encoder", frame_idx))?;

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    export_session
        .finish()
        .context("Failed to finalize video export container")?;

    let elapsed = start_instant.elapsed();
    let render_fps = total_frames as f64 / elapsed.as_secs_f64().max(0.001);

    if json {
        let out = serde_json::json!({
            "status": "success",
            "output": args.output,
            "total_frames": total_frames,
            "elapsed_seconds": elapsed.as_secs_f64(),
            "render_fps": render_fps,
            "resolution": format!("{}x{}", width, height)
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "✓ Rendered {} frames to {} in {:.2}s ({:.1} fps)",
            total_frames,
            args.output.display(),
            elapsed.as_secs_f64(),
            render_fps
        );
    }

    Ok(())
}
