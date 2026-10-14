use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Hardware audio device error: {0}")]
    DeviceError(String),

    #[error("Stream creation failed: {0}")]
    StreamError(String),

    #[error("Sample rate conversion failed: {0}")]
    ResampleError(String),

    #[error("Audio buffer underrun in real-time playback thread")]
    Underrun,

    #[error("Audio buffer overflow")]
    Overflow,

    #[error("Core mixer error: {0}")]
    Mixer(#[from] slate_core::traits::mixer::AudioError),
}

pub type Result<T> = std::result::Result<T, AudioError>;
