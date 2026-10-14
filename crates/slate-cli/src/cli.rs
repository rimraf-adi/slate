use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "slate",
    bin_name = "slate",
    author = "Slate Video Authors",
    version,
    about = "High-performance non-linear video editing engine and automation CLI",
    long_about = "Slate provides a headless engine for timeline editing, offscreen rendering, \
                  media intelligence analysis, and automated video workflows."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Output machine-readable JSON on stdout
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Increase logging verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new Slate project bundle (.slateproj)
    New(NewArgs),

    /// Probe and inspect a media file (video/audio codecs, fps, duration)
    Info(InfoArgs),

    /// Render and export a project timeline to a target video container
    Render(RenderArgs),

    /// Import a media asset into a project bundle
    Import(ImportArgs),

    /// Extract a video thumbnail or contact sheet
    Thumbnail(ThumbnailArgs),

    /// Apply an atomic batch of operations from a JSON file
    Ops(OpsArgs),

    /// Display version, system, FFmpeg, and GPU capabilities
    Version,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    /// Path to the new project bundle (e.g. ./my_video.slateproj)
    #[arg(value_name = "PROJECT_DIR")]
    pub path: PathBuf,

    /// Name of the project
    #[arg(short, long)]
    pub name: Option<String>,

    /// Timeline frame rate in fps [default: 30]
    #[arg(long, default_value = "30")]
    pub fps: u32,

    /// Canvas width [default: 1920]
    #[arg(long, default_value = "1920")]
    pub width: u32,

    /// Canvas height [default: 1080]
    #[arg(long, default_value = "1080")]
    pub height: u32,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Path to the media file to probe
    #[arg(value_name = "FILE")]
    pub path: PathBuf,
}

#[derive(Args, Debug)]
pub struct RenderArgs {
    /// Path to the project bundle (.slateproj)
    #[arg(value_name = "PROJECT_DIR")]
    pub project: PathBuf,

    /// Target export video path (e.g. output.mp4)
    #[arg(short, long, value_name = "OUTPUT")]
    pub output: PathBuf,

    /// Export preset: 16:9 (1920x1080), 9:16 (1080x1920), 1:1 (1080x1080), or prores
    #[arg(long, default_value = "16:9")]
    pub preset: String,

    /// Video bitrate in kbps [default: 12000]
    #[arg(long, default_value = "12000")]
    pub bitrate: u32,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Path to the project bundle (.slateproj)
    #[arg(value_name = "PROJECT_DIR")]
    pub project: PathBuf,

    /// Media file to import
    #[arg(short, long, value_name = "FILE")]
    pub file: PathBuf,

    /// Generate 720p proxy video during import
    #[arg(long)]
    pub proxy: bool,

    /// Generate multi-resolution waveform (.slwf) during import
    #[arg(long)]
    pub waveform: bool,
}

#[derive(Args, Debug)]
pub struct ThumbnailArgs {
    /// Media file to capture from
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Target image path (e.g. thumb.png or thumb.jpg)
    #[arg(short, long, value_name = "OUTPUT")]
    pub output: PathBuf,

    /// Timestamp in seconds [default: 0.0]
    #[arg(short, long, default_value = "0.0")]
    pub time: f64,

    /// Output thumbnail width [default: 640]
    #[arg(long, default_value = "640")]
    pub width: u32,

    /// Output thumbnail height [default: 360]
    #[arg(long, default_value = "360")]
    pub height: u32,
}

#[derive(Args, Debug)]
pub struct OpsArgs {
    /// Path to the project bundle (.slateproj)
    #[arg(value_name = "PROJECT_DIR")]
    pub project: PathBuf,

    /// Path to JSON file containing a list of Ops
    #[arg(short, long, value_name = "FILE")]
    pub file: PathBuf,

    /// Perform a dry-run validation without writing changes to disk
    #[arg(long)]
    pub dry_run: bool,
}
