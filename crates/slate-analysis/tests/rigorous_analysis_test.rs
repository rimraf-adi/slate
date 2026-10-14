use slate_analysis::loudness::LoudnessMeter;
use slate_analysis::scene::SceneDetector;
use slate_analysis::transcribe::{export_srt, export_vtt, purge_filler_words};
use slate_analysis::types::{
    SceneParams, Transcript, TranscriptSegment, TranscriptWord, VadParams,
};
use slate_analysis::vad::EnergyVad;
use slate_core::id::MediaId;
use slate_core::time::RationalTime;

#[test]
fn test_purge_filler_words_rigorous() {
    let words = vec![
        TranscriptWord {
            word: "Um,".to_string(),
            start: RationalTime::from_seconds(0.0, 30),
            end: RationalTime::from_seconds(0.4, 30),
            confidence: 0.95,
            segment_id: 0,
        },
        TranscriptWord {
            word: "welcome".to_string(),
            start: RationalTime::from_seconds(0.5, 30),
            end: RationalTime::from_seconds(0.9, 30),
            confidence: 0.99,
            segment_id: 0,
        },
        TranscriptWord {
            word: "to".to_string(),
            start: RationalTime::from_seconds(1.0, 30),
            end: RationalTime::from_seconds(1.2, 30),
            confidence: 0.99,
            segment_id: 0,
        },
        TranscriptWord {
            word: "uh".to_string(),
            start: RationalTime::from_seconds(1.3, 30),
            end: RationalTime::from_seconds(1.5, 30),
            confidence: 0.90,
            segment_id: 0,
        },
        TranscriptWord {
            word: "Slate!".to_string(),
            start: RationalTime::from_seconds(1.6, 30),
            end: RationalTime::from_seconds(2.0, 30),
            confidence: 0.98,
            segment_id: 0,
        },
    ];

    // Purge default fillers ("um", "uh")
    let keep_ranges = purge_filler_words(&words, &[]);
    assert_eq!(keep_ranges.len(), 2);
    // Range 1: words[1..=2] ("welcome", "to")
    assert_eq!(keep_ranges[0].start_word_idx, 1);
    assert_eq!(keep_ranges[0].end_word_idx, 2);
    // Range 2: words[4..=4] ("Slate!")
    assert_eq!(keep_ranges[1].start_word_idx, 4);
    assert_eq!(keep_ranges[1].end_word_idx, 4);

    // Empty words input
    let empty_ranges = purge_filler_words(&[], &[]);
    assert!(empty_ranges.is_empty());

    // Custom filler list
    let custom_fillers = ["welcome"];
    let custom_ranges = purge_filler_words(&words, &custom_fillers);
    // Should exclude only "welcome" (index 1)
    assert_eq!(custom_ranges.len(), 2);
    assert_eq!(custom_ranges[0].start_word_idx, 0);
    assert_eq!(custom_ranges[0].end_word_idx, 0); // "Um,"
    assert_eq!(custom_ranges[1].start_word_idx, 2);
    assert_eq!(custom_ranges[1].end_word_idx, 4); // "to", "uh", "Slate!"
}

#[test]
fn test_export_srt_and_vtt_formatting() {
    let transcript = Transcript {
        media_id: MediaId::new(),
        language: "en".to_string(),
        segments: vec![
            TranscriptSegment {
                id: 0,
                start: RationalTime::from_seconds(1.25, 1000), // 00:00:01,250
                end: RationalTime::from_seconds(3.50, 1000),   // 00:00:03,500
                text: "Hello world.".to_string(),
            },
            TranscriptSegment {
                id: 1,
                start: RationalTime::from_seconds(3.75, 1000), // 00:00:03,750
                end: RationalTime::from_seconds(4.90, 1000),   // 00:00:04,900
                text: "Testing captions.".to_string(),
            },
        ],
        words: Vec::new(),
    };

    // SRT uses commas for milliseconds
    let srt = export_srt(&transcript);
    assert!(srt.contains("1\n00:00:01,250 --> 00:00:03,500\nHello world."));
    assert!(srt.contains("2\n00:00:03,750 --> 00:00:04,900\nTesting captions."));

    // VTT uses dots for milliseconds and has WEBVTT header
    let vtt = export_vtt(&transcript);
    assert!(vtt.starts_with("WEBVTT\n\n"));
    assert!(vtt.contains("1\n00:00:01.250 --> 00:00:03.500\nHello world."));
    assert!(vtt.contains("2\n00:00:03.750 --> 00:00:04.900\nTesting captions."));
}

#[test]
fn test_scene_detector_color_delta() {
    let detector = SceneDetector::new(SceneParams {
        threshold: 0.3,
        min_scene_length_frames: 2,
        color_bins: 8,
    });

    // 100x100 black frame (RGBA = 0, 0, 0, 255)
    let black_frame = vec![0u8; 100 * 100 * 4];
    // 100x100 white frame (RGBA = 255, 255, 255, 255)
    let white_frame = vec![255u8; 100 * 100 * 4];

    // Sequence of 5 frames: Black, Black, Black, White, White
    let frames: Vec<&[u8]> = vec![
        &black_frame,
        &black_frame,
        &black_frame,
        &white_frame,
        &white_frame,
    ];

    let cuts = detector.detect_from_rgba_frames(&frames, 30);
    assert_eq!(cuts.len(), 1);
    assert_eq!(cuts[0].frame, 3); // Cut detected at frame index 3
    assert_eq!(cuts[0].time, RationalTime::from_frames(3, 30));
    assert!(cuts[0].score > 0.5);

    // Empty frames returns empty cuts
    assert!(detector.detect_from_rgba_frames(&[], 30).is_empty());
}

#[test]
fn test_energy_vad_voice_detection() {
    let vad = EnergyVad::new(16000, VadParams::default());

    // 1 second of silence
    let silence = vec![0.0f32; 16000];
    let res_silence = vad.process(&silence);
    assert!(res_silence.speech_segments.is_empty());

    // 1 second of loud sine tone (speech-like energy)
    let mut speech = vec![0.0f32; 16000];
    for (i, sample) in speech.iter_mut().enumerate() {
        let t = i as f32 / 16000.0;
        *sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.8;
    }

    let res_speech = vad.process(&speech);
    assert!(!res_speech.speech_segments.is_empty());
}

#[test]
fn test_loudness_meter_ebur128() {
    let mut meter = LoudnessMeter::new(2, 48000, -14.0).expect("create loudness meter");

    // 1 second of 48000 stereo samples (1kHz sine wave at 0.5 amplitude)
    let mut samples = vec![0.0f32; 48000 * 2];
    for i in 0..48000 {
        let val = (i as f32 * 1000.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 0.5;
        samples[i * 2] = val;
        samples[i * 2 + 1] = val;
    }

    meter.add_frames_f32(&samples).expect("add frames");
    let report = meter.finish().expect("finish report");

    assert_eq!(report.target_lufs, -14.0);
    // A 0.5 sine wave should be roughly between -12 and -6 LUFS
    assert!(report.integrated_lufs > -20.0 && report.integrated_lufs < 0.0);
    // True peak for 0.5 amplitude is ~ -6 dBFS
    assert!(report.true_peak_dbfs > -10.0 && report.true_peak_dbfs < 0.0);
}
