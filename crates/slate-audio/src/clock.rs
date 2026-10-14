use slate_core::time::RationalTime;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The master playback clock for the Slate editor.
/// Tracks current playhead position in audio sample ticks and coordinates A/V synchronization.
#[derive(Debug)]
pub struct PlaybackClock {
    sample_rate: u32,
    sample_position: AtomicU64,
    is_playing: AtomicBool,
}

impl PlaybackClock {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            sample_position: AtomicU64::new(0),
            is_playing: AtomicBool::new(false),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Acquire)
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::Release);
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::Release);
    }

    pub fn set_playing(&self, playing: bool) {
        self.is_playing.store(playing, Ordering::Release);
    }

    /// Seeks the master clock to a specific RationalTime position.
    pub fn seek(&self, time: RationalTime) {
        let seconds = time.to_seconds().max(0.0);
        let samples = (seconds * self.sample_rate as f64).round() as u64;
        self.sample_position.store(samples, Ordering::Release);
    }

    /// Advances the clock by N samples (called exclusively by the audio render or output callback).
    pub fn advance_samples(&self, sample_frames: usize) {
        self.sample_position.fetch_add(sample_frames as u64, Ordering::AcqRel);
    }

    /// Reads the current playback position as fractional seconds.
    pub fn current_seconds(&self) -> f64 {
        let samples = self.sample_position.load(Ordering::Acquire);
        samples as f64 / self.sample_rate as f64
    }

    /// Reads the current playback position as a frame-accurate RationalTime at the target rate (e.g. 30fps).
    pub fn current_time(&self, rate: u32) -> RationalTime {
        RationalTime::from_seconds(self.current_seconds(), rate)
    }

    /// Returns the raw current sample counter.
    pub fn current_sample_position(&self) -> u64 {
        self.sample_position.load(Ordering::Acquire)
    }
}
