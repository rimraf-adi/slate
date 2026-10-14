use crate::scene::hist::ColorHistogram;

/// Computes normalized Chi-Squared distance between two color histograms.
/// Returns a value between 0.0 (identical) and 1.0 (completely disjoint).
pub fn chi_squared_distance(h1: &ColorHistogram, h2: &ColorHistogram) -> f32 {
    fn channel_dist(c1: &[f32], c2: &[f32]) -> f32 {
        let mut dist = 0.0f32;
        for (&v1, &v2) in c1.iter().zip(c2.iter()) {
            let sum = v1 + v2;
            if sum > 1e-7 {
                let diff = v1 - v2;
                dist += (diff * diff) / sum;
            }
        }
        dist * 0.5
    }

    let d_r = channel_dist(&h1.r, &h2.r);
    let d_g = channel_dist(&h1.g, &h2.g);
    let d_b = channel_dist(&h1.b, &h2.b);

    ((d_r + d_g + d_b) / 3.0).min(1.0)
}
