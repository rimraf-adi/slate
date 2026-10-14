pub mod audio;
pub mod cache;
pub mod decoder;
pub mod encoder;
pub mod error;
pub mod frame;
pub mod probe;
pub mod proxy;
pub mod thumbnail;

pub use audio::{WaveformBucket, WaveformGenerator};
pub use cache::{CacheKey, FrameCache};
pub use decoder::FfmpegMediaDecoder;
pub use encoder::ExportSession;
pub use error::{MediaError, Result};
pub use frame::{ColorRange, ColorSpace, PixelFormat};
pub use probe::MediaProbe;
pub use proxy::ProxyGenerator;
pub use thumbnail::ThumbnailGenerator;

/// Initializes the global FFmpeg library subsystems and logging.
pub fn init() -> Result<()> {
    ffmpeg_next::init().map_err(MediaError::Ffmpeg)
}
