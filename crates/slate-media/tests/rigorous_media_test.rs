use slate_core::model::ExportPreset;
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::decoder::VideoFrame;
use slate_media::audio::waveform::{WaveformBucket, WaveformGenerator};
use slate_media::cache::{CacheKey, FrameCache};
use slate_media::encoder::ExportSession;
use slate_media::thumbnail::ThumbnailGenerator;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use tempfile::tempdir;

#[test]
fn test_cache_key_hash_and_equality() {
    let t1 = RationalTime::from_frames(10, 24);
    let t2 = RationalTime::from_frames(10, 24);
    let t3 = RationalTime::from_frames(11, 24);

    let k1 = CacheKey::new(42, t1);
    let k2 = CacheKey::new(42, t2);
    let k3 = CacheKey::new(42, t3);
    let k4 = CacheKey::new(99, t1);

    assert_eq!(k1, k2);
    assert_ne!(k1, k3);
    assert_ne!(k1, k4);

    let mut set = HashSet::new();
    set.insert(k1);
    assert!(set.contains(&k2));
    assert!(!set.contains(&k3));
    assert!(!set.contains(&k4));
}

#[test]
fn test_frame_cache_clear_and_replacement() {
    let cache = FrameCache::new(500);

    let k1 = CacheKey::new(1, RationalTime::from_frames(0, 30));
    let frame1 = VideoFrame {
        pts: RationalTime::ZERO,
        width: 10,
        height: 5,
        data: vec![128u8; 200],
        stride: 40,
    };

    cache.insert(k1, frame1);
    assert_eq!(cache.current_bytes(), 200);

    // Overwriting the same key with different size (300 bytes)
    let frame2 = VideoFrame {
        pts: RationalTime::ZERO,
        width: 10,
        height: 5,
        data: vec![255u8; 300],
        stride: 40,
    };
    cache.insert(k1, frame2);
    assert_eq!(cache.current_bytes(), 300);

    let cached = cache.get(&k1).expect("k1 should exist");
    assert_eq!(cached.data.len(), 300);
    assert_eq!(cached.data[0], 255);

    // Test clear
    cache.clear();
    assert_eq!(cache.current_bytes(), 0);
    assert!(cache.get(&k1).is_none());
}

#[test]
fn test_frame_cache_multi_eviction() {
    // Budget 1000 bytes
    let cache = FrameCache::new(1000);

    // Insert 4 frames of 200 bytes = 800 bytes
    for i in 1..=4 {
        let key = CacheKey::new(1, RationalTime::from_frames(i, 30));
        let frame = VideoFrame {
            pts: RationalTime::from_frames(i, 30),
            width: 5,
            height: 10,
            data: vec![i as u8; 200],
            stride: 20,
        };
        cache.insert(key, frame);
    }
    assert_eq!(cache.current_bytes(), 800);

    // Insert one large frame of 700 bytes -> total would be 1500 > 1000
    // Must evict frame 1 and frame 2 (200 + 200 = 400 freed -> 800 - 400 + 700 = 1100 > 1000, so frame 3 also evicted!
    // 800 - 600 + 700 = 900 <= 1000)
    let big_key = CacheKey::new(1, RationalTime::from_frames(99, 30));
    let big_frame = VideoFrame {
        pts: RationalTime::from_frames(99, 30),
        width: 10,
        height: 10,
        data: vec![99u8; 700],
        stride: 40,
    };
    cache.insert(big_key, big_frame);

    assert_eq!(cache.current_bytes(), 900); // 200 (frame 4) + 700 (big_frame)
    assert!(cache.get(&CacheKey::new(1, RationalTime::from_frames(1, 30))).is_none());
    assert!(cache.get(&CacheKey::new(1, RationalTime::from_frames(2, 30))).is_none());
    assert!(cache.get(&CacheKey::new(1, RationalTime::from_frames(3, 30))).is_none());
    assert!(cache.get(&CacheKey::new(1, RationalTime::from_frames(4, 30))).is_some());
    assert!(cache.get(&big_key).is_some());
}

#[test]
fn test_waveform_binary_file_format_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let slwf_path = dir.path().join("test.slwf");

    let buckets = vec![
        WaveformBucket { min: -0.8, max: 0.9, rms: 0.45 },
        WaveformBucket { min: -0.2, max: 0.3, rms: 0.15 },
        WaveformBucket { min: 0.0, max: 0.0, rms: 0.0 },
    ];

    WaveformGenerator::save_waveform_file(&buckets, &slwf_path).expect("save_waveform_file failed");
    assert!(slwf_path.exists());

    // Read back and verify format
    let mut file = File::open(&slwf_path).expect("open slwf");
    let mut header = [0u8; 4];
    file.read_exact(&mut header).expect("read magic");
    assert_eq!(&header, b"SLWF");

    let mut ver_bytes = [0u8; 4];
    file.read_exact(&mut ver_bytes).expect("read version");
    assert_eq!(u32::from_le_bytes(ver_bytes), 1);

    let mut count_bytes = [0u8; 8];
    file.read_exact(&mut count_bytes).expect("read count");
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 3);

    for orig in &buckets {
        let mut min_b = [0u8; 4];
        let mut max_b = [0u8; 4];
        let mut rms_b = [0u8; 4];
        file.read_exact(&mut min_b).unwrap();
        file.read_exact(&mut max_b).unwrap();
        file.read_exact(&mut rms_b).unwrap();

        assert_eq!(f32::from_le_bytes(min_b), orig.min);
        assert_eq!(f32::from_le_bytes(max_b), orig.max);
        assert_eq!(f32::from_le_bytes(rms_b), orig.rms);
    }
}

#[test]
fn test_contact_sheet_zero_dimensions_error() {
    let dir = tempdir().expect("tempdir");
    let fake_path = dir.path().join("empty.mp4");
    std::fs::write(&fake_path, b"").unwrap();

    let range = TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(5.0, 30));
    let result = ThumbnailGenerator::generate_contact_sheet(&fake_path, range, 0, 4, 160);
    assert!(result.is_err(), "0 columns must return error");

    let result = ThumbnailGenerator::generate_contact_sheet(&fake_path, range, 4, 0, 160);
    assert!(result.is_err(), "0 rows must return error");
}

#[test]
fn test_export_session_metadata_and_empty_frames() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("test_empty.mp4");

    let preset = ExportPreset {
        name: "Preset720".into(),
        container: "mp4".into(),
        video_codec: "h264".into(),
        audio_codec: "aac".into(),
        width: 1280,
        height: 720,
        fps: 60.0,
        video_bitrate_kbps: 4000,
        audio_bitrate_kbps: 192,
    };

    let mut session = ExportSession::start(&out, &preset).expect("start export session");
    assert_eq!(session.width(), 1280);
    assert_eq!(session.height(), 720);
    assert!((session.fps() - 60.0).abs() < 1e-4);
    assert_eq!(session.frames_written(), 0);

    // Writing an empty frame is a no-op and does not increment frames_written
    let empty_frame = VideoFrame {
        pts: RationalTime::ZERO,
        width: 1280,
        height: 720,
        data: Vec::new(),
        stride: 0,
    };
    session.write_video_frame(&empty_frame).expect("write empty frame");
    assert_eq!(session.frames_written(), 0);

    // Dropping or finishing session
    let _ = session.finish();
}
