use slate_core::model::ExportPreset;
use slate_core::time::RationalTime;
use slate_core::traits::decoder::VideoFrame;
use slate_media::encoder::ExportSession;
use slate_media::probe::MediaProbe;
use slate_media::thumbnail::ThumbnailGenerator;
use tempfile::tempdir;

#[test]
fn test_export_and_probe_pipeline() {
    slate_media::init().expect("FFmpeg init failed");

    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("render_test.mp4");

    let preset = ExportPreset {
        name: "Test 360p30".into(),
        container: "mp4".into(),
        video_codec: "h264".into(),
        audio_codec: "aac".into(),
        width: 320,
        height: 180,
        fps: 30.0,
        video_bitrate_kbps: 1000,
        audio_bitrate_kbps: 128,
    };

    let mut session = ExportSession::start(&output_path, &preset).expect("ExportSession start");

    // Write 30 frames (1 second of solid color frames)
    let frame_bytes = (320 * 180 * 4) as usize;
    let mut frame_data = vec![0u8; frame_bytes];
    // Fill with semi-transparent cyan
    for chunk in frame_data.chunks_exact_mut(4) {
        chunk[0] = 0;   // R
        chunk[1] = 200; // G
        chunk[2] = 255; // B
        chunk[3] = 255; // A
    }

    for i in 0..30 {
        let frame = VideoFrame {
            pts: RationalTime::from_frames(i, 30),
            width: 320,
            height: 180,
            data: frame_data.clone(),
            stride: 320 * 4,
        };
        session.write_video_frame(&frame).expect("write frame failed");
    }

    session.finish().expect("Export finish failed");

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path).unwrap().len() > 0);

    // Now probe the generated file
    let metadata = MediaProbe::probe(&output_path).expect("probe failed");
    assert_eq!(metadata.width, Some(320));
    assert_eq!(metadata.height, Some(180));
    assert!(metadata.duration.to_seconds() >= 0.9);

    // Test thumbnail extraction
    let (tw, th, tdata) = ThumbnailGenerator::extract_frame(
        &output_path,
        RationalTime::from_frames(10, 30),
        160,
    )
    .expect("thumbnail extract failed");

    assert_eq!(tw, 160);
    assert_eq!(th, 90);
    assert_eq!(tdata.len(), (160 * 90 * 4) as usize);
}
