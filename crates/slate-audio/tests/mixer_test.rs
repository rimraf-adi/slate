use slate_audio::clock::PlaybackClock;
use slate_audio::dsp::{db_to_linear, equal_power_pan, soft_clip};
use slate_audio::mixer::SoftwareAudioMixer;
use slate_core::id::TrackId;
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::decoder::AudioBuffer;
use slate_core::traits::mixer::{AudioMixer, MixRequest};

#[test]
fn test_playback_clock_synchronization() {
    let clock = PlaybackClock::new(48000);
    assert_eq!(clock.current_sample_position(), 0);
    assert_eq!(clock.current_seconds(), 0.0);
    assert!(!clock.is_playing());

    clock.play();
    assert!(clock.is_playing());

    // Advance 48,000 samples (1 second)
    clock.advance_samples(48000);
    assert_eq!(clock.current_sample_position(), 48000);
    assert_eq!(clock.current_seconds(), 1.0);

    let t30 = clock.current_time(30);
    assert_eq!(t30.ticks, 30);
    assert_eq!(t30.rate, 30);

    // Seek to 2.5 seconds
    clock.seek(RationalTime::from_seconds(2.5, 30));
    assert_eq!(clock.current_seconds(), 2.5);
    assert_eq!(clock.current_sample_position(), 120_000);
}

#[test]
fn test_dsp_math() {
    // 0 dB is unity gain 1.0
    assert!((db_to_linear(0.0) - 1.0).abs() < 1e-4);

    // -6 dB is approximately 0.5
    let lin_6db = db_to_linear(-6.0206);
    assert!((lin_6db - 0.5).abs() < 1e-3);

    // Equal-power panning
    // Center pan: left and right equal, each ~ 1/sqrt(2) ≈ 0.7071
    let (left_c, right_c) = equal_power_pan(0.0);
    assert!((left_c - right_c).abs() < 1e-4);
    assert!((left_c - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);

    // Hard left: left ≈ 1.0, right ≈ 0.0
    let (left_l, right_l) = equal_power_pan(-1.0);
    assert!((left_l - 1.0).abs() < 1e-4);
    assert!(right_l.abs() < 1e-4);

    // Hard right: left ≈ 0.0, right ≈ 1.0
    let (left_r, right_r) = equal_power_pan(1.0);
    assert!(left_r.abs() < 1e-4);
    assert!((right_r - 1.0).abs() < 1e-4);

    // Soft clipping
    assert_eq!(soft_clip(0.0), 0.0);
    assert_eq!(soft_clip(0.5), 0.5); // completely linear below 2/3
    assert!(soft_clip(1.5) <= 1.0);   // saturated
    assert!(soft_clip(-1.5) >= -1.0); // saturated
}

#[test]
fn test_multi_track_mixing() {
    let mixer = SoftwareAudioMixer::new();

    // 1 second buffer at 48kHz (48,000 stereo samples = 96,000 float samples)
    let track1_id = TrackId::new();
    let track2_id = TrackId::new();

    // Track 1: Solid 0.2 left/right
    let buf1 = AudioBuffer {
        sample_rate: 48000,
        channels: 2,
        samples: vec![0.2; 96000],
    };

    // Track 2: Solid 0.3 left/right
    let buf2 = AudioBuffer {
        sample_rate: 48000,
        channels: 2,
        samples: vec![0.3; 96000],
    };

    let request = MixRequest {
        range: TimeRange::new(
            RationalTime::from_frames(0, 30),
            RationalTime::from_frames(30, 30),
        ),
        track_streams: vec![
            (track1_id, buf1, 0.0, -1.0), // Track 1 hard left at 0 dB
            (track2_id, buf2, 0.0, 1.0),  // Track 2 hard right at 0 dB
        ],
        master_volume_db: 0.0,
    };

    let mixed = mixer.mix(&request).expect("mixing failed");
    assert_eq!(mixed.sample_rate, 48000);
    assert_eq!(mixed.channels, 2);
    assert_eq!(mixed.samples.len(), 96000);

    // Left channel should be from Track 1 (0.2), Right channel from Track 2 (0.3)
    let left_sample = mixed.samples[0];
    let right_sample = mixed.samples[1];

    assert!((left_sample - 0.2).abs() < 1e-3);
    assert!((right_sample - 0.3).abs() < 1e-3);
}
