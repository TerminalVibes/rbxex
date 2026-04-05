pub mod init;
pub mod pack;

use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

#[derive(Parser, Debug)]
#[command(name = "rbxex", version, about, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Pack(pack::PackArgs),
    Init(init::InitArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub struct GlobalOptions {
    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,
}

/// A prelude module to unify imports
pub mod prelude {
    pub use anyhow::Result;
    pub use clap::{Parser, ValueEnum};
    pub use std::path::PathBuf;
}
