use crate::error::{MediaError, Result};
use slate_core::model::ExportPreset;
use slate_core::traits::decoder::VideoFrame;
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

pub struct ExportSession {
    child: Child,
    stdin: ChildStdin,
    width: u32,
    height: u32,
    fps: f64,
    frames_written: u64,
}

impl ExportSession {
    /// Spawns an export encoding session configured with `preset` writing to `output_path`.
    pub fn start(output_path: &Path, preset: &ExportPreset) -> Result<Self> {
        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let width = preset.width;
        let height = preset.height;
        let fps = preset.fps;

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("-s")
            .arg(format!("{}x{}", width, height))
            .arg("-r")
            .arg(format!("{:.3}", fps))
            .arg("-i")
            .arg("-");

        // Codec configuration from preset
        match preset.video_codec.to_lowercase().as_str() {
            "prores" => {
                #[cfg(target_os = "macos")]
                cmd.arg("-c:v").arg("prores_videotoolbox");
                #[cfg(not(target_os = "macos"))]
                cmd.arg("-c:v").arg("prores_ks");
                cmd.arg("-profile:v").arg("3"); // ProRes 422 HQ
            }
            "hevc" | "h265" => {
                #[cfg(target_os = "macos")]
                cmd.arg("-c:v").arg("hevc_videotoolbox");
                #[cfg(not(target_os = "macos"))]
                cmd.arg("-c:v").arg("libx265");
                cmd.arg("-crf").arg("20");
            }
            _ => {
                // Default H.264
                #[cfg(target_os = "macos")]
                cmd.arg("-c:v").arg("h264_videotoolbox");
                #[cfg(not(target_os = "macos"))]
                cmd.arg("-c:v").arg("libx264");
                cmd.arg("-pix_fmt").arg("yuv420p");
            }
        }

        cmd.arg("-b:v")
            .arg(format!("{}k", preset.video_bitrate_kbps))
            .arg(output_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| MediaError::Io {
            path: output_path.to_path_buf(),
            source: e,
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| MediaError::ExportFailed("Failed to open stdin for FFmpeg encoder".into()))?;

        Ok(Self {
            child,
            stdin,
            width,
            height,
            fps,
            frames_written: 0,
        })
    }

    /// Pushes a rendered RGBA video frame into the encoder pipeline.
    pub fn write_video_frame(&mut self, frame: &VideoFrame) -> Result<()> {
        if frame.data.is_empty() {
            return Ok(());
        }

        self.stdin
            .write_all(&frame.data)
            .map_err(|e| MediaError::ExportFailed(format!("Failed to write frame to encoder: {}", e)))?;

        self.frames_written += 1;
        Ok(())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn frames_written(&self) -> u64 {
        self.frames_written
    }

    /// Flushes and finalizes the export session, waiting for the container muxing to finish.
    pub fn finish(self) -> Result<()> {
        drop(self.stdin);

        let output = self.child.wait_with_output().map_err(|e| MediaError::ExportFailed(e.to_string()))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(MediaError::ExportFailed(err.to_string()));
        }

        Ok(())
    }
}
