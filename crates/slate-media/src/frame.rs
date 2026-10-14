pub use slate_core::traits::decoder::{AudioBuffer, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Rgba8,
    Bgra8,
    Rgb24,
    Nv12,
    Yuv420p,
    Yuv422p10Le,
    Yuv444p10Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Bt709,
    Bt2020,
    Bt601,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRange {
    Full,
    Limited,
}
