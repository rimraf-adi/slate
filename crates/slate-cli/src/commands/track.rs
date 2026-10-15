//! Command handler for `slate track` (add, remove, list tracks).

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use slate_core::history::OpSource;
use slate_core::model::TrackType;
use slate_core::op::Op;
use slate_core::project::ProjectBundle;
use slate_core::Engine;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct TrackArgs {
    #[command(subcommand)]
    pub command: TrackCommands,
}

#[derive(Subcommand, Debug)]
pub enum TrackCommands {
    /// List all tracks in the active project timeline
    List {
        #[arg(value_name = "PROJECT_DIR")]
        project: PathBuf,
    },
    /// Add a new track to the project timeline
    Add {
        #[arg(value_name = "PROJECT_DIR")]
        project: PathBuf,

        /// Track name (e.g. V2, A2, B-Roll, Foley)
        #[arg(value_name = "NAME")]
        name: String,

        /// Track type: video, audio, or caption [default: video]
        #[arg(short, long, default_value = "video")]
        r#type: String,

        /// Optional index where to insert the track
        #[arg(short, long)]
        index: Option<usize>,
    },
    /// Remove an existing track from the timeline
    Remove {
        #[arg(value_name = "PROJECT_DIR")]
        project: PathBuf,

        /// Track name or UUID to remove
        #[arg(value_name = "TRACK")]
        track: String,
    },
}

pub fn execute(args: TrackArgs, json: bool) -> Result<()> {
    match args.command {
        TrackCommands::List { project } => {
            let p = ProjectBundle::load_from_bundle(&project)
                .with_context(|| format!("Failed to load project at {:?}", project))?;
            let tl = p.active_timeline().context("No active timeline")?;

            if json {
                let list = tl.tracks.iter().map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "name": t.name,
                        "type": t.track_type,
                        "muted": t.muted,
                        "locked": t.locked,
                        "clipCount": t.items.len(),
                    })
                }).collect::<Vec<_>>();
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("Tracks for {}:", p.name);
                for (idx, t) in tl.tracks.iter().enumerate() {
                    println!("  #{:02} {:?} \"{}\" (ID: {}) - {} clips", idx + 1, t.track_type, t.name, t.id, t.items.len());
                }
            }
        }
        TrackCommands::Add { project, name, r#type, index } => {
            let track_type = match r#type.to_lowercase().as_str() {
                "audio" | "a" => TrackType::Audio,
                "caption" | "subtitle" | "sub" => TrackType::Caption,
                _ => TrackType::Video,
            };

            let op = Op::AddTrack {
                name: name.clone(),
                track_type,
                index,
            };

            let mut engine = Engine::open_bundle(project.clone())
                .map_err(|e| anyhow::anyhow!("Failed to open engine: {}", e))?;

            engine.apply_op(op, OpSource::Human, format!("CLI Add Track '{}'", name))
                .context("Failed to add track")?;

            engine.save_to_bundle(&project)
                .map_err(|e| anyhow::anyhow!("Failed to save project: {}", e))?;

            if json {
                let out = serde_json::json!({
                    "status": "success",
                    "action": "add_track",
                    "name": name,
                    "type": track_type
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("✓ Added {:?} track '{}' to project {}", track_type, name, project.display());
            }
        }
        TrackCommands::Remove { project, track } => {
            let mut engine = Engine::open_bundle(project.clone())
                .map_err(|e| anyhow::anyhow!("Failed to open engine: {}", e))?;

            let track_id = {
                let tl = engine.project().active_timeline().context("No active timeline")?;
                let found = tl.tracks.iter().find(|t| t.name == track || t.id.to_string() == track);
                found.map(|t| t.id).with_context(|| format!("Track '{}' not found", track))?
            };

            let op = Op::RemoveTrack { track_id };
            engine.apply_op(op, OpSource::Human, format!("CLI Remove Track '{}'", track))
                .context("Failed to remove track")?;

            engine.save_to_bundle(&project)
                .map_err(|e| anyhow::anyhow!("Failed to save project: {}", e))?;

            if json {
                let out = serde_json::json!({
                    "status": "success",
                    "action": "remove_track",
                    "track_id": track_id
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("✓ Removed track '{}' (ID: {})", track, track_id);
            }
        }
    }

    Ok(())
}
