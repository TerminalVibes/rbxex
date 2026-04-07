use crate::cli::ops;

use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CliTarget {
    /// Debug build with source maps enabled
    #[value(help = "Debug build with source maps enabled")]
    Dev,
    /// Debug build with source maps and older Luau compatibility transforms
    #[value(help = "Debug build with source maps and older Luau compatibility transforms")]
    DevCompat,
    /// Minified release build
    #[value(help = "Minified release build")]
    Rel,
    /// Minified release build with older Luau compatibility transforms
    #[value(help = "Minified release build with older Luau compatibility transforms")]
    RelCompat,
}

impl CliTarget {
    fn compat_variant(self) -> Self {
        match self {
            Self::Dev | Self::DevCompat => Self::DevCompat,
            Self::Rel | Self::RelCompat => Self::RelCompat,
        }
    }
}

#[derive(Parser, Debug)]
#[command(about = "Compile the project into a Roblox model")]
pub struct PackArgs {
    /// Path to the input: a directory, a .rbxm file, or a Rojo project file (*.project.json).
    /// Defaults to the current directory, which is searched for *.project.json files.
    #[arg(value_name = "INPUT", default_value = ".")]
    pub input: PathBuf,

    /// Build the release bundle instead of the debug bundle
    #[arg(long, conflicts_with = "all")]
    pub release: bool,

    /// Build both debug and release bundles
    #[arg(long)]
    pub all: bool,

    /// Also build variants with older Luau compatibility transforms
    #[arg(long)]
    pub compat: bool,

    /// Explicit build target(s), comma-separated
    #[arg(
        short = 't',
        long = "target",
        value_name = "TARGETS",
        value_enum,
        value_delimiter = ',',
        conflicts_with_all = ["release", "all", "compat"],
        help_heading = "Advanced"
    )]
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

impl PackArgs {
    pub(crate) fn selected_targets(&self) -> Vec<CliTarget> {
        if !self.targets.is_empty() {
            return self.targets.clone();
        }

        let profiles = if self.all {
            &[CliTarget::Dev, CliTarget::Rel][..]
        } else if self.release {
            &[CliTarget::Rel][..]
        } else {
            &[CliTarget::Dev][..]
        };

        let mut targets = Vec::with_capacity(profiles.len() * if self.compat { 2 } else { 1 });
        for &target in profiles {
            targets.push(target);
            if self.compat {
                targets.push(target.compat_variant());
            }
        }
        targets
    }
}

pub fn exec(args: PackArgs) -> Result<()> {
    ops::pack::run(args)?;
    Ok(())
}
