use std::{borrow::Cow, fs, path::Path, process::Command};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use serde_json::{Value, json};

use crate::cli::commands::init::{InitArgs, PackageManager, Template, ToolchainManager};

// ── Embedded templates ────────────────────────────────────────────────────────

const ESLINT: &str = include_str!("../../../templates/common/eslint.config.mjs");
const PRETTIER: &str = include_str!("../../../templates/common/prettier.config.mjs");
const GITIGNORE: &str = include_str!("../../../templates/common/gitignore");
const VSCODE_SETTINGS: &str = include_str!("../../../templates/common/vscode.settings.json");
const VSCODE_EXTENSIONS: &str = include_str!("../../../templates/common/vscode.extensions.json");

const ROKIT_TOML: &str = include_str!("../../../templates/toolchain/rokit.toml");
const AFTMAN_TOML: &str = include_str!("../../../templates/toolchain/aftman.toml");
const FOREMAN_TOML: &str = include_str!("../../../templates/toolchain/foreman.toml");

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

// ── Resolved options ──────────────────────────────────────────────────────────

pub(crate) struct ResolvedOptions {
    pub(crate) name: String,
    pub(crate) template: Template,
    pub(crate) package_manager: PackageManager,
    pub(crate) toolchain_manager: ToolchainManager,
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

    scaffold_files(&dir, &files)?;
    run_install(&dir, opts.package_manager)?;
    patch_versions(&dir)?;

    if opts.git {
        run_git_init(&dir)?;
    }

    println!("Initialized project \"{}\" at {}", opts.name, dir.display());
    Ok(())
}

// ── Option resolution (prompts or defaults) ───────────────────────────────────

fn resolve_options(args: InitArgs, dir: &Path) -> Result<ResolvedOptions> {
    let theme = ColorfulTheme::default();

    // Derive the default name from the directory's final component
    let dir_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("my-project")
        .to_string();

    if args.yes {
        return Ok(ResolvedOptions {
            name: args.name.unwrap_or(dir_name),
            template: args.template.unwrap_or(Template::Script),
            package_manager: args.package_manager.unwrap_or_default(),
            toolchain_manager: args.toolchain_manager.unwrap_or_default(),
            git: args.git,
            eslint: args.eslint,
            prettier: args.prettier,
            vscode: args.vscode,
        });
    }

    // Interactive mode — prompt for each option

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

    let package_manager = match args.package_manager {
        Some(pm) => pm,
        None => {
            let idx = Select::with_theme(&theme)
                .with_prompt("Package manager")
                .items(&["npm", "pnpm", "yarn"])
                .default(0)
                .interact()?;
            match idx {
                0 => PackageManager::Npm,
                1 => PackageManager::Pnpm,
                _ => PackageManager::Yarn,
            }
        }
    };

    let toolchain_manager = match args.toolchain_manager {
        Some(tm) => tm,
        None => {
            let idx = Select::with_theme(&theme)
                .with_prompt("Toolchain manager")
                .items(&["rokit", "aftman", "foreman", "none"])
                .default(0)
                .interact()?;
            match idx {
                0 => ToolchainManager::Rokit,
                1 => ToolchainManager::Aftman,
                2 => ToolchainManager::Foreman,
                _ => ToolchainManager::None,
            }
        }
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
        ToolchainManager::Rokit => files.push((
            "rokit.toml",
            Cow::Owned(ROKIT_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        ToolchainManager::Aftman => files.push((
            "aftman.toml",
            Cow::Owned(AFTMAN_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        ToolchainManager::Foreman => files.push((
            "foreman.toml",
            Cow::Owned(FOREMAN_TOML.replace("{{rbxex_version}}", rbxex_version)),
        )),
        ToolchainManager::None => {}
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
                "build:pack": "rbxex pack -o dist",
                "watch:pack": "rbxex pack -o dist -w"
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
    let (cmd, args): (&str, &[&str]) = match pm {
        PackageManager::Npm => ("npm", &["install"]),
        PackageManager::Pnpm => ("pnpm", &["install"]),
        PackageManager::Yarn => ("yarn", &["install"]),
    };

    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| format!("Failed to run `{} install`. Is it installed?", cmd))?;

    if !status.success() {
        bail!("`{} install` failed", cmd);
    }

    Ok(())
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
    let status = Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .status()
        .context("Failed to run `git init`. Is git installed?")?;

    if !status.success() {
        bail!("`git init` failed");
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_file(dir: &Path, relative: &str, contents: &str) -> Result<()> {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("Failed to write: {}", path.display()))
}
