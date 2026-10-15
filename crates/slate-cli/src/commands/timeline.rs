//! Command handler for `slate timeline` (ASCII multi-track visualization & summary).

use anyhow::{Context, Result};
use slate_core::model::TrackItem;
use slate_core::project::ProjectBundle;
use slate_core::time::Timecode;
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct TimelineArgs {
    /// Path to the project bundle (.slateproj)
    #[arg(value_name = "PROJECT_DIR")]
    pub project: PathBuf,
}

pub fn execute(args: TimelineArgs, json: bool) -> Result<()> {
    let project = ProjectBundle::load_from_bundle(&args.project)
        .with_context(|| format!("Failed to load project bundle at {:?}", args.project))?;

    let timeline = project
        .active_timeline()
        .context("Project contains no active timeline")?;

    if json {
        let out = serde_json::json!({
            "status": "success",
            "project": {
                "name": project.name,
                "id": project.id,
                "schemaVersion": project.schema_version,
            },
            "timeline": {
                "id": timeline.id,
                "name": timeline.name,
                "duration": timeline.duration(),
                "durationSeconds": timeline.duration().to_seconds(),
                "timecode": Timecode::format(timeline.duration(), timeline.settings.drop_frame),
                "settings": timeline.settings,
                "tracksCount": timeline.tracks.len(),
                "tracks": timeline.tracks.iter().map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "name": t.name,
                        "type": t.track_type,
                        "muted": t.muted,
                        "locked": t.locked,
                        "itemsCount": t.items.len(),
                        "clips": t.items.iter().filter_map(|item| {
                            if let TrackItem::Clip(c) = item {
                                Some(serde_json::json!({
                                    "id": c.id,
                                    "name": c.name,
                                    "mediaId": c.media_id,
                                    "start": c.timeline_range.start.to_seconds(),
                                    "duration": c.timeline_range.duration.to_seconds(),
                                    "timecode": Timecode::format(c.timeline_range.start, false),
                                }))
                            } else {
                                None
                            }
                        }).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            },
            "mediaPoolCount": project.media_pool.len(),
            "mediaPool": project.media_pool,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let dur_tc = Timecode::format(timeline.duration(), timeline.settings.drop_frame);
    let dur_secs = timeline.duration().to_seconds();

    println!("\x1b[1;36m┌────────────────────────────────────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[1;36m│\x1b[0m  \x1b[1;37m● SLATE PROJECT: {}\x1b[0m",
        project.name
    );
    println!(
        "\x1b[1;36m│\x1b[0m  \x1b[32m{}×{}\x1b[0m @ \x1b[33m{}.00 fps\x1b[0m  •  Duration: \x1b[1;35m{}\x1b[0m ({:.2}s)",
        timeline.settings.width, timeline.settings.height, timeline.settings.timebase, dur_tc, dur_secs
    );
    println!("\x1b[1;36m└────────────────────────────────────────────────────────────────────────┘\x1b[0m");
    println!();

    println!("\x1b[1;4mTIMELINE TRACKS ({})\x1b[0m:", timeline.tracks.len());
    if timeline.tracks.is_empty() {
        println!("  (No tracks in timeline)");
    } else {
        for (idx, track) in timeline.tracks.iter().enumerate() {
            let type_tag = match track.track_type {
                slate_core::model::TrackType::Video => "\x1b[36m[VIDEO]\x1b[0m",
                slate_core::model::TrackType::Audio => "\x1b[32m[AUDIO]\x1b[0m",
                slate_core::model::TrackType::Caption => "\x1b[35m[TEXT ]\x1b[0m",
            };

            let lock_tag = if track.locked { " 🔒" } else { "" };
            let mute_tag = if track.muted { " [MUTED]" } else { "" };

            println!(
                "  #{:02} {} \x1b[1m{}\x1b[0m ({} items){}{}",
                idx + 1,
                type_tag,
                track.name,
                track.items.len(),
                lock_tag,
                mute_tag
            );

            for item in &track.items {
                if let TrackItem::Clip(clip) = item {
                    let start_tc = Timecode::format(clip.timeline_range.start, false);
                    let end_tc = Timecode::format(clip.timeline_range.end_time(), false);
                    let clip_dur = clip.timeline_range.duration.to_seconds();

                    println!(
                        "       └── \x1b[33m[{start_tc} ──▶ {end_tc}]\x1b[0m  \x1b[37m\"{name}\"\x1b[0m ({dur:.2}s)",
                        start_tc = start_tc,
                        end_tc = end_tc,
                        name = clip.name,
                        dur = clip_dur,
                    );
                }
            }
        }
    }

    println!();
    println!("\x1b[1;4mMEDIA POOL ({})\x1b[0m:", project.media_pool.len());
    if project.media_pool.is_empty() {
        println!("  (No media files imported)");
    } else {
        for (idx, media) in project.media_pool.iter().enumerate() {
            let filename = media
                .original_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("media");
            let mb = media.metadata.file_size_bytes as f64 / (1024.0 * 1024.0);
            let dur = media.metadata.duration.to_seconds();

            let codec_info = match (&media.metadata.video_codec, &media.metadata.audio_codec) {
                (Some(v), Some(a)) => format!("{} / {}", v, a),
                (Some(v), None) => v.clone(),
                (None, Some(a)) => a.clone(),
                (None, None) => "unknown".into(),
            };

            println!(
                "  [{:02}] \x1b[1m{}\x1b[0m • {:.1}s ({codec_info}) • {:.2} MB",
                idx + 1,
                filename,
                dur,
                mb,
            );
        }
    }
    println!();

    Ok(())
}
