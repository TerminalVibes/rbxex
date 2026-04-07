use crate::cli::ops;

use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CliTarget {
    /// Debug build with source maps enabled
    #[value(help = "Debug build with source maps enabled")]
    Dev,
    /// Debug build with source maps and Lua 5.1 compatibility (no floor division, compound assignment, etc.)
    #[value(help = "Debug build with source maps and Lua 5.1 compatibility")]
    DevCompat,
    /// Minified release build
    #[value(help = "Minified release build")]
    Rel,
    /// Minified release build with Lua 5.1 compatibility
    #[value(help = "Minified release build with Lua 5.1 compatibility")]
    RelCompat,
}

#[derive(Parser, Debug)]
#[command(about = "Compile the project into a Roblox model")]
pub struct PackArgs {
    /// Path to the input: a directory, a .rbxm file, or a Rojo project file (*.project.json).
    /// Defaults to the current directory, which is searched for *.project.json files.
    #[arg(value_name = "INPUT", default_value = ".")]
    pub input: PathBuf,

    /// One or more build targets
    #[arg(short = 't', long = "target", value_enum, value_delimiter = ',', default_values_t = [CliTarget::Dev, CliTarget::Rel])]
    pub targets: Vec<CliTarget>,

    /// Output directory for generated bundles
    #[arg(
        short = 'o',
        long = "out-dir",
        value_name = "DIR",
        default_value = "dist"
    )]
    pub out_dir: PathBuf,

    /// Path to a custom header file
    #[arg(long)]
    pub header: Option<PathBuf>,

    /// Watch for file changes and rebuild automatically
    #[arg(short = 'w', long)]
    pub watch: bool,
}

pub fn exec(args: PackArgs) -> Result<()> {
    ops::pack::run(args)?;
    Ok(())
}
