use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use notify::{RecursiveMode, Watcher};
use serde_json::Value;
use tracing::{debug, instrument, warn};

#[instrument(skip(project_path), fields(path = ?project_path), err)]
pub fn build_rojo(project_path: &Path) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_path = std::env::temp_dir().join(format!("rbxex-{}.rbxm", timestamp));

    debug!(output = ?temp_path, "Executing rojo build");

    let output = Command::new("rojo")
        .arg("build")
        .arg(project_path)
        .arg("--output")
        .arg(&temp_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to execute rojo build. Is rojo installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Rojo build failed:\n{}", stderr.trim_end());
    }

    Ok(temp_path)
}

pub fn is_rojo_project(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".project.json"))
        .unwrap_or(false)
}

/// Registers watches for a `.project.json` file and all its `$path` values.
pub fn register_project_watches(watcher: &mut impl Watcher, project_path: &Path) -> Result<()> {
    watcher.watch(project_path, RecursiveMode::NonRecursive)?;
    debug!(?project_path, "Watching project file");

    let project_dir = project_path
        .parent()
        .context("Project file has no parent")?;
    for path in collect_project_paths(project_path, project_dir)? {
        if path.exists() {
            watcher.watch(&path, RecursiveMode::Recursive)?;
            debug!(?path, "Watching path");
        } else {
            warn!(?path, "Watch path does not exist, skipping");
        }
    }

    Ok(())
}

/// Walks the Rojo project JSON and collects all `$path` values as absolute paths.
pub(crate) fn collect_project_paths(
    project_path: &Path,
    project_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let content = fs::read_to_string(project_path).context("Failed to read project file")?;
    let value: Value = serde_json::from_str(&content).context("Failed to parse project file")?;

    let mut paths = Vec::new();
    collect_paths_recursive(&value, project_dir, &mut paths);
    Ok(paths)
}

fn collect_paths_recursive(value: &Value, base: &Path, out: &mut Vec<PathBuf>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(p)) = map.get("$path") {
                out.push(base.join(p));
            }
            for v in map.values() {
                collect_paths_recursive(v, base, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_paths_recursive(v, base, out);
            }
        }
        _ => {}
    }
}
