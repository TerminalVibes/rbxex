use rbx_dom_weak::{InstanceBuilder, WeakDom};

use rbxex::core::pack::{BundleOptions, bundle, config};

use super::make_dom;

#[test]
fn bundle_empty_dom_returns_err() {
    let dom = WeakDom::new(InstanceBuilder::new("DataModel"));
    let result = bundle(&dom, BundleOptions::default());
    assert!(result.is_err());
}

#[test]
fn bundle_multi_root_returns_err() {
    let dom = make_dom(&[
        ("First", "ModuleScript", "return 1"),
        ("Second", "ModuleScript", "return 2"),
    ]);
    let result = bundle(&dom, BundleOptions::default());
    assert!(result.is_err());
}

#[test]
fn bundle_single_module_script_contains_runtime_shim() {
    let dom = make_dom(&[("Main", "ModuleScript", "return 42")]);
    let output = bundle(&dom, BundleOptions::default()).unwrap();

    assert!(output.contains("__rbx"));
    assert!(output.contains("__lua"));
    assert!(output.contains("__start"));
}

#[test]
fn bundle_single_module_script_contains_start_call() {
    let dom = make_dom(&[("Main", "ModuleScript", "return 42")]);
    let output = bundle(&dom, BundleOptions::default()).unwrap();

    assert!(output.trim_end().ends_with("__start()"));
}

#[test]
fn bundle_local_script_uses_lua_macro() {
    let dom = make_dom(&[("Client", "LocalScript", "print('hi')")]);
    let output = bundle(&dom, BundleOptions::default()).unwrap();

    assert!(output.contains(r#"__lua(1, 0, "Client", "LocalScript", function()"#));
}

#[test]
fn bundle_folder_uses_rbx_macro() {
    let dom = make_dom(&[("Workspace", "Folder", "")]);
    let output = bundle(&dom, BundleOptions::default()).unwrap();

    assert!(output.contains(r#"__rbx(1, 0, "Workspace", "Folder")"#));
}

#[test]
fn bundle_script_source_is_embedded() {
    let dom = make_dom(&[("Main", "ModuleScript", "return 42")]);
    let output = bundle(&dom, BundleOptions::default()).unwrap();

    assert!(output.contains("return 42"));
}

#[test]
fn bundle_dev_target_produces_loadstring_wrapper() {
    let dom = make_dom(&[("Main", "ModuleScript", "return 42")]);
    let output = bundle(
        &dom,
        BundleOptions {
            sourcemap: true,
            preprocess: Some(config::dev()),
            postprocess: None,
        },
    )
    .unwrap();

    assert!(output.contains("loadstring("));
    assert!(output.contains(")(__env)"));
}

#[test]
fn bundle_rel_target_no_loadstring_wrapper() {
    let dom = make_dom(&[("Main", "ModuleScript", "local value = 1\nreturn value")]);
    let output = bundle(
        &dom,
        BundleOptions {
            sourcemap: false,
            preprocess: None,
            postprocess: Some(config::minify()),
        },
    )
    .unwrap();

    assert!(!output.contains("loadstring("));
}

#[test]
fn bundle_child_instance_parent_id_is_nonzero() {
    let dom = WeakDom::new(
        InstanceBuilder::new("DataModel").with_child(
            InstanceBuilder::new("Folder").with_name("Root").with_child(
                InstanceBuilder::new("ModuleScript")
                    .with_name("Child")
                    .with_property("Source", "return 42"),
            ),
        ),
    );
    let output = bundle(&dom, BundleOptions::default()).unwrap();

    assert!(output.contains("__lua(2, 1,"));
}
