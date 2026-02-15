use std::path::Path;

use crate::types::config::RashConfig;
use crate::types::error::{ErrorEntry, E_PARSE_ERROR};
use crate::types::handler::HandlerSpec;
use crate::types::middleware::MiddlewareSpec;
use crate::types::model::ModelSpec;
use crate::types::route::RouteSpec;
use crate::types::schema::SchemaSpec;

/// Parse a rash.config.json file
#[allow(clippy::result_large_err)]
pub fn parse_config(content: &str, file_path: &str) -> Result<RashConfig, ErrorEntry> {
    serde_json::from_str(content).map_err(|e| serde_error_to_entry(e, file_path))
}

/// Parse a *.route.json file
#[allow(clippy::result_large_err)]
pub fn parse_route(content: &str, file_path: &str) -> Result<RouteSpec, ErrorEntry> {
    serde_json::from_str(content).map_err(|e| serde_error_to_entry(e, file_path))
}

/// Parse a *.schema.json file
#[allow(clippy::result_large_err)]
pub fn parse_schema(content: &str, file_path: &str) -> Result<SchemaSpec, ErrorEntry> {
    serde_json::from_str(content).map_err(|e| serde_error_to_entry(e, file_path))
}

/// Parse a *.model.json file
#[allow(clippy::result_large_err)]
pub fn parse_model(content: &str, file_path: &str) -> Result<ModelSpec, ErrorEntry> {
    serde_json::from_str(content).map_err(|e| serde_error_to_entry(e, file_path))
}

/// Parse a *.middleware.json file
#[allow(clippy::result_large_err)]
pub fn parse_middleware(content: &str, file_path: &str) -> Result<MiddlewareSpec, ErrorEntry> {
    serde_json::from_str(content).map_err(|e| serde_error_to_entry(e, file_path))
}

/// Parse a *.handler.json file
#[allow(clippy::result_large_err)]
pub fn parse_handler(content: &str, file_path: &str) -> Result<HandlerSpec, ErrorEntry> {
    serde_json::from_str(content).map_err(|e| serde_error_to_entry(e, file_path))
}

/// Detect spec file type from file name
pub fn detect_spec_type(file_path: &Path) -> Option<SpecFileType> {
    let name = file_path.file_name()?.to_str()?;
    if name == "rash.config.json" {
        Some(SpecFileType::Config)
    } else if name.ends_with(".route.json") {
        Some(SpecFileType::Route)
    } else if name.ends_with(".schema.json") {
        Some(SpecFileType::Schema)
    } else if name.ends_with(".model.json") {
        Some(SpecFileType::Model)
    } else if name.ends_with(".middleware.json") {
        Some(SpecFileType::Middleware)
    } else if name.ends_with(".handler.json") {
        Some(SpecFileType::Handler)
    } else {
        None
    }
}

/// Spec file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecFileType {
    Config,
    Route,
    Schema,
    Model,
    Middleware,
    Handler,
}

/// Convert a serde_json error into a structured ErrorEntry
fn serde_error_to_entry(err: serde_json::Error, file_path: &str) -> ErrorEntry {
    let line = err.line();
    let col = err.column();
    let path = format!("$.line:{line}:col:{col}");

    ErrorEntry::error(
        E_PARSE_ERROR,
        format!("JSON parse error: {err}"),
        file_path,
        &path,
    )
    .with_suggestion("Check JSON syntax and field types")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let json = r#"{
            "version": "1.0.0",
            "name": "test",
            "target": { "language": "typescript", "framework": "express", "runtime": "bun" },
            "server": { "port": 3000, "host": "0.0.0.0" }
        }"#;

        let config = parse_config(json, "rash.config.json").unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.version, "1.0.0");
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"{ invalid json }"#;
        let err = parse_config(json, "rash.config.json").unwrap_err();
        assert_eq!(err.code, E_PARSE_ERROR);
        assert_eq!(err.file, "rash.config.json");
        assert!(err.message.contains("JSON parse error"));
    }

    #[test]
    fn test_parse_valid_route() {
        let json = r#"{
            "path": "/v1/users",
            "methods": {
                "GET": {
                    "operationId": "listUsers",
                    "handler": { "ref": "users.listUsers" }
                }
            }
        }"#;

        let route = parse_route(json, "routes/users.route.json").unwrap();
        assert_eq!(route.path, "/v1/users");
    }

    #[test]
    fn test_parse_valid_schema() {
        let json = r#"{
            "name": "User",
            "definitions": {
                "CreateUserBody": {
                    "type": "object",
                    "properties": {
                        "email": { "type": "string" }
                    }
                }
            }
        }"#;

        let schema = parse_schema(json, "schemas/user.schema.json").unwrap();
        assert_eq!(schema.name, "User");
    }

    #[test]
    fn test_parse_valid_model() {
        let json = r#"{
            "name": "User",
            "tableName": "users",
            "columns": {
                "id": { "type": "uuid", "primaryKey": true }
            }
        }"#;

        let model = parse_model(json, "models/user.model.json").unwrap();
        assert_eq!(model.name, "User");
    }

    #[test]
    fn test_parse_valid_middleware() {
        let json = r#"{
            "name": "auth",
            "type": "request",
            "handler": { "ref": "auth.verifyToken" }
        }"#;

        let mw = parse_middleware(json, "middleware/auth.middleware.json").unwrap();
        assert_eq!(mw.name, "auth");
    }

    #[test]
    fn test_detect_spec_type() {
        assert_eq!(
            detect_spec_type(Path::new("rash.config.json")),
            Some(SpecFileType::Config)
        );
        assert_eq!(
            detect_spec_type(Path::new("users.route.json")),
            Some(SpecFileType::Route)
        );
        assert_eq!(
            detect_spec_type(Path::new("user.schema.json")),
            Some(SpecFileType::Schema)
        );
        assert_eq!(
            detect_spec_type(Path::new("user.model.json")),
            Some(SpecFileType::Model)
        );
        assert_eq!(
            detect_spec_type(Path::new("auth.middleware.json")),
            Some(SpecFileType::Middleware)
        );
        assert_eq!(
            detect_spec_type(Path::new("users.handler.json")),
            Some(SpecFileType::Handler)
        );
        assert_eq!(detect_spec_type(Path::new("README.md")), None);
    }

    #[test]
    fn test_serde_error_includes_location() {
        let json = r#"{
            "version": 123
        }"#;
        let err = parse_config(json, "rash.config.json").unwrap_err();
        assert!(err.path.contains("line:"));
        assert!(err.path.contains("col:"));
    }
}
