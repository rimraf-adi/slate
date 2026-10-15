//! Command handler for `slate clip` (list, split, remove clips).

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use slate_core::history::OpSource;
use slate_core::model::TrackItem;
use slate_core::op::Op;
use slate_core::project::ProjectBundle;
use slate_core::time::{RationalTime, Timecode};
use slate_core::Engine;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct ClipArgs {
    #[command(subcommand)]
    pub command: ClipCommands,
}

#[derive(Subcommand, Debug)]
pub enum ClipCommands {
    /// List all clips across all tracks in the active project timeline
    List {
        #[arg(value_name = "PROJECT_DIR")]
        project: PathBuf,
    },
    /// Split a clip at a given timestamp in seconds
    Split {
        #[arg(value_name = "PROJECT_DIR")]
        project: PathBuf,

        /// Name or UUID of the clip to split
        #[arg(short, long, value_name = "CLIP")]
        clip: String,

        /// Timestamp in seconds where the cut should occur
        #[arg(short, long, value_name = "SECONDS")]
        time: f64,
    },
    /// Remove a clip from the timeline
    Remove {
        #[arg(value_name = "PROJECT_DIR")]
        project: PathBuf,

        /// Name or UUID of the clip to remove
        #[arg(short, long, value_name = "CLIP")]
        clip: String,

        /// Ripple delete following clips to close the gap
        #[arg(long)]
        ripple: bool,
    },
}

pub fn execute(args: ClipArgs, json: bool) -> Result<()> {
    match args.command {
        ClipCommands::List { project } => {
            let p = ProjectBundle::load_from_bundle(&project)
                .with_context(|| format!("Failed to load project at {:?}", project))?;
            let tl = p.active_timeline().context("No active timeline")?;

            let mut clips_list = Vec::new();
            for track in &tl.tracks {
                for item in &track.items {
                    if let TrackItem::Clip(c) = item {
                        clips_list.push(serde_json::json!({
                            "id": c.id,
                            "name": c.name,
                            "trackId": track.id,
                            "trackName": track.name,
                            "start": c.timeline_range.start.to_seconds(),
                            "duration": c.timeline_range.duration.to_seconds(),
                            "startTimecode": Timecode::format(c.timeline_range.start, false),
                            "endTimecode": Timecode::format(c.timeline_range.end_time(), false),
                        }));
                    }
                }
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&clips_list)?);
            } else {
                println!("Clips in {}:", p.name);
                for (idx, c) in clips_list.iter().enumerate() {
                    println!(
                        "  #{:02} \"{}\" on [{}] • {} ──▶ {} ({:.2}s) [ID: {}]",
                        idx + 1,
                        c["name"].as_str().unwrap_or(""),
                        c["trackName"].as_str().unwrap_or(""),
                        c["startTimecode"].as_str().unwrap_or(""),
                        c["endTimecode"].as_str().unwrap_or(""),
                        c["duration"].as_f64().unwrap_or(0.0),
                        c["id"].as_str().unwrap_or("")
                    );
                }
            }
        }
        ClipCommands::Split { project, clip, time } => {
            let mut engine = Engine::open_bundle(project.clone())
                .map_err(|e| anyhow::anyhow!("Failed to open engine: {}", e))?;

            let (clip_id, timebase) = {
                let tl = engine.project().active_timeline().context("No active timeline")?;
                let mut found = None;
                for track in &tl.tracks {
                    for item in &track.items {
                        if let TrackItem::Clip(c) = item {
                            if c.name == clip || c.id.to_string() == clip {
                                found = Some((c.id, tl.settings.timebase));
                                break;
                            }
                        }
                    }
                }
                found.with_context(|| format!("Clip '{}' not found", clip))?
            };

            let split_time = RationalTime::from_seconds(time, timebase);
            let op = Op::SplitClip {
                clip_id,
                at: split_time,
                new_clip_id: slate_core::id::ClipId::new(),
            };

            engine.apply_op(op, OpSource::Human, format!("CLI Split Clip at {:.2}s", time))
                .context("Failed to split clip")?;

            engine.save_to_bundle(&project)
                .map_err(|e| anyhow::anyhow!("Failed to save project: {}", e))?;

            if json {
                let out = serde_json::json!({
                    "status": "success",
                    "action": "split_clip",
                    "clip_id": clip_id,
                    "split_time": time,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("✓ Split clip '{}' at {:.2}s ({})", clip, time, Timecode::format(split_time, false));
            }
        }
        ClipCommands::Remove { project, clip, ripple } => {
            let mut engine = Engine::open_bundle(project.clone())
                .map_err(|e| anyhow::anyhow!("Failed to open engine: {}", e))?;

            let clip_id = {
                let tl = engine.project().active_timeline().context("No active timeline")?;
                let mut found = None;
                for track in &tl.tracks {
                    for item in &track.items {
                        if let TrackItem::Clip(c) = item {
                            if c.name == clip || c.id.to_string() == clip {
                                found = Some(c.id);
                                break;
                            }
                        }
                    }
                }
                found.with_context(|| format!("Clip '{}' not found", clip))?
            };

            let op = Op::RemoveClip { clip_id };
            engine.apply_op(op, OpSource::Human, format!("CLI Remove Clip '{}'", clip))
                .context("Failed to remove clip")?;

            engine.save_to_bundle(&project)
                .map_err(|e| anyhow::anyhow!("Failed to save project: {}", e))?;

            if json {
                let out = serde_json::json!({
                    "status": "success",
                    "action": "remove_clip",
                    "clip_id": clip_id,
                    "ripple": ripple,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("✓ Removed clip '{}' (ripple: {})", clip, ripple);
            }
        }
    }

    Ok(())
}
