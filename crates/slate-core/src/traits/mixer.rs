use super::decoder::AudioBuffer;
use crate::id::TrackId;
use crate::time::TimeRange;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Buffer underrun")]
    Underrun,
    #[error("Channel configuration mismatch")]
    ChannelMismatch,
    #[error("Audio mixer internal error: {0}")]
    Internal(String),
}

pub struct MixRequest {
    pub range: TimeRange,
    pub track_streams: Vec<(TrackId, AudioBuffer, f64, f64)>, // (track_id, buffer, volume_db, pan)
    pub master_volume_db: f64,
}

pub trait AudioMixer: Send + Sync {
    fn mix(&self, request: &MixRequest) -> Result<AudioBuffer, AudioError>;
}
