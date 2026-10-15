//! Command handler for `slate doctor` (system, GPU, audio, and FFmpeg diagnostics).

use anyhow::Result;
use slate_audio::cpal::traits::{DeviceTrait, HostTrait};
use slate_render::context::GpuContext;
use std::process::Command;

pub fn execute(json: bool) -> Result<()> {
    let _ = slate_media::init();

    // 1. FFmpeg Diagnostic
    let ffmpeg_output = Command::new("ffmpeg").arg("-version").output();
    let (ffmpeg_ok, ffmpeg_info) = match ffmpeg_output {
        Ok(out) if out.status.success() => {
            let first_line = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("FFmpeg installed")
                .to_string();
            (true, first_line)
        }
        _ => (false, "FFmpeg binary not found on PATH".to_string()),
    };

    // 2. GPU wgpu Diagnostic
    let gpu_result = GpuContext::new_headless();
    let (gpu_ok, gpu_info) = match gpu_result {
        Ok(c) => {
            let info = c.adapter.get_info();
            (
                true,
                format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type),
            )
        }
        Err(e) => (false, format!("Failed to acquire GPU adapter: {}", e)),
    };

    // 3. Audio Device Diagnostic
    let (audio_ok, audio_info) = {
        let host = slate_audio::cpal::default_host();
        match host.default_output_device() {
            Some(dev) => match dev.name() {
                Ok(name) => (true, format!("{} ({:?})", name, host.id())),
                Err(_) => (true, format!("Default Audio Device ({:?})", host.id())),
            },
            None => (false, "No default audio output device available".to_string()),
        }
    };

    let all_ok = ffmpeg_ok && gpu_ok && audio_ok;

    if json {
        let out = serde_json::json!({
            "status": if all_ok { "healthy" } else { "warning" },
            "system": {
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "version": env!("CARGO_PKG_VERSION"),
            },
            "diagnostics": {
                "ffmpeg": {
                    "available": ffmpeg_ok,
                    "details": ffmpeg_info
                },
                "gpu": {
                    "available": gpu_ok,
                    "details": gpu_info
                },
                "audio": {
                    "available": audio_ok,
                    "details": audio_info
                }
            }
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("\x1b[1;36m┌────────────────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1;37mSLATE STUDIO DIAGNOSTICS & SYSTEM DOCTOR\x1b[0m               \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m└────────────────────────────────────────────────────────┘\x1b[0m");
    println!();

    println!("  Operating System : {} ({})", std::env::consts::OS, std::env::consts::ARCH);
    println!("  Slate Version    : v{}", env!("CARGO_PKG_VERSION"));
    println!();

    // FFmpeg
    let ffmpeg_mark = if ffmpeg_ok { "\x1b[1;32m[PASS]\x1b[0m" } else { "\x1b[1;31m[FAIL]\x1b[0m" };
    println!("  {} Media Engine : {}", ffmpeg_mark, ffmpeg_info);

    // GPU
    let gpu_mark = if gpu_ok { "\x1b[1;32m[PASS]\x1b[0m" } else { "\x1b[1;31m[FAIL]\x1b[0m" };
    println!("  {} GPU Pipeline : {}", gpu_mark, gpu_info);

    // Audio
    let audio_mark = if audio_ok { "\x1b[1;32m[PASS]\x1b[0m" } else { "\x1b[1;33m[WARN]\x1b[0m" };
    println!("  {} Audio Output : {}", audio_mark, audio_info);

    println!();
    if all_ok {
        println!("\x1b[1;32m✓ All studio systems operational. Ready for real-time editing & GPU rendering.\x1b[0m");
    } else {
        println!("\x1b[1;33m⚠ Some components reported warnings. Headless fallback available.\x1b[0m");
    }
    println!();

    Ok(())
}
