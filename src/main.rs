mod cli;

use std::process::ExitCode;

use clap::Parser;
use cli::commands::Cli;

fn main() -> ExitCode {
    Cli::parse().run()
}
