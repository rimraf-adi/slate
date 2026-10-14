use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("FFmpeg internal error: {0}")]
    Ffmpeg(#[from] ffmpeg_next::Error),

    #[error("IO error accessing media file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("File not found or unreadable: '{0}'")]
    FileNotFound(PathBuf),

    #[error("No video stream found in media file: '{0}'")]
    NoVideoStream(PathBuf),

    #[error("No audio stream found in media file: '{0}'")]
    NoAudioStream(PathBuf),

    #[error("Unsupported codec '{codec}' in file '{path}'")]
    UnsupportedCodec { codec: String, path: PathBuf },

    #[error("Hardware acceleration initialization failed: {0}")]
    HardwareAccelFailed(String),

    #[error("Decoder worker thread failed or disconnected: {0}")]
    WorkerDisconnected(String),

    #[error("Decode timeout or frame retrieval failed for PTS {pts}")]
    FrameDecodeFailed { pts: i64 },

    #[error("Seek operation failed to target PTS {pts}: {reason}")]
    SeekFailed { pts: i64, reason: String },

    #[error("Transcode / Proxy generation failed: {0}")]
    TranscodeFailed(String),

    #[error("Export encoding error: {0}")]
    ExportFailed(String),

    #[error("Image processing error: {0}")]
    ImageError(String),
}

pub type Result<T> = std::result::Result<T, MediaError>;
