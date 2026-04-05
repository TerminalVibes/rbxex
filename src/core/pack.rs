use anyhow::Result;
use rbx_dom_weak::WeakDom;
use serde_json::Value;

use self::manifest::write_manifest_serial;
pub(crate) mod codegen;
pub mod config;
pub(crate) mod literal;
pub(crate) mod manifest;
pub(crate) mod transform;

#[derive(Debug, Clone, Default)]
pub struct BundleOptions {
    /// Enables source mapping using `loadstring` to preserve original file paths and line numbers in stack traces.
    /// Note: This does not work in environments that do not support `loadstring`.
    pub sourcemap: bool,

    /// Darklua config to apply to each individual script before bundling.
    pub preprocess: Option<Value>,

    /// Darklua config to apply to the *final bundled script*.
    pub postprocess: Option<Value>,
}

/// A "pure" function that bundles the given DOM according to the specified options.
pub fn bundle(dom: &WeakDom, options: BundleOptions) -> Result<String> {
    let mut source = String::with_capacity(64 * 1024);
    // insert the runtime shim
    source.push_str("-- Runtime Library\n");
    source.push_str("-- This is the runtime shim for environment virtualization.\n");
    source.push_str(include_str!("./pack/runtime.lua").trim());
    source.push('\n');

    // insert instance tree manifest
    source.push_str("-- Instance Tree Manifest\n");
    source.push_str("-- This is the generated instance hierarchy for the bundle.\n");
    write_manifest_serial(&mut source, dom, &options)?;

    // insert the entrypoint
    source.push_str("__start()\n");

    // postprocess final source if specified
    if let Some(config) = options.postprocess {
        source = transform::process_lua(source, "post-processing", &config)?;
    }
    Ok(source)
}
