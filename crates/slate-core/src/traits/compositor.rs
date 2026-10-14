use super::decoder::VideoFrame;
use crate::model::{Clip, Transition};
use crate::time::RationalTime;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Shader compilation failed: {0}")]
    ShaderError(String),
    #[error("Out of GPU VRAM")]
    OutOfMemory,
    #[error("Device lost")]
    DeviceLost,
    #[error("Compositor internal error: {0}")]
    Internal(String),
}

pub struct FrameRequest {
    pub timeline_time: RationalTime,
    pub visible_clips: Vec<(Clip, VideoFrame)>,
    pub active_transitions: Vec<(Transition, VideoFrame, VideoFrame, f32)>, // (trans, from, to, progress)
    pub canvas_width: u32,
    pub canvas_height: u32,
}

pub struct RenderedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba_pixels: Vec<u8>,
}

pub trait Compositor: Send + Sync {
    fn composite_frame(&self, frame_request: &FrameRequest) -> Result<RenderedFrame, RenderError>;
}
