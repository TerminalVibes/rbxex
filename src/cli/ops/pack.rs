use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chrono::Local;
use notify::{RecursiveMode, Watcher};
use owo_colors::{OwoColorize, Stream};
use rbx_dom_weak::WeakDom;
use rbxex::core::pack::{BundleOptions, bundle, config};
use serde_json::Value;
use tracing::{debug, instrument};

use crate::cli::commands::pack::{CliTarget, PackArgs};
use crate::cli::utils::rojo::{build_rojo, is_rojo_project, register_project_watches};

struct BuildOutcome {
    successful: usize,
    total: usize,
    elapsed: Duration,
}

impl BuildOutcome {
    fn errors(&self) -> usize {
        self.total - self.successful
    }
}

#[instrument(skip_all, err)]
pub fn run(args: PackArgs) -> Result<()> {
    if args.watch {
        return run_watch(args);
    }
    let outcome = build_once(&args)?;
    let errors = outcome.errors();
    println!(
        "Packed {}/{} targets successfully in {:.2}s with {} {}.",
        outcome.successful,
        outcome.total,
        outcome.elapsed.as_secs_f64(),
        fmt_error_count(errors),
        if errors == 1 { "error" } else { "errors" }
    );
    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn build_once(args: &PackArgs) -> Result<BuildOutcome> {
    let start_time = Instant::now();

    let inputs = resolve_inputs(&args.input)?;
    if inputs.is_empty() {
        bail!("No .project.json or .rbxm files found in {:?}", args.input);
    }

    fs::create_dir_all(&args.out_dir).context("Failed to create output directory")?;

    let header = load_header(args)?;
    let targets = args.selected_targets();

    let mut failed = 0usize;
    for input in &inputs {
        if let Err(e) = build_input(input, args, &header, &targets) {
            eprintln!(
                "{} {}: {:#}",
                "error:".if_supports_color(Stream::Stderr, |t| t.red()),
                input.display(),
                e
            );
            failed += 1;
        }
    }

    let total = inputs.len() * targets.len();
    let successful = (inputs.len() - failed) * targets.len();

    Ok(BuildOutcome {
        successful,
        total,
        elapsed: start_time.elapsed(),
    })
}

fn build_input(
    input_path: &Path,
    args: &PackArgs,
    header: &Option<String>,
    targets: &[CliTarget],
) -> Result<()> {
    let temp_rbxm = if is_rojo_project(input_path) {
        debug!(path = ?input_path, "Building Rojo project");
        Some(build_rojo(input_path)?)
    } else {
        None
    };

    let model_path = temp_rbxm.as_deref().unwrap_or(input_path);

    debug!(path = ?model_path, "Loading model");
    let dom = load_model(model_path)?;

    if let Some(ref path) = temp_rbxm {
        debug!(?path, "Cleaning up temporary file");
        let _ = fs::remove_file(path);
    }

    let stem = output_stem(input_path)?;

    for &target in targets {
        let span = tracing::debug_span!("pack_target", ?target);
        let _enter = span.enter();

        let (suffix, options) = configure_target(target);
        let filename = format!("{}.{}.lua", stem, suffix);
        let output_path = args.out_dir.join(&filename);

        debug!("Packing target");

        let mut source =
            bundle(&dom, options).with_context(|| format!("Failed to pack target {:?}", target))?;

        if let Some(h) = header {
            source = format!("{h}\n{source}");
        }

        fs::write(&output_path, source)
            .with_context(|| format!("Failed to write output to {}", output_path.display()))?;

        debug!(%filename, "Target packed");
    }

    Ok(())
}

pub(crate) fn output_stem(input_path: &Path) -> Result<String> {
    if is_rojo_project(input_path) {
        return rojo_project_name(input_path);
    }

    Ok(input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("bundle")
        .to_string())
}

fn rojo_project_name(path: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read project file {}", path.display()))?;
    let project: Value = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse project file {}", path.display()))?;

    let Some(name) = project.get("name").and_then(Value::as_str) else {
        bail!(
            "Project file {} is missing a string `name` field",
            path.display()
        );
    };

    if name.trim().is_empty() {
        bail!("Project file {} has an empty `name` field", path.display());
    }

    Ok(name.to_string())
}

fn timestamp() -> String {
    Local::now().format("%-I:%M:%S %p").to_string()
}

fn fmt_error_count(n: usize) -> String {
    if n > 0 {
        format!("{}", n.if_supports_color(Stream::Stdout, |n| n.red()))
    } else {
        format!("{}", n.if_supports_color(Stream::Stdout, |n| n.green()))
    }
}

fn print_watch_status(outcome: &BuildOutcome) {
    let errors = outcome.errors();
    let ts = format!("[{}]", timestamp());
    println!(
        "{} Found {} {}. Watching for file changes.\n",
        ts.if_supports_color(Stream::Stdout, |t| t.dimmed()),
        fmt_error_count(errors),
        if errors == 1 { "error" } else { "errors" }
    );
}

fn run_watch(args: PackArgs) -> Result<()> {
    let input = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;

    match build_once(&args) {
        Ok(outcome) => print_watch_status(&outcome),
        Err(e) => eprintln!(
            "{} {:#}",
            "error:".if_supports_color(Stream::Stderr, |t| t.red()),
            e
        ),
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    if input.is_dir() {
        watcher.watch(&input, RecursiveMode::NonRecursive)?;
        for project_path in resolve_inputs(&input)? {
            register_project_watches(&mut watcher, &project_path)?;
        }
    } else if is_rojo_project(&input) {
        register_project_watches(&mut watcher, &input)?;
    } else {
        watcher.watch(&input, RecursiveMode::NonRecursive)?;
        debug!(?input, "Watching .rbxm file");
    }

    loop {
        match rx.recv() {
            Err(_) => break,
            Ok(Err(e)) => {
                eprintln!(
                    "{} watch error: {}",
                    "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()),
                    e
                );
                continue;
            }
            Ok(Ok(_)) => {}
        }

        // debounce: drain further events within 50ms
        let deadline = Instant::now() + Duration::from_millis(50);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match rx.recv_timeout(remaining) {
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let ts = format!("[{}]", timestamp());
        println!(
            "{} File change detected. Packing...",
            ts.if_supports_color(Stream::Stdout, |t| t.dimmed())
        );
        match build_once(&args) {
            Ok(outcome) => print_watch_status(&outcome),
            Err(e) => eprintln!(
                "{} {:#}",
                "error:".if_supports_color(Stream::Stderr, |t| t.red()),
                e
            ),
        }
    }

    Ok(())
}

/// Resolves the input to a list of concrete `.project.json` or `.rbxm` files.
/// For a directory, finds all `*.project.json` files directly within it.
pub(crate) fn resolve_inputs(input: &Path) -> Result<Vec<PathBuf>> {
    if !input.exists() {
        bail!("Input path does not exist: {}", input.display());
    }

    if input.is_dir() {
        // Prefer default.project.json if it exists
        let default = input.join("default.project.json");
        if default.is_file() {
            return Ok(vec![default]);
        }

        let entries =
            fs::read_dir(input).with_context(|| format!("Failed to read directory {:?}", input))?;
        let mut found = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_rojo_project(&path) {
                found.push(path);
            }
        }
        Ok(found)
    } else if input.is_file() {
        Ok(vec![input.to_path_buf()])
    } else {
        bail!("Input path is not a file or directory: {}", input.display());
    }
}

pub(crate) fn load_header(args: &PackArgs) -> Result<Option<String>> {
    if let Some(header_path) = &args.header {
        debug!(?header_path, "Reading custom header");
        Ok(Some(
            fs::read_to_string(header_path).context("Failed to read header file")?,
        ))
    } else {
        Ok(None)
    }
}

#[instrument(skip(path), fields(path = ?path), err)]
fn load_model(path: &Path) -> Result<WeakDom> {
    let file = fs::File::open(path).context("Failed to open model file")?;
    let reader = std::io::BufReader::new(file);
    rbx_binary::from_reader(reader).context("Failed to decode model")
}

pub(crate) fn configure_target(target: CliTarget) -> (&'static str, BundleOptions) {
    match target {
        CliTarget::Dev => (
            "debug",
            BundleOptions {
                sourcemap: true,
                preprocess: Some(config::dev()),
                postprocess: None,
            },
        ),
        CliTarget::DevCompat => (
            "debug.c",
            BundleOptions {
                sourcemap: true,
                preprocess: Some(config::dev_compat()),
                postprocess: None,
            },
        ),
        CliTarget::Rel => (
            "release",
            BundleOptions {
                sourcemap: false,
                preprocess: None,
                postprocess: Some(config::minify()),
            },
        ),
        CliTarget::RelCompat => (
            "release.c",
            BundleOptions {
                sourcemap: false,
                preprocess: None,
                postprocess: Some(config::minify_compat()),
            },
        ),
    }
}
