use jsonschema::Validator;
use schemars::schema_for;

use crate::types::config::RashConfig;
use crate::types::error::{ErrorEntry, E_SCHEMA_VIOLATION};

/// Generate a JSON Schema for `RashConfig`.
pub fn generate_config_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(RashConfig)).expect("schema serialization should not fail")
}

/// Validate a JSON value against a JSON Schema, returning errors in `ErrorEntry` format.
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> Vec<ErrorEntry> {
    let compiled = match Validator::new(schema) {
        Ok(v) => v,
        Err(e) => {
            return vec![ErrorEntry::error(
                E_SCHEMA_VIOLATION,
                format!("Invalid schema: {e}"),
                "",
                "$",
            )];
        }
    };

    compiled
        .iter_errors(value)
        .map(|err| {
            let instance_path = err.instance_path.as_str();
            let path = if instance_path.is_empty() {
                "$".to_string()
            } else {
                format!("${instance_path}")
            };
            ErrorEntry::error(E_SCHEMA_VIOLATION, err.to_string(), "", &path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn valid_config_json() -> serde_json::Value {
        serde_json::json!({
            "version": "1.0.0",
            "name": "my-server",
            "target": {
                "language": "typescript",
                "framework": "express",
                "runtime": "bun"
            },
            "server": {
                "port": 3000,
                "host": "0.0.0.0"
            }
        })
    }

    #[test]
    fn generated_schema_is_valid_json_schema() {
        let schema = generate_config_schema();

        // Should be an object with standard JSON Schema fields
        assert!(schema.is_object());
        assert!(schema.get("$schema").is_some() || schema.get("type").is_some());
        assert!(schema.get("properties").is_some() || schema.get("$ref").is_some());
    }

    #[test]
    fn valid_config_passes_validation() {
        let schema = generate_config_schema();
        let config = valid_config_json();

        let errors = validate_against_schema(&config, &schema);
        assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
    }

    #[test]
    fn invalid_config_missing_required_fields() {
        let schema = generate_config_schema();
        let config = serde_json::json!({
            "version": "1.0.0"
            // missing: name, target, server
        });

        let errors = validate_against_schema(&config, &schema);
        assert!(!errors.is_empty(), "Expected errors for missing fields");

        // All errors should use E_SCHEMA_VIOLATION code
        for err in &errors {
            assert_eq!(err.code, E_SCHEMA_VIOLATION);
        }
    }

    #[test]
    fn invalid_config_wrong_types() {
        let schema = generate_config_schema();
        let config = serde_json::json!({
            "version": 123,  // should be string
            "name": true,    // should be string
            "target": "not an object",
            "server": "not an object"
        });

        let errors = validate_against_schema(&config, &schema);
        assert!(!errors.is_empty(), "Expected errors for wrong types");
    }

    #[test]
    fn invalid_config_wrong_enum_value() {
        let schema = generate_config_schema();
        let config = serde_json::json!({
            "version": "1.0.0",
            "name": "test",
            "target": {
                "language": "cobol",   // invalid language
                "framework": "express",
                "runtime": "bun"
            },
            "server": {
                "port": 3000,
                "host": "0.0.0.0"
            }
        });

        let errors = validate_against_schema(&config, &schema);
        assert!(!errors.is_empty(), "Expected errors for invalid enum value");
    }

    #[test]
    fn error_entries_have_path_info() {
        let schema = generate_config_schema();
        let config = serde_json::json!({
            "version": "1.0.0"
            // missing required fields
        });

        let errors = validate_against_schema(&config, &schema);
        assert!(!errors.is_empty());

        // Root-level errors should have "$" path
        for err in &errors {
            assert!(err.path.starts_with('$'), "Path should start with $: {}", err.path);
        }
    }

    #[test]
    fn valid_full_config_passes() {
        let schema = generate_config_schema();
        let config = serde_json::json!({
            "$schema": "https://rash.dev/schemas/config.json",
            "version": "1.0.0",
            "name": "my-server",
            "description": "My awesome server",
            "target": {
                "language": "typescript",
                "framework": "express",
                "runtime": "bun"
            },
            "server": {
                "port": 3000,
                "host": "0.0.0.0",
                "protocol": "http",
                "basePath": "/api"
            },
            "database": {
                "type": "postgresql",
                "orm": "prisma"
            },
            "codegen": {
                "outDir": "./dist",
                "sourceMap": true,
                "strict": true
            },
            "middleware": {
                "global": [
                    { "ref": "cors" }
                ]
            },
            "meta": {
                "createdAt": "2026-01-15T00:00:00Z",
                "rashVersion": "0.1.0"
            }
        });

        let errors = validate_against_schema(&config, &schema);
        assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
    }
}
