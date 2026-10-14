use crate::cli::OpsArgs;
use anyhow::{Context, Result};
use slate_core::history::OpSource;
use slate_core::op::Op;
use slate_core::Engine;

pub fn execute(args: OpsArgs, json: bool) -> Result<()> {
    let ops_file_content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read ops file from {:?}", args.file))?;

    let ops: Vec<Op> = serde_json::from_str(&ops_file_content)
        .with_context(|| format!("Failed to parse JSON ops from {:?}", args.file))?;

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

    if !args.dry_run {
        engine
            .save_to_bundle(&args.project)
            .map_err(|e| anyhow::anyhow!("Failed to save project: {}", e))?;
    }

    if json {
        let out = serde_json::json!({
            "status": "success",
            "dry_run": args.dry_run,
            "applied_ops": applied_count
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else if args.dry_run {
        println!("✓ Dry-run: validated {} operations successfully (no changes written)", applied_count);
    } else {
        println!("✓ Applied and saved {} operations to {}", applied_count, args.project.display());
    }

    Ok(())
}
