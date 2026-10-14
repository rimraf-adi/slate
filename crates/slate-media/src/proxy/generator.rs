use crate::error::{MediaError, Result};
use std::path::Path;
use std::process::Command;

pub struct ProxyGenerator;

impl ProxyGenerator {
    /// Transcodes input video at `source_path` to a lightweight 720p H.264/AAC proxy at `dest_path`.
    pub fn generate_proxy(
        source_path: &Path,
        dest_path: &Path,
        on_progress: Option<Box<dyn Fn(f32) + Send + Sync>>,
    ) -> Result<()> {
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| MediaError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Use FFmpeg subprocess with scale=1280:720 (maintaining aspect ratio with letterboxing or scaling)
        // crf=23, preset=veryfast for rapid lightweight editing proxies
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-i")
            .arg(source_path)
            .arg("-vf")
            .arg("scale=1280:-2")
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("veryfast")
            .arg("-crf")
            .arg("23")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("128k")
            .arg(dest_path);

        let output = cmd.output().map_err(|e| MediaError::Io {
            path: dest_path.to_path_buf(),
            source: e,
        })?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(MediaError::TranscodeFailed(err_msg.to_string()));
        }

        if let Some(cb) = on_progress {
            cb(1.0);
        }

        Ok(())
    }
}
