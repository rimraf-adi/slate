use crate::cli::ThumbnailArgs;
use anyhow::{Context, Result};
use slate_core::time::RationalTime;
use slate_media::ThumbnailGenerator;

pub fn execute(args: ThumbnailArgs, json: bool) -> Result<()> {
    slate_media::init().context("Failed to initialize FFmpeg subsystem")?;

    let time = RationalTime::from_seconds(args.time, 30);
    let (thumb_w, thumb_h, rgba_data) = ThumbnailGenerator::extract_frame(&args.file, time, args.width)
        .with_context(|| format!("Failed to generate thumbnail for {:?}", args.file))?;

    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image::ImageBuffer::from_raw(thumb_w, thumb_h, rgba_data)
        .ok_or_else(|| anyhow::anyhow!("Failed to create image from thumbnail buffer"))?;

    img.save(&args.output)
        .with_context(|| format!("Failed to save thumbnail image to {:?}", args.output))?;

    if json {
        let out = serde_json::json!({
            "status": "success",
            "file": args.file,
            "output": args.output,
            "time_seconds": args.time,
            "width": args.width,
            "height": args.height
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "✓ Saved thumbnail at {:.2}s ({}x{}) to {}",
            args.time,
            args.width,
            args.height,
            args.output.display()
        );
    }

    Ok(())
}
