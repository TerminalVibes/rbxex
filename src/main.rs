mod cli;

use clap::Parser;
use cli::commands::{Cli, Commands};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::commands;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let indicatif_layer = IndicatifLayer::new();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif_layer.get_stderr_writer())
                .with_filter(cli.global.verbosity.tracing_level_filter()),
        )
        .with(indicatif_layer)
        .init();
    match cli.command {
        Commands::Pack(args) => commands::pack::exec(args)?,
        Commands::Init(args) => commands::init::exec(args)?,
    }
    Ok(())
}
