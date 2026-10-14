use crate::cli::NewArgs;
use anyhow::{Context, Result};
use slate_core::model::{Project, Timeline, TimelineSettings};
use slate_core::project::ProjectBundle;

pub fn execute(args: NewArgs, json: bool) -> Result<()> {
    let project_dir = args.path;
    let project_name = args.name.unwrap_or_else(|| {
        project_dir
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Project")
            .to_string()
    });

    ProjectBundle::init_bundle(&project_dir)
        .with_context(|| format!("Failed to initialize project bundle at {:?}", project_dir))?;

    let mut project = Project::new(project_name.clone());

    // Update settings if custom resolution/fps provided
    let settings = TimelineSettings {
        timebase: args.fps,
        width: args.width,
        height: args.height,
        drop_frame: false,
        sample_rate: 48000,
    };
    project.timelines[0] = Timeline::new("Main Sequence", settings);

    ProjectBundle::save_to_bundle(&project_dir, &project)
        .with_context(|| format!("Failed to save project bundle at {:?}", project_dir))?;

    if json {
        let out = serde_json::json!({
            "status": "success",
            "project": {
                "id": project.id,
                "name": project.name,
                "path": project_dir,
                "resolution": format!("{}x{}", args.width, args.height),
                "fps": args.fps
            }
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "✓ Created new Slate project '{}' at {}\n  Resolution: {}x{}, {} fps",
            project_name,
            project_dir.display(),
            args.width,
            args.height,
            args.fps
        );
    }

    Ok(())
}
