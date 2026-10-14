//! Command handler for `slate gui`.

use anyhow::{Context, Result};
use slate_core::model::Project;
use slate_core::project::ProjectBundle;
use crate::cli::GuiArgs;

pub fn execute(args: GuiArgs, _json: bool) -> Result<()> {
    let project = if let Some(ref proj_dir) = args.project {
        ProjectBundle::load_from_bundle(proj_dir)
            .with_context(|| format!("Failed to load project bundle at {:?}", proj_dir))?
    } else {
        Project::new("Untitled Project")
    };

    slate_ui::run_desktop_app(project)
        .map_err(|e| anyhow::anyhow!("Failed to run Slate desktop UI: {}", e))?;

    Ok(())
}
