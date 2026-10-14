#[derive(Debug, Clone, PartialEq)]
pub struct ColorHistogram {
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
}

impl ColorHistogram {
    pub fn from_rgba_frame(rgba: &[u8], num_bins: usize) -> Self {
        let bins = num_bins.max(4);
        let mut r_hist = vec![0.0f32; bins];
        let mut g_hist = vec![0.0f32; bins];
        let mut b_hist = vec![0.0f32; bins];

        let num_pixels = rgba.len() / 4;
        if num_pixels == 0 {
            return Self {
                r: r_hist,
                g: g_hist,
                b: b_hist,
            };
        }

        let bin_width = 256.0 / bins as f32;

        for pixel in rgba.chunks_exact(4) {
            let r_bin = ((pixel[0] as f32 / bin_width) as usize).min(bins - 1);
            let g_bin = ((pixel[1] as f32 / bin_width) as usize).min(bins - 1);
            let b_bin = ((pixel[2] as f32 / bin_width) as usize).min(bins - 1);

            r_hist[r_bin] += 1.0;
            g_hist[g_bin] += 1.0;
            b_hist[b_bin] += 1.0;
        }

        // Normalize
        let total = num_pixels as f32;
        for i in 0..bins {
            r_hist[i] /= total;
            g_hist[i] /= total;
            b_hist[i] /= total;
        }

        Self {
            r: r_hist,
            g: g_hist,
            b: b_hist,
        }
    }
}
