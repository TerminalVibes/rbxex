pub mod commands;
pub mod ops;
pub mod utils;

use std::process::ExitCode;

use clap::CommandFactory;
use commands::{Cli, Commands};
use owo_colors::{OwoColorize, Stream};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

impl Cli {
    pub fn run(self) -> ExitCode {
        let Some(command) = self.command else {
            if let Err(err) = Cli::command().print_help() {
                print_error(err);
                return ExitCode::FAILURE;
            }
            println!();
            return ExitCode::SUCCESS;
        };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_filter(self.global.verbosity.tracing_level_filter()),
            )
            .init();

        if let Err(err) = command.run() {
            print_error(err);
            return ExitCode::FAILURE;
        }

        ExitCode::SUCCESS
    }
}

impl Commands {
    fn run(self) -> anyhow::Result<()> {
        match self {
            Commands::Pack(args) => commands::pack::exec(args),
            Commands::Init(args) => commands::init::exec(args),
        }
    }
}

fn print_error(err: impl std::fmt::Display) {
    eprintln!(
        "{} {:#}",
        "error:".if_supports_color(Stream::Stderr, |t| t.red()),
        err
    );
}
