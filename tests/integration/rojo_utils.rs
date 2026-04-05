use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pretty_assertions::assert_eq;
use tempfile::tempdir;

use crate::cli::ops::init::run_git_init;
use crate::cli::utils::rojo::{build_rojo, collect_project_paths, is_rojo_project};

use super::fixture_path;

fn sorted_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut paths = paths;
    paths.sort();
    paths
}

#[test]
fn is_rojo_project_true_for_project_json_suffix() {
    assert!(is_rojo_project(&fixture_path(
        "project/default.project.json"
    )));
}

#[test]
fn is_rojo_project_false_for_plain_json() {
    assert!(!is_rojo_project(Path::new("plain.json")));
}

#[test]
fn is_rojo_project_false_for_rbxm() {
    assert!(!is_rojo_project(Path::new("model.rbxm")));
}

#[test]
fn collect_project_paths_finds_all_dollar_path_values() {
    let project_path = fixture_path("project/multi_path.project.json");
    let project_dir = project_path.parent().unwrap();

    let paths = sorted_paths(collect_project_paths(&project_path, project_dir).unwrap());

    assert_eq!(
        paths,
        sorted_paths(vec![
            project_dir.join("src/shared"),
            project_dir.join("packages/vendor"),
            project_dir.join("src/client"),
        ])
    );
}

#[test]
fn collect_project_paths_handles_nested_tree() {
    let project_path = fixture_path("project/multi_path.project.json");
    let project_dir = project_path.parent().unwrap();

    let paths = collect_project_paths(&project_path, project_dir).unwrap();

    assert!(paths.contains(&project_dir.join("packages/vendor")));
    assert!(paths.contains(&project_dir.join("src/client")));
}

#[test]
fn collect_project_paths_empty_project_returns_empty() {
    let project_path = fixture_path("project/no_paths.project.json");
    let project_dir = project_path.parent().unwrap();

    let paths = collect_project_paths(&project_path, project_dir).unwrap();

    assert!(paths.is_empty());
}

#[test]
fn collect_project_paths_missing_file_returns_err() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing.project.json");

    let result = collect_project_paths(&missing, dir.path());

    assert!(result.is_err());
}

#[test]
fn run_git_init_creates_dot_git_directory() {
    let dir = tempdir().unwrap();

    run_git_init(dir.path()).unwrap();

    assert!(dir.path().join(".git").is_dir());
}

#[test]
#[ignore = "requires rojo on PATH"]
fn build_rojo_creates_rbxm_file() {
    if !matches!(
        Command::new("rojo").arg("--version").output(),
        Ok(output) if output.status.success()
    ) {
        eprintln!("skipping ignored test because `rojo` is not usable in this environment");
        return;
    }

    let dir = tempdir().unwrap();
    let project_path = dir.path().join("default.project.json");
    let shared_dir = dir.path().join("src/shared");

    fs::create_dir_all(&shared_dir).unwrap();
    fs::write(shared_dir.join("init.luau"), "return {}").unwrap();
    fs::write(
        &project_path,
        fs::read_to_string(fixture_path("project/default.project.json")).unwrap(),
    )
    .unwrap();

    let output = build_rojo(&project_path).unwrap();

    assert!(output.is_file());

    let _ = fs::remove_file(output);
}
