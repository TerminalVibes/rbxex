use std::fs;
use std::path::PathBuf;

use pretty_assertions::assert_eq;
use tempfile::tempdir;

use crate::cli::commands::pack::{CliTarget, PackArgs};
use crate::cli::ops::pack::{configure_target, load_header, output_stem, resolve_inputs};

use super::{fixture_path, make_dom, write_rbxm};

fn pack_args(header: Option<PathBuf>) -> PackArgs {
    PackArgs {
        input: PathBuf::from("."),
        targets: vec![CliTarget::Dev],
        out_dir: PathBuf::from("out"),
        header,
        watch: false,
    }
}

fn sorted_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut paths = paths;
    paths.sort();
    paths
}

#[test]
fn resolve_inputs_dir_with_default_returns_only_default() {
    let project_dir = fixture_path("project");
    let inputs = resolve_inputs(&project_dir).unwrap();

    assert_eq!(inputs, vec![project_dir.join("default.project.json")]);
}

#[test]
fn resolve_inputs_dir_without_default_returns_all_project_jsons() {
    let dir = tempdir().unwrap();
    let alpha = dir.path().join("alpha.project.json");
    let beta = dir.path().join("beta.project.json");

    fs::write(&alpha, "{}").unwrap();
    fs::write(&beta, "{}").unwrap();
    fs::write(dir.path().join("plain.json"), "{}").unwrap();

    let inputs = sorted_paths(resolve_inputs(dir.path()).unwrap());

    assert_eq!(inputs, sorted_paths(vec![alpha, beta]));
}

#[test]
fn resolve_inputs_dir_with_no_project_files_returns_empty() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("plain.json"), "{}").unwrap();

    let inputs = resolve_inputs(dir.path()).unwrap();

    assert!(inputs.is_empty());
}

#[test]
fn resolve_inputs_file_path_returns_itself() {
    let dir = tempdir().unwrap();
    let dom = make_dom(&[("Main", "ModuleScript", "return 42")]);
    let model_path = write_rbxm(dir.path(), "model", &dom);

    let inputs = resolve_inputs(&model_path).unwrap();

    assert_eq!(inputs, vec![model_path]);
}

#[test]
fn resolve_inputs_nonexistent_path_returns_err() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing.project.json");

    let result = resolve_inputs(&missing);

    assert!(result.is_err());
}

#[test]
fn configure_target_dev_sets_sourcemap_true() {
    let (suffix, options) = configure_target(CliTarget::Dev);

    assert_eq!(suffix, "debug");
    assert!(options.sourcemap);
}

#[test]
fn configure_target_rel_sets_sourcemap_false() {
    let (suffix, options) = configure_target(CliTarget::Rel);

    assert_eq!(suffix, "release");
    assert!(!options.sourcemap);
}

#[test]
fn configure_target_dev_compat_has_preprocess() {
    let (_, options) = configure_target(CliTarget::DevCompat);
    assert!(options.preprocess.is_some());
}

#[test]
fn configure_target_rel_compat_has_postprocess() {
    let (_, options) = configure_target(CliTarget::RelCompat);
    assert!(options.postprocess.is_some());
}

#[test]
fn load_header_none_returns_none() {
    let header = load_header(&pack_args(None)).unwrap();
    assert_eq!(header, None);
}

#[test]
fn load_header_existing_file_returns_content() {
    let dir = tempdir().unwrap();
    let header_path = dir.path().join("header.lua");
    fs::write(&header_path, "-- custom header").unwrap();

    let header = load_header(&pack_args(Some(header_path))).unwrap();

    assert_eq!(header.as_deref(), Some("-- custom header"));
}

#[test]
fn load_header_missing_file_returns_err() {
    let dir = tempdir().unwrap();
    let result = load_header(&pack_args(Some(dir.path().join("missing.lua"))));
    assert!(result.is_err());
}

#[test]
fn output_stem_uses_rojo_project_name_field() {
    let project_path = fixture_path("project/default.project.json");

    let stem = output_stem(&project_path).unwrap();

    assert_eq!(stem, "FixtureProject");
}

#[test]
fn output_stem_uses_filename_for_rbxm_input() {
    let dir = tempdir().unwrap();
    let dom = make_dom(&[("Main", "ModuleScript", "return 42")]);
    let model_path = write_rbxm(dir.path(), "model", &dom);

    let stem = output_stem(&model_path).unwrap();

    assert_eq!(stem, "model");
}

#[test]
fn output_stem_errors_when_rojo_project_name_is_missing() {
    let dir = tempdir().unwrap();
    let project_path = dir.path().join("broken.project.json");
    fs::write(&project_path, "{\"tree\":{}}").unwrap();

    let result = output_stem(&project_path);

    assert!(result.is_err());
}
