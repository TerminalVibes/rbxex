use anyhow::{Context, Result, anyhow, bail};
use darklua_core::{Configuration, Options, Parser, Resources};
use serde_json::Value;
use std::path::Path;

/// Processes Lua source code using Darklua with the provided configuration.
/// `file_name` is used for the virtual file path in the in-memory resource manager.
pub fn process_lua(source: String, id: &str, config: &Value) -> Result<String> {
    let config: Configuration =
        serde_json::from_value(config.clone()).context("Failed to parse darklua configuration")?;

    Parser::default()
        .parse(&source)
        .map_err(|error| anyhow!("Failed to parse Lua source: {}", error))?;

    let resources = Resources::from_memory();
    let virtual_path = format!("temp-{}.lua", id);

    resources
        .write(&virtual_path, &source)
        .map_err(|e| anyhow!("Failed to write to darklua resources: {:?}", e))?;

    let opts = Options::new(Path::new(&virtual_path)).with_configuration(config);

    match darklua_core::process(&resources, opts) {
        Ok(_) => {
            let result = resources
                .get(&virtual_path)
                .map_err(|e| anyhow!("Failed to retrieve preprocessed content: {:?}", e))?;

            Ok(result)
        }
        Err(e) => bail!("Darklua processing failed for '{}': {}", id, e),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::process_lua;

    #[test]
    fn process_lua_passthrough_with_empty_rules() {
        let output = process_lua(
            "local value = 1\nreturn value".to_string(),
            "passthrough",
            &json!({ "rules": [] }),
        )
        .unwrap();

        assert!(output.contains("local value = 1"));
        assert!(output.contains("return value"));
    }

    #[test]
    fn process_lua_remove_comments_strips_comment() {
        let output = process_lua(
            "-- comment\nlocal value = 1\nreturn value".to_string(),
            "comments",
            &json!({ "rules": ["remove_comments"] }),
        )
        .unwrap();

        assert!(!output.contains("comment"));
        assert!(output.contains("return value"));
    }

    #[test]
    fn process_lua_invalid_source_returns_err() {
        let result = process_lua(
            "local function broken(".to_string(),
            "broken",
            &json!({ "rules": ["remove_comments"] }),
        );
        assert!(result.is_err());
    }
}
