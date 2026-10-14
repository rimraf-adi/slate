use slate_audio::clock::PlaybackClock;
use slate_audio::dsp::{db_to_linear, equal_power_pan, linear_to_db, soft_clip};
use slate_audio::mixer::SoftwareAudioMixer;
use slate_core::id::TrackId;
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::decoder::AudioBuffer;
use slate_core::traits::mixer::{AudioMixer, MixRequest};

#[test]
fn test_playback_clock_full_lifecycle() {
    let clock = PlaybackClock::new(48000);
    assert_eq!(clock.sample_rate(), 48000);
    assert!(!clock.is_playing());
    assert_eq!(clock.current_sample_position(), 0);
    assert_eq!(clock.current_seconds(), 0.0);
    assert_eq!(clock.current_time(30).ticks, 0);

    // Play
    clock.play();
    assert!(clock.is_playing());

    // Advance 48000 samples (1 second)
    clock.advance_samples(48000);
    assert_eq!(clock.current_sample_position(), 48000);
    assert_eq!(clock.current_seconds(), 1.0);
    assert_eq!(clock.current_time(30), RationalTime::new(30, 30));

    // Pause
    clock.pause();
    assert!(!clock.is_playing());

    // Seek to 2.5 seconds
    clock.seek(RationalTime::from_seconds(2.5, 30));
    assert_eq!(clock.current_seconds(), 2.5);
    assert_eq!(clock.current_sample_position(), 120000); // 2.5 * 48000

    // Set playing toggle
    clock.set_playing(true);
    assert!(clock.is_playing());
    clock.set_playing(false);
    assert!(!clock.is_playing());
}

#[test]
fn test_dsp_decibel_and_linear_conversions() {
    // 0 dB = 1.0 linear amplitude
    let unity = db_to_linear(0.0);
    assert!((unity - 1.0).abs() < 1e-4);
    assert!((linear_to_db(unity) - 0.0).abs() < 1e-3);

    // -6 dB ≈ 0.501
    let minus_six = db_to_linear(-6.0206);
    assert!((minus_six - 0.5).abs() < 1e-3);
    assert!((linear_to_db(0.5) - (-6.0206)).abs() < 1e-3);

    // +6 dB ≈ 2.0
    let plus_six = db_to_linear(6.0206);
    assert!((plus_six - 2.0).abs() < 1e-3);

    // Silence threshold (-96 dB or lower -> 0.0)
    assert_eq!(db_to_linear(-100.0), 0.0);
    assert_eq!(linear_to_db(0.0), -100.0);
}

#[test]
fn test_dsp_equal_power_panning() {
    // Center pan: equal energy (~0.7071 for each channel)
    let (left_c, right_c) = equal_power_pan(0.0);
    assert!((left_c - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    assert!((right_c - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    // Total power sum: L^2 + R^2 ≈ 1.0
    assert!((left_c * left_c + right_c * right_c - 1.0).abs() < 1e-4);

    // Hard Left (-1.0)
    let (left_l, right_l) = equal_power_pan(-1.0);
    assert!((left_l - 1.0).abs() < 1e-4);
    assert!(right_l.abs() < 1e-4);

    // Hard Right (+1.0)
    let (left_r, right_r) = equal_power_pan(1.0);
    assert!(left_r.abs() < 1e-4);
    assert!((right_r - 1.0).abs() < 1e-4);
}

#[test]
fn test_dsp_soft_clipping_saturation() {
    // Linear region: |x| <= 2/3 ≈ 0.6667
    assert_eq!(soft_clip(0.0), 0.0);
    assert_eq!(soft_clip(0.5), 0.5);
    assert_eq!(soft_clip(-0.5), -0.5);
    assert_eq!(soft_clip(2.0 / 3.0), 2.0 / 3.0);

    // Compression region: 2/3 < x < 1.0
    let compressed = soft_clip(0.8);
    assert!(compressed > 2.0 / 3.0 && compressed <= 1.0);

    // Hard limit: clamped to [-1.0, 1.0]
    assert_eq!(soft_clip(1.5), 1.0);
    assert_eq!(soft_clip(-2.0), -1.0);
}

#[test]
fn test_software_audio_mixer_stereo_and_mono_mixing() {
    let mixer = SoftwareAudioMixer::with_format(48000, 2);

    // 100ms request with 48000 rate timebase
    let range = TimeRange::new(
        RationalTime::new(0, 48000),
        RationalTime::from_seconds(0.1, 48000),
    );

    // Track 1: Mono 1.0 amplitude centered
    let mono_buffer = AudioBuffer {
        samples: vec![0.5f32; 4800],
        sample_rate: 48000,
        channels: 1,
    };

    // Track 2: Stereo 0.5 amplitude panned hard left
    let stereo_buffer = AudioBuffer {
        samples: vec![0.5f32; 9600],
        sample_rate: 48000,
        channels: 2,
    };

    let request = MixRequest {
        range,
        track_streams: vec![
            (TrackId::new(), mono_buffer, 0.0, 0.0),
            (TrackId::new(), stereo_buffer, 0.0, -1.0),
        ],
        master_volume_db: 0.0,
    };

    let mixed = mixer.mix(&request).expect("mix succeed");
    assert_eq!(mixed.sample_rate, 48000);
    assert_eq!(mixed.channels, 2);
    assert_eq!(mixed.samples.len(), 4800 * 2);

    // Because Track 2 is panned hard left, Left channel should be noticeably louder than Right
    let left_sample = mixed.samples[0];
    let right_sample = mixed.samples[1];
    assert!(left_sample > right_sample);
}
