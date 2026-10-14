use image::{imageops, Rgba, RgbaImage};

use crate::error::AnalysisError;
use crate::types::ContactSheetParams;

pub struct ContactSheetGenerator {
    params: ContactSheetParams,
}

impl ContactSheetGenerator {
    pub fn new(params: ContactSheetParams) -> Self {
        Self { params }
    }

    /// Stitches a list of RGBA image frames into an (NxM) contact sheet.
    pub fn generate(&self, thumbnails: &[RgbaImage]) -> Result<RgbaImage, AnalysisError> {
        let cols = self.params.cols.max(1);
        let rows = self.params.rows.max(1);
        let thumb_w = self.params.thumb_width.max(32);
        let thumb_h = self.params.thumb_height.max(32);
        let margin = 4u32;

        let total_w = cols as u32 * thumb_w + (cols as u32 + 1) * margin;
        let total_h = rows as u32 * thumb_h + (rows as u32 + 1) * margin;

        let mut sheet = RgbaImage::from_pixel(total_w, total_h, Rgba([20, 20, 25, 255]));

        for (idx, thumb) in thumbnails.iter().enumerate().take(cols * rows) {
            let col = (idx % cols) as u32;
            let row = (idx / cols) as u32;

            let x = margin + col * (thumb_w + margin);
            let y = margin + row * (thumb_h + margin);

            // Resize thumbnail to target cell size if needed
            let resized = if thumb.width() != thumb_w || thumb.height() != thumb_h {
                imageops::resize(thumb, thumb_w, thumb_h, imageops::FilterType::Triangle)
            } else {
                thumb.clone()
            };

            imageops::overlay(&mut sheet, &resized, x as i64, y as i64);
        }

        Ok(sheet)
    }
}
