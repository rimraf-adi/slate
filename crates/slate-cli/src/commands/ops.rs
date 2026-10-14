use crate::cli::OpsArgs;
use anyhow::{Context, Result};
use slate_core::history::OpSource;
use slate_core::op::Op;
use slate_core::project::ProjectBundle;
use slate_core::{Engine, OpApplicable};

pub fn execute(args: OpsArgs, json: bool) -> Result<()> {
    let ops_file_content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read ops file from {:?}", args.file))?;

    let ops: Vec<Op> = serde_json::from_str(&ops_file_content)
        .with_context(|| format!("Failed to parse JSON ops from {:?}", args.file))?;

    if args.dry_run {
        let mut project = ProjectBundle::load_from_bundle(&args.project)
            .with_context(|| format!("Failed to load project bundle at {:?}", args.project))?;

        let active_timeline = project
            .timelines
            .iter_mut()
            .find(|t| t.id == project.active_timeline_id)
            .ok_or_else(|| anyhow::anyhow!("Active timeline not found in project"))?;

        let mut applied_count = 0;
        for (idx, op) in ops.iter().enumerate() {
            op.apply(active_timeline, &mut project.media_pool)
                .with_context(|| format!("Dry-run validation failed at op {}: {:?}", idx, op))?;
            applied_count += 1;
        }

        if json {
            let out = serde_json::json!({
                "status": "success",
                "dry_run": true,
                "applied_ops": applied_count
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("✓ Dry-run: validated {} operations successfully (no changes written)", applied_count);
        }

        return Ok(());
    }

    let mut engine = Engine::open_bundle(args.project.clone())
        .map_err(|e| anyhow::anyhow!("Failed to open engine: {}", e))?;

    let mut applied_count = 0;
    for (idx, op) in ops.iter().enumerate() {
        let label = format!("CLI Op #{}", idx + 1);
        engine
            .apply_op(op.clone(), OpSource::Agent { session_id: "cli".into() }, label)
            .with_context(|| format!("Failed to apply op at index {}: {:?}", idx, op))?;
        applied_count += 1;
    }

    engine
        .save_to_bundle(&args.project)
        .map_err(|e| anyhow::anyhow!("Failed to save project: {}", e))?;

    if json {
        let out = serde_json::json!({
            "status": "success",
            "dry_run": false,
            "applied_ops": applied_count
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("✓ Applied and saved {} operations to {}", applied_count, args.project.display());
    }

    Ok(())
}
