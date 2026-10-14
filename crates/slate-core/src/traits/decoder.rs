use crate::model::MediaMetadata;
use crate::time::{RationalTime, TimeRange};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("Failed to probe file '{0}': {1}")]
    ProbeFailed(PathBuf, String),
    #[error("Codec not supported: {0}")]
    UnsupportedCodec(String),
    #[error("Seek error at time {0:?}")]
    SeekError(RationalTime),
    #[error("Hardware acceleration error: {0}")]
    HardwareAccel(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecoderHandle(pub u64);

pub struct VideoFrame {
    pub pts: RationalTime,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA or YUV planar buffer
    pub stride: usize,
}

pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>, // Interleaved 32-bit floating point PCM
}

pub trait MediaDecoder: Send + Sync {
    fn probe(&self, path: &Path) -> Result<MediaMetadata, MediaError>;
    fn open(&self, path: &Path) -> Result<DecoderHandle, MediaError>;
    fn decode_frame(
        &self,
        handle: &DecoderHandle,
        time: RationalTime,
    ) -> Result<VideoFrame, MediaError>;
    fn decode_audio(
        &self,
        handle: &DecoderHandle,
        range: TimeRange,
    ) -> Result<AudioBuffer, MediaError>;
    fn close(&self, handle: DecoderHandle) -> Result<(), MediaError>;
}
