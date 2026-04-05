use anyhow::Result;
use std::fmt::Write;

use crate::core::pack::transform::process_lua;
use crate::core::pack::{BundleOptions, literal::append_luau_string};

/// This generates the luau body for a script, injecting environment setup and
/// optionally preprocessing with Darklua.
pub fn generate_script_body(
    id: usize,
    raw_source: &str,
    chunk_name: &str,
    options: &BundleOptions,
) -> Result<String> {
    let mut source = inject_env(id, raw_source, options.sourcemap)?;
    if let Some(ref config) = options.preprocess {
        source = process_lua(source, chunk_name, config)?;
    }
    if options.sourcemap {
        let mut output = String::with_capacity(source.len() + 128);
        output.push_str("return assert(loadstring(");
        append_luau_string(&mut output, &source);
        output.push_str(", ");
        append_luau_string(&mut output, chunk_name);
        output.push_str("))(__env)");
        Ok(output)
    } else {
        Ok(source)
    }
}

/// Injects the environment setup code into the source.
fn inject_env(id: usize, src: &str, sourcemap: bool) -> Result<String> {
    let mut output = String::with_capacity(src.len() + 80);
    output.push_str("local _=");
    output.push_str(if sourcemap { "(...)" } else { "__env" });
    output.push('(');
    write!(output, "{}", id)?;
    output.push_str(");local script,require=_.script,_.require;");
    output.push_str(src);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{generate_script_body, inject_env};
    use crate::core::pack::BundleOptions;

    #[test]
    fn inject_env_sourcemap_uses_spread_args() {
        let output = inject_env(7, "return 42", true).unwrap();
        assert!(output.starts_with("local _=(...)(7);"));
    }

    #[test]
    fn inject_env_no_sourcemap_uses_env_name() {
        let output = inject_env(7, "return 42", false).unwrap();
        assert!(output.starts_with("local _=__env(7);"));
    }

    #[test]
    fn generate_script_body_sourcemap_wraps_in_loadstring() {
        let output = generate_script_body(
            1,
            "return 42",
            "Game.Module",
            &BundleOptions {
                sourcemap: true,
                ..BundleOptions::default()
            },
        )
        .unwrap();

        assert!(output.starts_with("return assert(loadstring("));
        assert!(output.contains("))(__env)"));
    }

    #[test]
    fn generate_script_body_no_sourcemap_no_loadstring() {
        let output =
            generate_script_body(1, "return 42", "Game.Module", &BundleOptions::default()).unwrap();

        assert!(output.starts_with("local _=__env(1);"));
        assert!(!output.contains("loadstring("));
    }
}
