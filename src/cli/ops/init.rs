use std::{
    borrow::Cow,
    fmt, fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use serde_json::{Value, json};

use crate::cli::{
    commands::init::{InitArgs, PackageManager, Template, ToolchainManager},
    utils::command::resolve_command,
};

// ── Embedded templates ────────────────────────────────────────────────────────

const ESLINT: &str = include_str!("../../../templates/common/eslint.config.mjs");
const PRETTIER: &str = include_str!("../../../templates/common/prettier.config.mjs");
const GITIGNORE: &str = include_str!("../../../templates/common/gitignore");
const VSCODE_SETTINGS: &str = include_str!("../../../templates/common/vscode.settings.json");
const VSCODE_EXTENSIONS: &str = include_str!("../../../templates/common/vscode.extensions.json");

const ROKIT_TOML: &str = include_str!("../../../templates/toolchain/rokit.toml");
const AFTMAN_TOML: &str = include_str!("../../../templates/toolchain/aftman.toml");
const FOREMAN_TOML: &str = include_str!("../../../templates/toolchain/foreman.toml");
const MISE_TOML: &str = include_str!("../../../templates/toolchain/mise.toml");

const PACKAGE_TSCONFIG: &str = include_str!("../../../templates/package/tsconfig.json");
const PACKAGE_INDEX: &str = include_str!("../../../templates/package/index.ts");

const SCRIPT_TSCONFIG: &str = include_str!("../../../templates/script/tsconfig.json");
const SCRIPT_INDEX: &str = include_str!("../../../templates/script/index.client.ts");
const SCRIPT_PROJECT: &str = include_str!("../../../templates/script/default.project.json");

// ── Dev dependencies (versions written as "*" initially) ─────────────────────

const DEPS_CORE: &[&str] = &[
    "@executor-ts/types",
    "@rbxts/compiler-types",
    "@rbxts/types",
    "roblox-ts",
    "typescript",
];

const DEPS_ESLINT: &[&str] = &[
    "@eslint/js",
    "@typescript-eslint/eslint-plugin",
    "@typescript-eslint/parser",
    "eslint",
    "eslint-plugin-roblox-ts",
];

const DEPS_PRETTIER: &[&str] = &["prettier"];

// Only needed when both eslint and prettier are enabled
const DEPS_ESLINT_PRETTIER_BRIDGE: &[&str] = &["eslint-config-prettier", "eslint-plugin-prettier"];

const GENERATED_TOOLCHAIN_TOOL_IDS: &[&str] = &["rojo-rbx/rojo", "terminalvibes/rbxex"];

// ── Tool detection ────────────────────────────────────────────────────────────

fn is_installed(cmd: &str) -> bool {
    let Some(resolved) = resolve_command(cmd) else {
        return false;
    };

    command_succeeds(resolved, &["--version"])
}

fn command_succeeds(command: PathBuf, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub(crate) fn detect_package_managers_with_resolver(
    mut resolve: impl FnMut(&str) -> Option<PathBuf>,
) -> Vec<PackageManager> {
    [
        PackageManager::Npm,
        PackageManager::Pnpm,
        PackageManager::Yarn,
    ]
    .into_iter()
    .filter(|pm| {
        let Some(resolved) = resolve(pm.as_cmd()) else {
            return false;
        };

        command_succeeds(resolved, &["--version"])
    })
    .collect()
}

struct DetectedTools {
    package_managers: Vec<PackageManager>,
    rokit: bool,
    aftman: bool,
    foreman: bool,
    mise: bool,
}

fn detect_tools() -> DetectedTools {
    let package_managers = detect_package_managers_with_resolver(resolve_command);

    DetectedTools {
        package_managers,
        rokit: is_installed("rokit"),
        aftman: is_installed("aftman"),
        foreman: is_installed("foreman"),
        mise: is_installed("mise"),
    }
}

/// Returns all toolchain config formats the user can choose, given what's installed.
/// Rokit supports aftman.toml and foreman.toml natively, so those are offered
/// whenever rokit is present — rokit will be used as the actual installer.
fn available_toolchain_options(d: &DetectedTools) -> Vec<ToolchainManager> {
    let mut opts = Vec::new();
    if d.rokit {
        opts.push(ToolchainManager::Rokit);
        opts.push(ToolchainManager::Aftman);
        opts.push(ToolchainManager::Foreman);
    } else {
        if d.aftman {
            opts.push(ToolchainManager::Aftman);
        }
        if d.foreman {
            opts.push(ToolchainManager::Foreman);
        }
    }
    if d.mise {
        opts.push(ToolchainManager::Mise);
    }
    opts
}

impl PackageManager {
    fn as_cmd(self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "yarn",
        }
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_cmd())
    }
}

impl fmt::Display for ToolchainManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ToolchainManager::Rokit => "rokit",
            ToolchainManager::Aftman => "aftman",
            ToolchainManager::Foreman => "foreman",
            ToolchainManager::Mise => "mise",
        };
        f.write_str(s)
    }
}

// ── Resolved options ──────────────────────────────────────────────────────────

pub(crate) struct ResolvedOptions {
    pub(crate) name: String,
    pub(crate) template: Template,
    pub(crate) package_manager: PackageManager,
    pub(crate) toolchain_manager: Option<ToolchainManager>,
    pub(crate) rokit_available: bool,
    pub(crate) git: bool,
    pub(crate) eslint: bool,
    pub(crate) prettier: bool,
    pub(crate) vscode: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(args: InitArgs) -> Result<()> {
    fs::create_dir_all(&args.path)
        .with_context(|| format!("Failed to create directory: {}", args.path.display()))?;
    let dir = args
        .path
        .canonicalize()
        .context("Failed to resolve directory path")?;

    let force = args.force;
    let opts = resolve_options(args, &dir)?;
    let files = build_file_list(&opts);

    if !force {
        check_conflicts(&dir, &files)?;
    }

    let start_time = Instant::now();
    run_final_setup(&dir, &opts, &files)?;

    println!(
        "Successfully initialized project in {:.2}s.",
        start_time.elapsed().as_secs_f64()
    );
    Ok(())
}

fn run_final_setup(dir: &Path, opts: &ResolvedOptions, files: &[(&str, Cow<str>)]) -> Result<()> {
    let spinner = SetupSpinner::start("Creating project files...");

    scaffold_files(dir, files)?;

    if opts.toolchain_manager.is_some() {
        spinner.set_message("Installing toolchain tools...");
        run_toolchain_install(dir, opts.toolchain_manager, opts.rokit_available)?;
    }

    spinner.set_message(format!(
        "Installing {} dependencies...",
        opts.package_manager
    ));
    run_install(dir, opts.package_manager)?;

    spinner.set_message("Recording installed dependency versions...");
    patch_versions(dir)?;

    if opts.git {
        spinner.set_message("Initializing git repository...");
        run_git_init(dir)?;
    }

    spinner.finish();

    Ok(())
}

// ── Option resolution (prompts or defaults) ───────────────────────────────────

fn resolve_options(args: InitArgs, dir: &Path) -> Result<ResolvedOptions> {
    let theme = ColorfulTheme::default();

    let detected = detect_tools();

    // Derive the default name from the directory's final component
    let dir_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("my-project")
        .to_string();

    // ── Package manager ───────────────────────────────────────────────────────

    let package_manager = if let Some(pm) = args.package_manager {
        if !detected.package_managers.contains(&pm) {
            bail!(
                "`{pm}` is not installed. Install it first or choose a different package manager."
            );
        }
        pm
    } else if detected.package_managers.is_empty() {
        bail!("No package manager found. Install one of: npm, pnpm, yarn.");
    } else if detected.package_managers.len() == 1 {
        let pm = detected.package_managers[0];
        println!("Package manager: {pm} (only one detected)");
        pm
    } else {
        // Will be resolved below in interactive / --yes paths
        // Placeholder — overwritten in both branches
        PackageManager::Npm
    };

    // ── Toolchain manager ─────────────────────────────────────────────────────

    let toolchain_options = available_toolchain_options(&detected);

    let toolchain_manager: Option<ToolchainManager> = if args.no_toolchain {
        None
    } else if let Some(tm) = args.toolchain_manager {
        if !toolchain_options.contains(&tm) {
            bail!(
                "`{tm}` is not available. Install it first or choose a different toolchain manager.\n\
                 Alternatively, use --no-toolchain to skip (not recommended)."
            );
        }
        Some(tm)
    } else if toolchain_options.is_empty() {
        bail!(
            "No toolchain manager found. Install one of: rokit (recommended), aftman, foreman, or mise.\n\
             Alternatively, use --no-toolchain to skip (not recommended)."
        );
    } else if toolchain_options.len() == 1 {
        let tm = toolchain_options[0];
        println!("Toolchain manager: {tm} (only one available)");
        Some(tm)
    } else {
        // Placeholder — overwritten in both branches below
        Some(toolchain_options[0])
    };

    // ── --yes fast path ───────────────────────────────────────────────────────

    if args.yes {
        // Resolve package manager if not already pinned to a single detected one
        let pm = if args.package_manager.is_some() || detected.package_managers.len() == 1 {
            package_manager
        } else {
            // Pick first (npm > pnpm > yarn, already in priority order)
            detected.package_managers[0]
        };

        // Resolve toolchain manager if not already pinned
        let tm = if args.no_toolchain
            || args.toolchain_manager.is_some()
            || toolchain_options.len() == 1
        {
            toolchain_manager
        } else {
            Some(toolchain_options[0])
        };

        return Ok(ResolvedOptions {
            name: args.name.unwrap_or(dir_name),
            template: args.template.unwrap_or(Template::Script),
            package_manager: pm,
            toolchain_manager: tm,
            rokit_available: detected.rokit,
            git: args.git,
            eslint: args.eslint,
            prettier: args.prettier,
            vscode: args.vscode,
        });
    }

    // ── Interactive mode ──────────────────────────────────────────────────────

    let name = match args.name {
        Some(n) => n,
        None => Input::with_theme(&theme)
            .with_prompt("Project name")
            .default(dir_name)
            .interact_text()?,
    };

    let template = match args.template {
        Some(t) => t,
        None => {
            let idx = Select::with_theme(&theme)
                .with_prompt("Template")
                .items(&["Script", "Package"])
                .default(0)
                .interact()?;
            if idx == 0 {
                Template::Script
            } else {
                Template::Package
            }
        }
    };

    // Resolve package manager interactively if multiple options remain
    let package_manager = if args.package_manager.is_some() || detected.package_managers.len() == 1
    {
        package_manager
    } else {
        let labels: Vec<String> = detected
            .package_managers
            .iter()
            .map(|pm| pm.to_string())
            .collect();
        let idx = Select::with_theme(&theme)
            .with_prompt("Package manager")
            .items(&labels)
            .default(0)
            .interact()?;
        detected.package_managers[idx]
    };

    // Resolve toolchain manager interactively if multiple options remain
    let toolchain_manager =
        if args.no_toolchain || args.toolchain_manager.is_some() || toolchain_options.len() == 1 {
            toolchain_manager
        } else {
            let labels: Vec<String> = toolchain_options.iter().map(|tm| tm.to_string()).collect();
            let idx = Select::with_theme(&theme)
                .with_prompt("Toolchain manager")
                .items(&labels)
                .default(0)
                .interact()?;
            Some(toolchain_options[idx])
        };

    // For boolean flags: if user explicitly passed --no-X (value is false), skip prompt.
    // If value is still true (the default), prompt them.

    let git = if !args.git {
        false
    } else {
        Confirm::with_theme(&theme)
            .with_prompt("Initialize a git repository?")
            .default(true)
            .interact()?
    };

    let eslint = if !args.eslint {
        false
    } else {
        Confirm::with_theme(&theme)
            .with_prompt("Set up ESLint?")
            .default(true)
            .interact()?
    };

    let prettier = if !args.prettier {
        false
    } else {
        Confirm::with_theme(&theme)
            .with_prompt("Set up Prettier?")
            .default(true)
            .interact()?
    };

    let vscode = if !args.vscode {
        false
    } else {
        Confirm::with_theme(&theme)
            .with_prompt("Generate VSCode settings?")
            .default(true)
            .interact()?
    };

    Ok(ResolvedOptions {
        name,
        template,
        package_manager,
        toolchain_manager,
        rokit_available: detected.rokit,
        git,
        eslint,
        prettier,
        vscode,
    })
}

// ── File scaffolding ──────────────────────────────────────────────────────────

/// Builds the complete list of (relative path, contents) pairs for the project.
/// This is the single source of truth — both conflict checking and writing use it.
pub(crate) fn build_file_list(opts: &ResolvedOptions) -> Vec<(&'static str, Cow<'static, str>)> {
    let mut files: Vec<(&'static str, Cow<'static, str>)> = Vec::new();

    files.push((
        "package.json",
        Cow::Owned(build_package_json(opts).to_string()),
    ));

    files.push((
        "tsconfig.json",
        Cow::Borrowed(match opts.template {
            Template::Package => PACKAGE_TSCONFIG,
            Template::Script => SCRIPT_TSCONFIG,
        }),
    ));

    match opts.template {
        Template::Package => files.push(("src/index.ts", Cow::Borrowed(PACKAGE_INDEX))),
        Template::Script => {
            files.push(("src/index.client.ts", Cow::Borrowed(SCRIPT_INDEX)));
            files.push((
                "default.project.json",
                Cow::Owned(SCRIPT_PROJECT.replace("{{name}}", &opts.name)),
            ));
        }
    }

    files.push((".gitignore", Cow::Borrowed(GITIGNORE)));

    if opts.eslint {
        files.push(("eslint.config.mjs", Cow::Borrowed(ESLINT)));
    }
    if opts.prettier {
        files.push(("prettier.config.mjs", Cow::Borrowed(PRETTIER)));
    }
    if opts.vscode {
        files.push((".vscode/settings.json", Cow::Borrowed(VSCODE_SETTINGS)));
        files.push((".vscode/extensions.json", Cow::Borrowed(VSCODE_EXTENSIONS)));
    }

    let rbxex_version = env!("CARGO_PKG_VERSION");
    match opts.toolchain_manager {
        Some(ToolchainManager::Rokit) => files.push((
            "rokit.toml",
            Cow::Owned(ROKIT_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        Some(ToolchainManager::Aftman) => files.push((
            "aftman.toml",
            Cow::Owned(AFTMAN_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        Some(ToolchainManager::Foreman) => files.push((
            "foreman.toml",
            Cow::Owned(FOREMAN_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        Some(ToolchainManager::Mise) => files.push((
            "mise.toml",
            Cow::Owned(MISE_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        None => {}
    }

    files
}

pub(crate) fn check_conflicts(dir: &Path, files: &[(&str, Cow<str>)]) -> Result<()> {
    let conflicts: Vec<&str> = files
        .iter()
        .map(|(path, _)| *path)
        .filter(|path| dir.join(path).exists())
        .collect();

    if !conflicts.is_empty() {
        let list = conflicts.join("\n  ");
        bail!(
            "The following files already exist in {}:\n  {}\n\nRun with --force to overwrite.",
            dir.display(),
            list
        );
    }

    Ok(())
}

pub(crate) fn scaffold_files(dir: &Path, files: &[(&str, Cow<str>)]) -> Result<()> {
    for (path, contents) in files {
        write_file(dir, path, contents)?;
    }
    Ok(())
}

pub(crate) fn build_package_json(opts: &ResolvedOptions) -> Value {
    let mut dep_list: Vec<&str> = DEPS_CORE.to_vec();
    if opts.eslint {
        dep_list.extend_from_slice(DEPS_ESLINT);
    }
    if opts.prettier {
        dep_list.extend_from_slice(DEPS_PRETTIER);
    }
    if opts.eslint && opts.prettier {
        dep_list.extend_from_slice(DEPS_ESLINT_PRETTIER_BRIDGE);
    }
    dep_list.sort_unstable();

    let star_deps: serde_json::Map<String, Value> = dep_list
        .iter()
        .map(|&dep| (dep.to_string(), json!("*")))
        .collect();

    match opts.template {
        Template::Script => json!({
            "name": opts.name,
            "version": "1.0.0",
            "description": "",
            "main": "index.js",
            "scripts": {
                "build": "rbxtsc",
                "watch": "rbxtsc -w",
                "pack": "rbxex pack",
                "pack:watch": "rbxex pack -w",
                "pack:release": "rbxex pack --release"
            },
            "keywords": [],
            "author": "",
            "license": "ISC",
            "type": "commonjs",
            "devDependencies": star_deps
        }),
        Template::Package => json!({
            "name": format!("@executor-ts/{}", opts.name),
            "version": "1.0.0",
            "description": "",
            "main": "out/init.lua",
            "scripts": {
                "build": "rbxtsc",
                "watch": "rbxtsc -w",
                "prepublishOnly": "npm run build"
            },
            "keywords": [],
            "author": "",
            "license": "ISC",
            "type": "commonjs",
            "types": "out/index.d.ts",
            "files": ["out", "!**/*.tsbuildinfo"],
            "publishConfig": {
                "access": "public"
            },
            "devDependencies": star_deps
        }),
    }
}

// ── Package manager install ───────────────────────────────────────────────────

fn run_install(dir: &Path, pm: PackageManager) -> Result<()> {
    run_command_silently(dir, pm.as_cmd(), &["install"])
}

// ── Toolchain manager install ─────────────────────────────────────────────────

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommandSpec {
    pub(crate) cmd: &'static str,
    pub(crate) args: Vec<&'static str>,
}

impl CommandSpec {
    fn new(cmd: &'static str, args: &[&'static str]) -> Self {
        Self {
            cmd,
            args: args.to_vec(),
        }
    }
}

fn run_toolchain_install(
    dir: &Path,
    manager: Option<ToolchainManager>,
    rokit_available: bool,
) -> Result<()> {
    let Some(manager) = manager else {
        return Ok(());
    };

    for command in toolchain_install_commands(manager, rokit_available) {
        run_command_silently(dir, command.cmd, &command.args)?;
    }

    Ok(())
}

pub(crate) fn toolchain_install_commands(
    manager: ToolchainManager,
    rokit_available: bool,
) -> Vec<CommandSpec> {
    match manager {
        ToolchainManager::Rokit => rokit_toolchain_install_commands(),
        ToolchainManager::Aftman => {
            if rokit_available {
                rokit_toolchain_install_commands()
            } else {
                let mut commands: Vec<CommandSpec> = GENERATED_TOOLCHAIN_TOOL_IDS
                    .iter()
                    .map(|tool| CommandSpec::new("aftman", &["trust", *tool]))
                    .collect();
                commands.push(CommandSpec::new("aftman", &["install"]));
                commands
            }
        }
        ToolchainManager::Foreman => {
            if rokit_available {
                rokit_toolchain_install_commands()
            } else {
                vec![CommandSpec::new("foreman", &["install"])]
            }
        }
        ToolchainManager::Mise => vec![
            CommandSpec::new("mise", &["trust", "mise.toml"]),
            CommandSpec::new("mise", &["install"]),
        ],
    }
}

fn rokit_toolchain_install_commands() -> Vec<CommandSpec> {
    let mut trust_args = Vec::with_capacity(GENERATED_TOOLCHAIN_TOOL_IDS.len() + 1);
    trust_args.push("trust");
    trust_args.extend_from_slice(GENERATED_TOOLCHAIN_TOOL_IDS);

    vec![
        CommandSpec::new("rokit", &trust_args),
        CommandSpec::new("rokit", &["install"]),
    ]
}

// ── Version patching ──────────────────────────────────────────────────────────

fn patch_versions(dir: &Path) -> Result<()> {
    let pkg_path = dir.join("package.json");
    let raw = fs::read_to_string(&pkg_path)
        .context("Failed to read package.json for version patching")?;
    let mut pkg: Value = serde_json::from_str(&raw).context("Failed to parse package.json")?;

    let dev_deps = pkg
        .get_mut("devDependencies")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow!("package.json has no devDependencies"))?;

    let dep_names: Vec<String> = dev_deps.keys().cloned().collect();

    for dep_name in dep_names {
        if dev_deps.get(&dep_name).and_then(|v| v.as_str()) != Some("*") {
            continue;
        }

        let resolved = read_installed_version(dir, &dep_name);
        match resolved {
            Some(version) => {
                dev_deps.insert(dep_name, json!(format!("^{}", version)));
            }
            None => {
                // Leave as "*" if we couldn't find the installed version
            }
        }
    }

    let patched =
        serde_json::to_string_pretty(&pkg).context("Failed to serialize patched package.json")?;
    fs::write(&pkg_path, patched).context("Failed to write patched package.json")?;

    Ok(())
}

fn read_installed_version(dir: &Path, pkg_name: &str) -> Option<String> {
    let pkg_json_path = dir.join("node_modules").join(pkg_name).join("package.json");
    let raw = fs::read_to_string(pkg_json_path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    value.get("version")?.as_str().map(|s| s.to_string())
}

// ── Git ───────────────────────────────────────────────────────────────────────

pub(crate) fn run_git_init(dir: &Path) -> Result<()> {
    run_command_silently(dir, "git", &["init"])
}

// ── Setup progress ───────────────────────────────────────────────────────────

struct SpinnerState {
    running: AtomicBool,
    message: Mutex<String>,
}

struct SetupSpinner {
    state: Option<Arc<SpinnerState>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl SetupSpinner {
    fn start(message: impl Into<String>) -> Self {
        if !io::stderr().is_terminal() {
            return Self {
                state: None,
                handle: None,
            };
        }

        let state = Arc::new(SpinnerState {
            running: AtomicBool::new(true),
            message: Mutex::new(message.into()),
        });

        let thread_state = Arc::clone(&state);
        let handle = thread::spawn(move || run_spinner(thread_state));

        Self {
            state: Some(state),
            handle: Some(handle),
        }
    }

    fn set_message(&self, message: impl Into<String>) {
        let Some(state) = &self.state else {
            return;
        };

        if let Ok(mut current) = state.message.lock() {
            *current = message.into();
        }
    }

    fn finish(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if let Some(state) = self.state.take() {
            state.running.store(false, Ordering::Relaxed);
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SetupSpinner {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_spinner(state: Arc<SpinnerState>) {
    let frames = ["-", "\\", "|", "/"];
    let mut frame = 0;
    let mut previous_len: usize = 0;
    let mut stderr = io::stderr();

    while state.running.load(Ordering::Relaxed) {
        let message = state
            .message
            .lock()
            .map(|message| message.clone())
            .unwrap_or_else(|_| "Setting up project...".to_string());
        let line = format!("{} {}", frames[frame % frames.len()], message);
        let padding = " ".repeat(previous_len.saturating_sub(line.len()));

        let _ = write!(stderr, "\r{line}{padding}");
        let _ = stderr.flush();

        previous_len = line.len();
        frame += 1;
        thread::sleep(Duration::from_millis(80));
    }

    let _ = write!(stderr, "\r{}\r", " ".repeat(previous_len));
    let _ = stderr.flush();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn run_command_silently(dir: &Path, cmd: &str, args: &[&str]) -> Result<()> {
    let command = format_command(cmd, args);
    let resolved = resolve_command(cmd)
        .with_context(|| format!("Failed to run `{command}`. Is `{cmd}` installed?"))?;
    let output = Command::new(resolved)
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("Failed to run `{command}`. Is `{cmd}` installed?"))?;

    if !output.status.success() {
        bail!(
            "{}",
            format_command_failure(
                &command,
                output.status.code(),
                &output.stdout,
                &output.stderr
            )
        );
    }

    Ok(())
}

fn format_command(cmd: &str, args: &[&str]) -> String {
    if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{cmd} {}", args.join(" "))
    }
}

pub(crate) fn format_command_failure(
    command: &str,
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let summary = match exit_code {
        Some(code) => format!("`{command}` failed with exit code {code}"),
        None => format!("`{command}` failed"),
    };

    let stdout = normalize_command_output(stdout);
    let stderr = normalize_command_output(stderr);

    match (stderr, stdout) {
        (Some(stderr), Some(stdout)) => {
            format!("{summary}:\nstderr:\n{stderr}\n\nstdout:\n{stdout}")
        }
        (Some(stderr), None) => format!("{summary}:\n{stderr}"),
        (None, Some(stdout)) => format!("{summary}:\n{stdout}"),
        (None, None) => summary,
    }
}

fn normalize_command_output(bytes: &[u8]) -> Option<String> {
    let output = String::from_utf8_lossy(bytes);
    let trimmed = output.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn write_file(dir: &Path, relative: &str, contents: &str) -> Result<()> {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("Failed to write: {}", path.display()))
}
