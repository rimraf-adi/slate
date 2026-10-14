use crate::cli::InfoArgs;
use anyhow::{Context, Result};
use slate_media::MediaProbe;

pub fn execute(args: InfoArgs, json: bool) -> Result<()> {
    slate_media::init().context("Failed to initialize FFmpeg subsystem")?;

    let meta = MediaProbe::probe(&args.path)
        .with_context(|| format!("Failed to probe media file: {:?}", args.path))?;

    if json {
        let out = serde_json::json!({
            "status": "success",
            "file": args.path,
            "metadata": meta
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("File: {}", args.path.display());
        println!("Size: {:.2} MB", meta.file_size_bytes as f64 / 1_048_576.0);
        println!("Duration: {:.2}s ({})", meta.duration.to_seconds(), meta.duration);

        if let (Some(w), Some(h)) = (meta.width, meta.height) {
            println!("Video: {}x{} ({:?})", w, h, meta.video_codec.as_deref().unwrap_or("unknown"));
            if let Some(fps) = meta.frame_rate {
                println!("Frame Rate: {:.2} fps", fps);
            }
        }

        if let Some(ref acodec) = meta.audio_codec {
            println!(
                "Audio: {} ({} ch, {} Hz)",
                acodec,
                meta.audio_channels.unwrap_or(0),
                meta.sample_rate.unwrap_or(0)
            );
        }
    }

    Ok(())
}
