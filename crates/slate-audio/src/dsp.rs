use std::f32::consts::FRAC_PI_2;

/// Converts decibels to a linear amplitude multiplier.
/// (0 dB = 1.0, -6 dB ≈ 0.5, -inf dB = 0.0)
pub fn db_to_linear(db: f64) -> f32 {
    if db <= -96.0 {
        0.0
    } else {
        (10.0f64.powf(db / 20.0)) as f32
    }
}

/// Converts linear amplitude multiplier to decibels.
pub fn linear_to_db(linear: f32) -> f64 {
    if linear <= 1e-5 {
        -100.0
    } else {
        20.0 * (linear as f64).log10()
    }
}

/// Calculates stereo gain factors using an equal-power (-3dB center) panning law.
/// `pan` range: [-1.0 (Full Left), 0.0 (Center), 1.0 (Full Right)].
pub fn equal_power_pan(pan: f64) -> (f32, f32) {
    let clamped_pan = pan.clamp(-1.0, 1.0) as f32;
    // Normalized angle [0.0, 1.0]
    let theta = (clamped_pan + 1.0) * 0.5 * FRAC_PI_2;

    let left_gain = theta.cos();
    let right_gain = theta.sin();

    (left_gain, right_gain)
}

/// Soft-clipping saturation function (cubic smoothstep) to prevent harsh digital clipping.
/// For |x| <= 2/3, response is completely linear.
/// For |x| > 2/3, it compresses smoothly asymptotically towards +/- 1.0.
pub fn soft_clip(sample: f32) -> f32 {
    if sample > 1.0 {
        1.0
    } else if sample < -1.0 {
        -1.0
    } else if sample > (2.0 / 3.0) {
        (3.0 - (2.0 - 3.0 * sample).powi(2)) / 3.0
    } else if sample < -(2.0 / 3.0) {
        -(3.0 - (2.0 + 3.0 * sample).powi(2)) / 3.0
    } else {
        sample
    }
}
