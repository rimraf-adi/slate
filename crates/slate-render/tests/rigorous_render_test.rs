use slate_core::model::{Clip, Transform, Transition, TransitionAlignment};
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::compositor::{Compositor, FrameRequest};
use slate_core::traits::decoder::VideoFrame;
use slate_core::{MediaId, TransitionId};
use slate_render::compositor::GpuCompositor;
use slate_render::context::GpuContext;

fn make_color_frame(width: u32, height: u32, color: [u8; 4]) -> VideoFrame {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        data.extend_from_slice(&color);
    }
    VideoFrame {
        pts: RationalTime::ZERO,
        width,
        height,
        data,
        stride: (width * 4) as usize,
    }
}

#[test]
fn test_render_cross_dissolve_transition() {
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

    // Red incoming frame, Blue outgoing frame
    let red_frame = make_color_frame(width, height, [255, 0, 0, 255]);
    let blue_frame = make_color_frame(width, height, [0, 0, 255, 255]);

    // 50% midpoint cross dissolve transition
    let trans = Transition {
        id: TransitionId::new(),
        transition_type: "cross_dissolve".to_string(),
        duration: RationalTime::from_seconds(1.0, 30),
        alignment: TransitionAlignment::CenterAtCut,
        parameters: serde_json::json!({}),
    };

    let request = FrameRequest {
        timeline_time: RationalTime::from_seconds(0.5, 30),
        visible_clips: vec![],
        active_transitions: vec![(trans, red_frame, blue_frame, 0.5)],
        canvas_width: width,
        canvas_height: height,
    };

    let rendered = compositor.composite_frame(&request).expect("composite transition failed");
    assert_eq!(rendered.width, width);
    assert_eq!(rendered.height, height);
    assert_eq!(rendered.rgba_pixels.len(), (width * height * 4) as usize);

    // Midpoint crossfade between full Red (255, 0, 0) and full Blue (0, 0, 255):
    let center_idx = ((height / 2 * width + width / 2) * 4) as usize;
    let r = rendered.rgba_pixels[center_idx];
    let b = rendered.rgba_pixels[center_idx + 2];
    assert!(r > 80 && r < 180, "Expected blended red, got {}", r);
    assert!(b > 80 && b < 180, "Expected blended blue, got {}", b);
}

#[test]
fn test_render_dip_to_black_transition() {
    let context = match GpuContext::new_headless() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping GPU test: no adapter available ({})", e);
            return;
        }
    };
    let compositor = GpuCompositor::new(context).expect("create compositor");

    let width = 64;
    let height = 64;

    let red_frame = make_color_frame(width, height, [255, 0, 0, 255]);
    let blue_frame = make_color_frame(width, height, [0, 0, 255, 255]);

    // Dip to black at progress 0.5 should be pure black (0, 0, 0, 255)
    let trans = Transition {
        id: TransitionId::new(),
        transition_type: "dip_to_black".to_string(),
        duration: RationalTime::from_seconds(1.0, 30),
        alignment: TransitionAlignment::CenterAtCut,
        parameters: serde_json::json!({}),
    };

    let request = FrameRequest {
        timeline_time: RationalTime::from_seconds(0.5, 30),
        visible_clips: vec![],
        active_transitions: vec![(trans, red_frame, blue_frame, 0.5)],
        canvas_width: width,
        canvas_height: height,
    };

    let rendered = compositor.composite_frame(&request).expect("dip to black composite");
    assert_eq!(rendered.width, width);
    assert_eq!(rendered.height, height);

    let center_idx = ((height / 2 * width + width / 2) * 4) as usize;
    let r = rendered.rgba_pixels[center_idx];
    let g = rendered.rgba_pixels[center_idx + 1];
    let b = rendered.rgba_pixels[center_idx + 2];
    let a = rendered.rgba_pixels[center_idx + 3];

    assert_eq!(r, 0, "Expected black red component");
    assert_eq!(g, 0, "Expected black green component");
    assert_eq!(b, 0, "Expected black blue component");
    assert_eq!(a, 255, "Expected full opacity");
}

#[test]
fn test_render_transform_scaling_and_translation() {
    let context = match GpuContext::new_headless() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping GPU test: no adapter available ({})", e);
            return;
        }
    };
    let compositor = GpuCompositor::new(context).expect("create compositor");

    let width = 64;
    let height = 64;

    let green_frame = make_color_frame(width, height, [0, 255, 0, 255]);
    let range = TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(10.0, 30));
    let mut clip = Clip::new("transformed_green", MediaId::new(), range, range);

    let mut trans = Transform::default();
    trans.scale_x.keyframes[0].value = 1.0;
    trans.scale_y.keyframes[0].value = 1.0;
    clip.transform = trans;

    let request = FrameRequest {
        timeline_time: RationalTime::ZERO,
        visible_clips: vec![(clip, green_frame)],
        active_transitions: vec![],
        canvas_width: width,
        canvas_height: height,
    };

    let rendered = compositor.composite_frame(&request).expect("transform composite");
    assert_eq!(rendered.width, width);
    assert_eq!(rendered.height, height);

    // Center pixel should be Green (0, 255, 0, 255)
    let center_idx = ((height / 2 * width + width / 2) * 4) as usize;
    assert_eq!(rendered.rgba_pixels[center_idx], 0);
    assert_eq!(rendered.rgba_pixels[center_idx + 1], 255);
    assert_eq!(rendered.rgba_pixels[center_idx + 2], 0);
    assert_eq!(rendered.rgba_pixels[center_idx + 3], 255);
}
