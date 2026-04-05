use super::prelude::*;
use crate::cli::ops;

#[derive(Parser, Debug)]
#[command(about = "Initialize a new or existing directory as a rbxex project")]
pub struct InitArgs {
    /// Path to initialize (defaults to current directory). If the path does not exist, it will be created.
    #[arg(default_value = ".", value_name = "PATH")]
    pub path: PathBuf,

    /// Project name (defaults to the directory name)
    #[arg(long = "name")]
    pub name: Option<String>,

    // === General & Safety ===
    /// Skip all interactive prompts and use defaults for any option not explicitly provided
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Destructive. Allows overwriting non-empty directories or existing configuration files
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    // === Feature Toggles ===
    /// Template to scaffold (package or script)
    #[arg(long = "template", value_enum)]
    pub template: Option<Template>,

    /// Initialize a git repository (default: on)
    #[arg(long = "no-git", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub git: bool,

    /// Set up ESLint configuration (default: on)
    #[arg(long = "no-eslint", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub eslint: bool,

    /// Set up Prettier configuration (default: on)
    #[arg(long = "no-prettier", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub prettier: bool,

    /// Generate VSCode settings (default: on)
    #[arg(long = "no-vscode", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub vscode: bool,

    // === Configuration (Selectors) ===
    /// Selects the toolchain manager to configure
    #[arg(long = "toolchain-manager", value_enum)]
    pub toolchain_manager: Option<ToolchainManager>,

    /// Selects the Node package manager to use
    #[arg(long = "package-manager", value_enum)]
    pub package_manager: Option<PackageManager>,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ToolchainManager {
    Foreman,
    Aftman,
    #[default]
    Rokit,
    None,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum PackageManager {
    #[default]
    Npm,
    Pnpm,
    Yarn,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum Template {
    Package,
    Script,
}

pub fn exec(args: InitArgs) -> Result<()> {
    ops::init::run(args)
}
