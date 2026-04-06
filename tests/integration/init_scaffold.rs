use std::borrow::Cow;
use std::fs;

use tempfile::tempdir;

use crate::cli::commands::init::{PackageManager, Template, ToolchainManager};
use crate::cli::ops::init::{
    ResolvedOptions, build_file_list, build_package_json, check_conflicts, format_command_failure,
    scaffold_files,
};

fn options(template: Template) -> ResolvedOptions {
    ResolvedOptions {
        name: "demo".to_string(),
        template,
        package_manager: PackageManager::Npm,
        toolchain_manager: Some(ToolchainManager::Rokit),
        rokit_available: false,
        git: true,
        eslint: true,
        prettier: true,
        vscode: true,
    }
}

fn file_names<'a>(files: &'a [(&'a str, Cow<'a, str>)]) -> Vec<&'a str> {
    files.iter().map(|(path, _)| *path).collect()
}

fn file_contents<'a>(files: &'a [(&str, Cow<'a, str>)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(candidate, _)| *candidate == path)
        .map(|(_, contents)| contents.as_ref())
        .unwrap()
}

#[test]
fn file_list_script_template_contains_client_ts() {
    let files = build_file_list(&options(Template::Script));
    assert!(file_names(&files).contains(&"src/index.client.ts"));
}

#[test]
fn file_list_script_template_contains_default_project_json() {
    let files = build_file_list(&options(Template::Script));
    assert!(file_names(&files).contains(&"default.project.json"));
}

#[test]
fn file_list_package_template_omits_default_project_json() {
    let files = build_file_list(&options(Template::Package));
    assert!(!file_names(&files).contains(&"default.project.json"));
}

#[test]
fn file_list_with_eslint_contains_eslint_config() {
    let files = build_file_list(&options(Template::Script));
    assert!(file_names(&files).contains(&"eslint.config.mjs"));
}

#[test]
fn file_list_without_eslint_omits_eslint_config() {
    let mut opts = options(Template::Script);
    opts.eslint = false;

    let files = build_file_list(&opts);

    assert!(!file_names(&files).contains(&"eslint.config.mjs"));
}

#[test]
fn file_list_with_prettier_contains_prettier_config() {
    let files = build_file_list(&options(Template::Script));
    assert!(file_names(&files).contains(&"prettier.config.mjs"));
}

#[test]
fn file_list_with_vscode_contains_both_vscode_files() {
    let files = build_file_list(&options(Template::Script));
    let names = file_names(&files);

    assert!(names.contains(&".vscode/settings.json"));
    assert!(names.contains(&".vscode/extensions.json"));
}

#[test]
fn file_list_rokit_toolchain_contains_rokit_toml() {
    let files = build_file_list(&options(Template::Script));
    assert!(file_names(&files).contains(&"rokit.toml"));
}

#[test]
fn file_list_no_toolchain_omits_all_toml_files() {
    let mut opts = options(Template::Script);
    opts.toolchain_manager = None;

    let files = build_file_list(&opts);
    let names = file_names(&files);

    assert!(!names.contains(&"rokit.toml"));
    assert!(!names.contains(&"aftman.toml"));
    assert!(!names.contains(&"foreman.toml"));
    assert!(!names.contains(&"mise.toml"));
}

#[test]
fn file_list_always_contains_package_json_tsconfig_gitignore() {
    let files = build_file_list(&options(Template::Script));
    let names = file_names(&files);

    assert!(names.contains(&"package.json"));
    assert!(names.contains(&"tsconfig.json"));
    assert!(names.contains(&".gitignore"));
}

#[test]
fn file_list_version_placeholder_is_substituted() {
    let files = build_file_list(&options(Template::Script));

    assert!(
        files
            .iter()
            .all(|(_, contents)| !contents.contains("{{rbxex_version}}"))
    );
}

#[test]
fn package_json_package_template_name_has_executor_scope() {
    let package_json = build_package_json(&options(Template::Package));
    assert_eq!(package_json["name"].as_str(), Some("@executor-ts/demo"));
}

#[test]
fn package_json_script_template_name_is_bare() {
    let package_json = build_package_json(&options(Template::Script));
    assert_eq!(package_json["name"].as_str(), Some("demo"));
}

#[test]
fn package_json_eslint_and_prettier_includes_bridge_deps() {
    let package_json = build_package_json(&options(Template::Script));
    let dev_deps = package_json["devDependencies"].as_object().unwrap();

    assert!(dev_deps.contains_key("eslint-config-prettier"));
    assert!(dev_deps.contains_key("eslint-plugin-prettier"));
}

#[test]
fn package_json_package_template_has_prepublish_script() {
    let package_json = build_package_json(&options(Template::Package));
    assert_eq!(
        package_json["scripts"]["prepublishOnly"].as_str(),
        Some("npm run build")
    );
}

#[test]
fn check_conflicts_empty_dir_ok() {
    let dir = tempdir().unwrap();
    let files = build_file_list(&options(Template::Script));

    check_conflicts(dir.path(), &files).unwrap();
}

#[test]
fn check_conflicts_existing_file_returns_err() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    let files = build_file_list(&options(Template::Script));

    let result = check_conflicts(dir.path(), &files);

    assert!(result.is_err());
}

#[test]
fn scaffold_files_creates_nested_dirs_and_writes_content() {
    let dir = tempdir().unwrap();
    let files = build_file_list(&options(Template::Package));

    scaffold_files(dir.path(), &files).unwrap();

    assert!(dir.path().join("src/index.ts").is_file());
    assert!(dir.path().join(".vscode/settings.json").is_file());
    assert_eq!(
        fs::read_to_string(dir.path().join(".vscode/settings.json")).unwrap(),
        file_contents(&files, ".vscode/settings.json")
    );
}

#[test]
fn format_command_failure_includes_both_streams_when_present() {
    let rendered = format_command_failure(
        "npm install",
        Some(1),
        b"installing dependencies\n",
        b"npm ERR! dependency conflict\n",
    );

    assert!(rendered.contains("`npm install` failed with exit code 1"));
    assert!(rendered.contains("stderr:\nnpm ERR! dependency conflict"));
    assert!(rendered.contains("stdout:\ninstalling dependencies"));
}

#[test]
fn format_command_failure_ignores_empty_streams() {
    let rendered = format_command_failure("git init", Some(128), b"\n", b"fatal: not a repo\n");

    assert_eq!(
        rendered,
        "`git init` failed with exit code 128:\nfatal: not a repo"
    );
}
