use crate::decoder::worker::FileDecoder;
use crate::error::{MediaError, Result};
use image::{ImageBuffer, Rgba};
use slate_core::time::{RationalTime, TimeRange};
use std::path::Path;

pub struct ThumbnailGenerator;

impl ThumbnailGenerator {
    /// Extracts a single frame thumbnail at `time` and returns raw RGBA image bytes (width, height, bytes).
    pub fn extract_frame(path: &Path, time: RationalTime, max_width: u32) -> Result<(u32, u32, Vec<u8>)> {
        let mut decoder = FileDecoder::open(path)?;
        let frame = decoder.decode_video_frame(time)?;

        let orig_w = frame.width;
        let orig_h = frame.height;

        let aspect = orig_w as f32 / orig_h as f32;
        let target_w = max_width.min(orig_w);
        let target_h = (target_w as f32 / aspect).round() as u32;

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(orig_w, orig_h, frame.data)
            .ok_or_else(|| MediaError::ImageError("Failed to create image from raw frame buffer".into()))?;

        let resized = image::imageops::resize(
            &img,
            target_w,
            target_h,
            image::imageops::FilterType::Triangle,
        );

        Ok((target_w, target_h, resized.into_raw()))
    }

    /// Generates a contact sheet grid of N thumbnails evenly spaced across `range`.
    pub fn generate_contact_sheet(
        path: &Path,
        range: TimeRange,
        columns: u32,
        rows: u32,
        thumb_width: u32,
    ) -> Result<(u32, u32, Vec<u8>)> {
        let mut decoder = FileDecoder::open(path)?;
        let total_thumbs = columns * rows;
        if total_thumbs == 0 {
            return Err(MediaError::ImageError("Grid dimensions must be > 0".into()));
        }

        let step_seconds = range.duration.to_seconds() / (total_thumbs as f64);
        let rate = range.start.rate;

        let mut thumbnails = Vec::new();
        let mut thumb_height = (thumb_width as f32 * 9.0 / 16.0).round() as u32;

        for i in 0..total_thumbs {
            let offset_secs = step_seconds * (i as f64 + 0.5);
            let frame_time = range.start + RationalTime::from_seconds(offset_secs, rate);

            if let Ok(frame) = decoder.decode_video_frame(frame_time) {
                let orig_w = frame.width;
                let orig_h = frame.height;
                thumb_height = (thumb_width as f32 * (orig_h as f32 / orig_w as f32)).round() as u32;

                if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(orig_w, orig_h, frame.data) {
                    let resized = image::imageops::resize(
                        &img,
                        thumb_width,
                        thumb_height,
                        image::imageops::FilterType::Nearest,
                    );
                    thumbnails.push(resized);
                    continue;
                }
            }

            // Fallback black thumbnail
            thumbnails.push(ImageBuffer::from_pixel(thumb_width, thumb_height, Rgba([0, 0, 0, 255])));
        }

        // Composite thumbnails into single contact sheet
        let sheet_width = columns * thumb_width;
        let sheet_height = rows * thumb_height;
        let mut sheet = ImageBuffer::from_pixel(sheet_width, sheet_height, Rgba([0, 0, 0, 255]));

        for (idx, thumb) in thumbnails.into_iter().enumerate() {
            let col = (idx as u32) % columns;
            let row = (idx as u32) / columns;
            let x = col * thumb_width;
            let y = row * thumb_height;

            image::imageops::overlay(&mut sheet, &thumb, x as i64, y as i64);
        }

        Ok((sheet_width, sheet_height, sheet.into_raw()))
    }
}
