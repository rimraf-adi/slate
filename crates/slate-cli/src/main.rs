mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = match cli.global.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .init();

    let json = cli.global.json;

    match cli.command {
        Commands::New(args) => commands::new::execute(args, json)?,
        Commands::Info(args) => commands::info::execute(args, json)?,
        Commands::Render(args) => commands::render::execute(args, json)?,
        Commands::Import(args) => commands::import::execute(args, json)?,
        Commands::Thumbnail(args) => commands::thumbnail::execute(args, json)?,
        Commands::Ops(args) => commands::ops::execute(args, json)?,
        Commands::Version => commands::version::execute(json)?,
    }

    Ok(())
}
