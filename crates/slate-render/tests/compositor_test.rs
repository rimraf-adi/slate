use slate_core::model::{Clip, Interpolation, Keyframe, Transform};
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::compositor::{Compositor, FrameRequest};
use slate_core::traits::decoder::VideoFrame;
use slate_core::{KeyframeId, MediaId};
use slate_render::compositor::GpuCompositor;
use slate_render::context::GpuContext;

#[test]
fn test_offscreen_solid_render() {
    let context = match GpuContext::new_headless() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping GPU test: no adapter available ({})", e);
            return;
        }
    };

    let compositor = GpuCompositor::new(context).expect("Failed to create GpuCompositor");

    let width = 64;
    let height = 64;

    // Create a 64x64 solid red input frame (RGBA)
    let mut red_pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        red_pixels.extend_from_slice(&[255, 0, 0, 255]);
    }

    let video_frame = VideoFrame {
        pts: RationalTime::new(0, 1),
        width,
        height,
        data: red_pixels,
        stride: (width * 4) as usize,
    };

    let range = TimeRange::new(RationalTime::new(0, 1), RationalTime::new(10, 1));
    let clip = Clip::new("solid_red", MediaId::new(), range, range);

    let request = FrameRequest {
        timeline_time: RationalTime::new(1, 1),
        visible_clips: vec![(clip, video_frame)],
        active_transitions: vec![],
        canvas_width: width,
        canvas_height: height,
    };

    let rendered = compositor.composite_frame(&request).expect("composite_frame failed");

    assert_eq!(rendered.width, width);
    assert_eq!(rendered.height, height);
    assert_eq!(rendered.rgba_pixels.len(), (width * height * 4) as usize);

    // Verify center pixel is red (255, 0, 0, 255)
    let center_idx = ((height / 2 * width + width / 2) * 4) as usize;
    let r = rendered.rgba_pixels[center_idx];
    let g = rendered.rgba_pixels[center_idx + 1];
    let b = rendered.rgba_pixels[center_idx + 2];
    let a = rendered.rgba_pixels[center_idx + 3];

    assert_eq!((r, g, b, a), (255, 0, 0, 255), "Pixel mismatch at center");
}

#[test]
fn test_clip_layering_and_opacity() {
    let context = match GpuContext::new_headless() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping GPU test: no adapter available ({})", e);
            return;
        }
    };

    let compositor = GpuCompositor::new(context).expect("Failed to create GpuCompositor");

    let width = 64;
    let height = 64;

    // Red background frame
    let mut red_pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        red_pixels.extend_from_slice(&[255, 0, 0, 255]);
    }
    let red_frame = VideoFrame {
        pts: RationalTime::new(0, 1),
        width,
        height,
        data: red_pixels,
        stride: (width * 4) as usize,
    };

    // Blue overlay frame
    let mut blue_pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        blue_pixels.extend_from_slice(&[0, 0, 255, 255]);
    }
    let blue_frame = VideoFrame {
        pts: RationalTime::new(0, 1),
        width,
        height,
        data: blue_pixels,
        stride: (width * 4) as usize,
    };

    let range = TimeRange::new(RationalTime::new(0, 1), RationalTime::new(10, 1));
    let clip1 = Clip::new("red", MediaId::new(), range, range);

    let mut clip2 = Clip::new("blue", MediaId::new(), range, range);
    let mut trans2 = Transform::default();
    trans2.opacity.keyframes.push(Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::new(0, 1),
        value: 0.5,
        interpolation: Interpolation::Linear,
    });
    clip2.transform = trans2;

    let request = FrameRequest {
        timeline_time: RationalTime::new(1, 1),
        visible_clips: vec![(clip1, red_frame), (clip2, blue_frame)],
        active_transitions: vec![],
        canvas_width: width,
        canvas_height: height,
    };

    let rendered = compositor.composite_frame(&request).expect("composite_frame failed");

    // Center pixel should have both Red and Blue mixed due to 50% opacity alpha blending
    let center_idx = ((height / 2 * width + width / 2) * 4) as usize;
    let r = rendered.rgba_pixels[center_idx];
    let g = rendered.rgba_pixels[center_idx + 1];
    let b = rendered.rgba_pixels[center_idx + 2];
    let a = rendered.rgba_pixels[center_idx + 3];

    // Blended Red and Blue:
    // Red back: [255, 0, 0, 255]
    // Overlaid with Blue at 50% opacity:
    assert!(r > 100 && r < 160, "Expected blended red ~128, got {}", r);
    assert_eq!(g, 0, "Expected green 0, got {}", g);
    assert!(b > 100 && b < 160, "Expected blended blue ~128, got {}", b);
    assert_eq!(a, 255, "Expected alpha 255, got {}", a);
}
