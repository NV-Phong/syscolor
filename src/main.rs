mod broadcast;
mod cli;
mod color;
mod constants;
mod errors;
mod power_watch;
mod registry;
mod system_colors;
mod win32;

use anyhow::Result;
use clap::Parser;
use crate::cli::commands::Cli;

fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    cli::commands::run(cli)?;

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .try_init();
}
