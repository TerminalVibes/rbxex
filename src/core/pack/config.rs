use serde_json::{Value, json};

pub fn dev() -> Value {
    json!({
        "rules": [
            "remove_types",
            "remove_comments",
            "convert_index_to_field"
        ]
    })
}

pub fn dev_compat() -> Value {
    json!({
        "rules": [
            "remove_types",
            "remove_comments",
            "remove_compound_assignment",
            "remove_floor_division",
            "remove_if_expression",
            "remove_interpolated_string",
            "remove_continue",
            "convert_luau_number",
            "convert_index_to_field"
        ]
    })
}

pub fn minify() -> Value {
    json!({
        "generator": {
            "name": "dense",
            "column_span": 10000000000u64
        },
        "rules": [
            "remove_comments",
            "remove_types",
            "remove_spaces",
            "compute_expression",
            "remove_debug_profiling",
            "filter_after_early_return",
            "remove_unused_if_branch",
            "remove_unused_while",
            "convert_index_to_field",
            "convert_luau_number",
            "convert_square_root_call",
            "remove_function_call_parens",
            "remove_method_definition",
            "convert_local_function_to_assign",
            "remove_nil_declaration",
            "remove_empty_do",
            "group_local_assignment",
            {
                "rule": "rename_variables",
                "include_functions": true,
                "globals": ["$default", "$roblox", "__rbx", "__lua", "__env", "__start"]
            }
        ]
    })
}

pub fn minify_compat() -> Value {
    json!({
        "generator": {
            "name": "dense",
            "column_span": 10000000000u64
        },
        "rules": [
            "remove_comments",
            "remove_types",
            "remove_spaces",
            "compute_expression",
            "remove_debug_profiling",
            "filter_after_early_return",
            "remove_unused_if_branch",
            "remove_unused_while",
            "remove_compound_assignment",
            "remove_floor_division",
            "remove_if_expression",
            "remove_interpolated_string",
            "remove_continue",
            "convert_luau_number",
            "convert_index_to_field",
            "convert_square_root_call",
            "remove_function_call_parens",
            "remove_method_definition",
            "convert_local_function_to_assign",
            "remove_nil_declaration",
            "remove_empty_do",
            "group_local_assignment",
            {
                "rule": "rename_variables",
                "include_functions": true,
                "globals": ["$default", "$roblox", "__rbx", "__lua", "__env", "__start"]
            }
        ]
    })
}
