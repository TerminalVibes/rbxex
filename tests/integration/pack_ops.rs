use std::fs;
use std::path::PathBuf;

use clap::Parser;
use pretty_assertions::assert_eq;
use tempfile::tempdir;

use crate::cli::commands::pack::{CliTarget, PackArgs};
use crate::cli::ops::pack::{
    build_header, configure_target, load_header, output_stem, resolve_inputs,
};

use super::{fixture_path, make_dom, write_rbxm};

fn pack_args(header: Option<PathBuf>) -> PackArgs {
    PackArgs {
        input: PathBuf::from("."),
        release: false,
        all: false,
        compat: false,
        targets: Vec::new(),
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
fn selected_targets_match_profile_flags() {
    let cases: &[(&[&str], &[CliTarget])] = &[
        (&["pack"], &[CliTarget::Dev]),
        (&["pack", "--release"], &[CliTarget::Rel]),
        (&["pack", "--all"], &[CliTarget::Dev, CliTarget::Rel]),
        (
            &["pack", "--compat"],
            &[CliTarget::Dev, CliTarget::DevCompat],
        ),
        (
            &["pack", "--release", "--compat"],
            &[CliTarget::Rel, CliTarget::RelCompat],
        ),
        (
            &["pack", "--all", "--compat"],
            &[
                CliTarget::Dev,
                CliTarget::DevCompat,
                CliTarget::Rel,
                CliTarget::RelCompat,
            ],
        ),
    ];

    for (argv, expected) in cases {
        let args = PackArgs::parse_from(*argv);
        assert_eq!(&args.selected_targets(), expected, "{argv:?}");
    }
}

#[test]
fn selected_targets_can_use_advanced_target_mode() {
    let args = PackArgs::parse_from(["pack", "--target", "rel,rel-compat"]);

    assert_eq!(
        args.selected_targets(),
        vec![CliTarget::Rel, CliTarget::RelCompat]
    );
}

#[test]
fn release_conflicts_with_all() {
    let result = PackArgs::try_parse_from(["pack", "--release", "--all"]);

    assert!(result.is_err());
}

#[test]
fn target_mode_conflicts_with_profile_flags() {
    let result = PackArgs::try_parse_from(["pack", "--target", "rel", "--compat"]);

    assert!(result.is_err());
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
fn build_header_places_custom_header_above_generated_header_with_one_separator() {
    assert_eq!(
        build_header(Some("-- custom header\n\n\r\n")),
        format!(
            "-- custom header\n-- Packed with rbxex v{}",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn build_header_uses_generated_header_when_custom_header_has_no_content() {
    let generated = format!("-- Packed with rbxex v{}", env!("CARGO_PKG_VERSION"));

    assert_eq!(build_header(None), generated);
    assert_eq!(build_header(Some("\r\n\n")), generated);
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
