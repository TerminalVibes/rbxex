mod cli;

use std::process::ExitCode;

use clap::Parser;
use cli::commands::{Cli, Commands};
use owo_colors::{OwoColorize, Stream};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::commands;

fn main() -> ExitCode {
    let cli = Cli::parse();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(cli.global.verbosity.tracing_level_filter()),
        )
        .init();
    let result = match cli.command {
        Commands::Pack(args) => commands::pack::exec(args),
        Commands::Init(args) => commands::init::exec(args),
    };

    if let Err(err) = result {
        eprintln!(
            "{} {:#}",
            "error:".if_supports_color(Stream::Stderr, |t| t.red()),
            err
        );
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
