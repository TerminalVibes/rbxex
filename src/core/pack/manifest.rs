use anyhow::{Context, Result};
use rbx_dom_weak::{
    WeakDom,
    types::{Ref, Variant},
};
use std::fmt::Write;
use ustr::ustr;

use crate::core::pack::{
    BundleOptions, codegen::generate_script_body, literal::append_luau_string,
};

pub fn write_manifest_serial(
    buffer: &mut String,
    dom: &WeakDom,
    options: &BundleOptions,
) -> Result<()> {
    let source_key = ustr("Source");

    let root_children = dom.root().children();
    if root_children.is_empty() {
        anyhow::bail!("Model file contains no instances");
    }
    if root_children.len() > 1 {
        anyhow::bail!(
            "Model file contains multiple top-level instances ({}). Expected exactly one.",
            root_children.len()
        );
    }

    // stack tuple: (Referent, Parent_ID)
    let mut stack = vec![(root_children[0], 0usize)];
    let mut next_id = 1;

    // depth-first traversal
    while let Some((ref_token, parent_id)) = stack.pop() {
        let instance = dom.get_by_ref(ref_token).context("Instance lost")?;
        let current_id = next_id;
        next_id += 1;
        if matches!(instance.class.as_str(), "LocalScript" | "ModuleScript") {
            let source_owned: String;
            let source = match instance.properties.get(&source_key) {
                Some(Variant::String(s)) => s.as_str(),
                Some(Variant::BinaryString(b)) => {
                    source_owned = String::from_utf8_lossy(b.as_ref()).into_owned();
                    &source_owned
                }
                _ => "",
            };
            let chunk_name = get_path(dom, ref_token);
            write_lua(
                buffer,
                (current_id, parent_id),
                (&instance.name, &instance.class),
                &generate_script_body(current_id, source, &chunk_name, options)?,
            )?;
        } else {
            write_rbx(
                buffer,
                (current_id, parent_id),
                (&instance.name, &instance.class),
            )?;
        }
        // push children in reverse (ordering)
        for child_ref in instance.children().iter().rev() {
            stack.push((*child_ref, current_id));
        }
    }

    Ok(())
}

/// Write a single Lua virtual instance to the buffer.
/// `Source` should be a valid Lua function body as a string.
pub(crate) fn write_lua(
    buffer: &mut String,
    (id, parent): (usize, usize),
    (name, class): (&str, &str),
    embed: &str,
) -> Result<()> {
    buffer.push_str("__lua(");
    write!(buffer, "{id}, {parent}, ")?;
    append_luau_string(buffer, name);
    buffer.push_str(", ");
    append_luau_string(buffer, class);
    buffer.push_str(", function()\n\t");
    buffer.push_str(embed);
    buffer.push_str("\nend)\n");
    Ok(())
}

/// Write a single generic virtual instance to the buffer.
/// These instances are generic and are only meant to represent structure.
pub(crate) fn write_rbx(
    buffer: &mut String,
    (id, parent): (usize, usize),
    (name, class): (&str, &str),
) -> Result<()> {
    buffer.push_str("__rbx(");
    write!(buffer, "{id}, {parent}, ")?;
    append_luau_string(buffer, name);
    buffer.push_str(", ");
    append_luau_string(buffer, class);
    buffer.push_str(")\n");
    Ok(())
}

/// Constructs the full path of an instance in the DOM, separated by dots.
/// This might not be unique or lua-safe, but is useful for debugging.
fn get_path(dom: &WeakDom, initial_ref: Ref) -> String {
    fn write_path(dom: &WeakDom, referent: Ref, buffer: &mut String) {
        if let Some(instance) = dom.get_by_ref(referent) {
            if referent != dom.root_ref() {
                write_path(dom, instance.parent(), buffer);
                buffer.push('.');
            }
            buffer.push_str(&instance.name);
        }
    }
    let mut buffer = String::with_capacity(64);
    write_path(dom, initial_ref, &mut buffer);
    buffer
}

#[cfg(test)]
mod tests {
    use rbx_dom_weak::{InstanceBuilder, WeakDom};

    use super::{write_lua, write_manifest_serial, write_rbx};
    use crate::core::pack::BundleOptions;

    fn dom_with_children(children: impl IntoIterator<Item = InstanceBuilder>) -> WeakDom {
        WeakDom::new(InstanceBuilder::new("DataModel").with_children(children))
    }

    #[test]
    fn write_lua_produces_lua_macro_call() {
        let mut buffer = String::new();
        write_lua(&mut buffer, (1, 0), ("Main", "ModuleScript"), "return 42").unwrap();
        assert!(buffer.starts_with("__lua("));
    }

    #[test]
    fn write_rbx_produces_rbx_macro_call() {
        let mut buffer = String::new();
        write_rbx(&mut buffer, (1, 0), ("Folder", "Folder")).unwrap();
        assert!(buffer.starts_with("__rbx("));
    }

    #[test]
    fn write_manifest_single_module_script_id_1_parent_0() {
        let dom = dom_with_children([InstanceBuilder::new("ModuleScript")
            .with_name("Main")
            .with_property("Source", "return 42")]);
        let mut buffer = String::new();

        write_manifest_serial(&mut buffer, &dom, &BundleOptions::default()).unwrap();

        assert!(buffer.contains("__lua(1, 0,"));
    }

    #[test]
    fn write_manifest_child_gets_parent_id() {
        let dom = dom_with_children(
            [InstanceBuilder::new("Folder").with_name("Root").with_child(
                InstanceBuilder::new("ModuleScript")
                    .with_name("Child")
                    .with_property("Source", "return 42"),
            )],
        );
        let mut buffer = String::new();

        write_manifest_serial(&mut buffer, &dom, &BundleOptions::default()).unwrap();

        assert!(buffer.contains("__lua(2, 1,"));
    }

    #[test]
    fn write_manifest_empty_root_returns_err() {
        let dom = WeakDom::new(InstanceBuilder::new("DataModel"));
        let mut buffer = String::new();

        let result = write_manifest_serial(&mut buffer, &dom, &BundleOptions::default());

        assert!(result.is_err());
    }

    #[test]
    fn write_manifest_multiple_top_level_returns_err() {
        let dom = dom_with_children([
            InstanceBuilder::new("ModuleScript")
                .with_name("One")
                .with_property("Source", "return 1"),
            InstanceBuilder::new("ModuleScript")
                .with_name("Two")
                .with_property("Source", "return 2"),
        ]);
        let mut buffer = String::new();

        let result = write_manifest_serial(&mut buffer, &dom, &BundleOptions::default());

        assert!(result.is_err());
    }
}
