use slate_analysis::*;
use slate_core::id::MediaId;
use slate_core::model::timeline::{Timeline, TimelineSettings};
use slate_core::model::{Clip, Track, TrackItem, TrackType};
use slate_core::time::{RationalTime, TimeRange};
use tempfile::tempdir;

#[test]
fn test_transcript_cache_and_srt_export() {
    let dir = tempdir().unwrap();
    let cache = TranscriptCache::new(dir.path());

    let media_id = MediaId::new();
    let params = TranscriptionParams::default();
    let key = TranscriptCache::compute_key("fake_hash_12345", &params);

    let transcript = Transcript {
        media_id,
        language: "en".to_string(),
        segments: vec![
            TranscriptSegment {
                id: 0,
                start: RationalTime::from_seconds(0.0, 1000),
                end: RationalTime::from_seconds(2.5, 1000),
                text: "Hello and welcome to Slate.".to_string(),
            },
            TranscriptSegment {
                id: 1,
                start: RationalTime::from_seconds(3.0, 1000),
                end: RationalTime::from_seconds(5.0, 1000),
                text: "This is AI-native video editing.".to_string(),
            },
        ],
        words: vec![
            TranscriptWord {
                word: "Hello".to_string(),
                start: RationalTime::from_seconds(0.0, 1000),
                end: RationalTime::from_seconds(0.5, 1000),
                confidence: 0.95,
                segment_id: 0,
            },
            TranscriptWord {
                word: "um".to_string(),
                start: RationalTime::from_seconds(0.6, 1000),
                end: RationalTime::from_seconds(0.9, 1000),
                confidence: 0.88,
                segment_id: 0,
            },
            TranscriptWord {
                word: "welcome".to_string(),
                start: RationalTime::from_seconds(1.0, 1000),
                end: RationalTime::from_seconds(1.5, 1000),
                confidence: 0.98,
                segment_id: 0,
            },
        ],
    };

    cache.put(&key, &transcript).unwrap();
    let retrieved = cache.get(&key).expect("Should retrieve cached transcript");
    assert_eq!(retrieved.language, "en");
    assert_eq!(retrieved.segments.len(), 2);

    let srt = export_srt(&transcript);
    assert!(srt.contains("00:00:00,000 --> 00:00:02,500"));
    assert!(srt.contains("Hello and welcome to Slate."));

    let vtt = export_vtt(&transcript);
    assert!(vtt.starts_with("WEBVTT"));
    assert!(vtt.contains("00:00:00.000 --> 00:00:02.500"));
}

#[test]
fn test_purge_filler_words() {
    let words = vec![
        TranscriptWord {
            word: "Hello".to_string(),
            start: RationalTime::from_seconds(0.0, 1000),
            end: RationalTime::from_seconds(0.5, 1000),
            confidence: 0.95,
            segment_id: 0,
        },
        TranscriptWord {
            word: "um".to_string(),
            start: RationalTime::from_seconds(0.6, 1000),
            end: RationalTime::from_seconds(0.9, 1000),
            confidence: 0.88,
            segment_id: 0,
        },
        TranscriptWord {
            word: "welcome".to_string(),
            start: RationalTime::from_seconds(1.0, 1000),
            end: RationalTime::from_seconds(1.5, 1000),
            confidence: 0.98,
            segment_id: 0,
        },
        TranscriptWord {
            word: "uh".to_string(),
            start: RationalTime::from_seconds(1.6, 1000),
            end: RationalTime::from_seconds(1.8, 1000),
            confidence: 0.70,
            segment_id: 0,
        },
    ];

    let kept = purge_filler_words(&words, &["um", "uh"]);
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].start_word_idx, 0);
    assert_eq!(kept[0].end_word_idx, 0); // "Hello"
    assert_eq!(kept[1].start_word_idx, 2);
    assert_eq!(kept[1].end_word_idx, 2); // "welcome"
}

#[test]
fn test_edit_by_transcript_ops() {
    let mut timeline = Timeline::new("Test", TimelineSettings::default());
    let mut track = Track::new("Video 1", TrackType::Video);
    let track_id = track.id;

    let media_id = MediaId::new();
    let clip = Clip::new(
        "Interview",
        media_id,
        TimeRange::new(
            RationalTime::from_seconds(0.0, 30),
            RationalTime::from_seconds(10.0, 30),
        ),
        TimeRange::new(
            RationalTime::from_seconds(0.0, 30),
            RationalTime::from_seconds(10.0, 30),
        ),
    );
    let clip_id = clip.id;
    track.items.push(TrackItem::Clip(clip));
    timeline.tracks.push(track);

    let words = vec![
        TranscriptWord {
            word: "KeepThisPart".to_string(),
            start: RationalTime::from_seconds(2.0, 1000),
            end: RationalTime::from_seconds(4.0, 1000),
            confidence: 0.9,
            segment_id: 0,
        },
        TranscriptWord {
            word: "AlsoKeepThis".to_string(),
            start: RationalTime::from_seconds(6.0, 1000),
            end: RationalTime::from_seconds(8.0, 1000),
            confidence: 0.9,
            segment_id: 0,
        },
    ];

    let edit = EditByTranscript::new(vec![
        WordRange {
            start_word_idx: 0,
            end_word_idx: 0,
        },
        WordRange {
            start_word_idx: 1,
            end_word_idx: 1,
        },
    ]);

    let op = edit
        .build_timeline_ops(&words, &timeline, track_id, clip_id)
        .unwrap();

    if let slate_core::Op::Batch { ops, label } = op {
        assert_eq!(label, "Edit by transcript");
        assert!(!ops.is_empty());
    } else {
        panic!("Expected Op::Batch");
    }
}

#[test]
fn test_energy_vad_and_silence_cut() {
    let sample_rate = 16000;
    // Create 3 seconds of audio: 1s silence, 1s speech (sine wave), 1s silence
    let mut samples = vec![0.0f32; sample_rate as usize * 3];
    #[allow(clippy::needless_range_loop)]
    for i in (sample_rate as usize)..(sample_rate as usize * 2) {
        let t = i as f32 / sample_rate as f32;
        samples[i] = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
    }

    let vad = EnergyVad::new(sample_rate, VadParams::default());
    let result = vad.process(&samples);

    assert!(!result.speech_segments.is_empty());
    assert!(!result.silence_segments.is_empty());

    // Test timeline silence removal batch
    let mut timeline = Timeline::new("Test", TimelineSettings::default());
    let mut track = Track::new("Audio 1", TrackType::Audio);
    let track_id = track.id;

    let media_id = MediaId::new();
    let clip = Clip::new(
        "Audio Clip",
        media_id,
        TimeRange::new(
            RationalTime::from_seconds(0.0, 30),
            RationalTime::from_seconds(3.0, 30),
        ),
        TimeRange::new(
            RationalTime::from_seconds(0.0, 30),
            RationalTime::from_seconds(3.0, 30),
        ),
    );
    let clip_id = clip.id;
    track.items.push(TrackItem::Clip(clip));
    timeline.tracks.push(track);

    let cut_op =
        build_silence_removal_ops(&result.silence_segments, &timeline, track_id, clip_id).unwrap();

    if let slate_core::Op::Batch { ops, label } = cut_op {
        assert_eq!(label, "Remove silence");
        assert!(!ops.is_empty());
    } else {
        panic!("Expected Op::Batch");
    }
}

#[test]
fn test_scene_detection() {
    // 16x16 red image
    let red_frame = [255, 0, 0, 255].repeat(256);
    // 16x16 blue image
    let blue_frame = [0, 0, 255, 255].repeat(256);

    let h_red = ColorHistogram::from_rgba_frame(&red_frame, 16);
    let h_red2 = ColorHistogram::from_rgba_frame(&red_frame, 16);
    let h_blue = ColorHistogram::from_rgba_frame(&blue_frame, 16);

    assert_eq!(chi_squared_distance(&h_red, &h_red2), 0.0);
    let diff = chi_squared_distance(&h_red, &h_blue);
    assert!(diff > 0.5, "Disjoint colors must have large distance");

    // Test detector sequence
    let detector = SceneDetector::new(SceneParams {
        threshold: 0.3,
        min_scene_length_frames: 1,
        color_bins: 16,
    });

    let frame_seq: Vec<&[u8]> = vec![
        &red_frame,
        &red_frame,
        &blue_frame, // Scene cut here at frame 2
        &blue_frame,
    ];

    let cuts = detector.detect_from_rgba_frames(&frame_seq, 30);
    assert_eq!(cuts.len(), 1);
    assert_eq!(cuts[0].frame, 2);
}

#[test]
fn test_loudness_meter() {
    let mut meter = LoudnessMeter::new(2, 48000, -14.0).unwrap();
    // 1 second of stereo 1kHz sine wave at -6 dBFS
    let num_samples = 48000;
    let mut samples = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let val = (2.0 * std::f32::consts::PI * 1000.0 * (i as f32 / 48000.0)).sin() * 0.5;
        samples.push(val);
        samples.push(val);
    }

    meter.add_frames_f32(&samples).unwrap();
    let report = meter.finish().unwrap();

    assert!(report.integrated_lufs < 0.0);
    assert_eq!(report.target_lufs, -14.0);
    // gain_adjustment_db = target - integrated
    assert!((report.gain_adjustment_db - (-14.0 - report.integrated_lufs)).abs() < 0.2);
}

#[test]
fn test_contact_sheet_generator() {
    let gen = ContactSheetGenerator::new(ContactSheetParams {
        cols: 2,
        rows: 2,
        thumb_width: 64,
        thumb_height: 36,
        time_range: None,
    });

    let t1 = image::RgbaImage::from_pixel(64, 36, image::Rgba([255, 0, 0, 255]));
    let t2 = image::RgbaImage::from_pixel(64, 36, image::Rgba([0, 255, 0, 255]));
    let t3 = image::RgbaImage::from_pixel(64, 36, image::Rgba([0, 0, 255, 255]));
    let t4 = image::RgbaImage::from_pixel(64, 36, image::Rgba([255, 255, 0, 255]));

    let sheet = gen.generate(&[t1, t2, t3, t4]).unwrap();
    assert!(sheet.width() > 128);
    assert!(sheet.height() > 72);
}
