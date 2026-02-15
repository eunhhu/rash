use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::parser::{self, SpecFileType};
use crate::types::config::RashConfig;
use crate::types::error::{ErrorEntry, ValidationReport, E_PARSE_ERROR};
use crate::types::handler::HandlerSpec;
use crate::types::middleware::MiddlewareSpec;
use crate::types::model::ModelSpec;
use crate::types::route::RouteSpec;
use crate::types::schema::SchemaSpec;

/// A loaded Rash project with all parsed spec files
#[derive(Debug, Clone)]
pub struct LoadedProject {
    /// Project root directory
    pub root: PathBuf,
    /// Parsed project config
    pub config: RashConfig,
    /// All parsed routes with their relative file paths
    pub routes: Vec<(String, RouteSpec)>,
    /// All parsed schemas with their relative file paths
    pub schemas: Vec<(String, SchemaSpec)>,
    /// All parsed models with their relative file paths
    pub models: Vec<(String, ModelSpec)>,
    /// All parsed middleware with their relative file paths
    pub middleware: Vec<(String, MiddlewareSpec)>,
    /// All parsed handlers with their relative file paths
    pub handlers: Vec<(String, HandlerSpec)>,
}

/// Load and parse an entire Rash project directory.
/// Accumulates errors instead of failing on the first one.
#[allow(clippy::result_large_err)]
pub fn load_project(project_dir: &Path) -> Result<(LoadedProject, ValidationReport), LoadError> {
    let mut report = ValidationReport::success();

    // Check directory exists
    if !project_dir.is_dir() {
        return Err(LoadError::ProjectNotFound(
            project_dir.to_string_lossy().into_owned(),
        ));
    }

    // Load config first (required)
    let config_path = project_dir.join("rash.config.json");
    if !config_path.exists() {
        return Err(LoadError::ConfigNotFound(
            config_path.to_string_lossy().into_owned(),
        ));
    }

    let config_content = std::fs::read_to_string(&config_path)
        .map_err(|e| LoadError::IoError(config_path.to_string_lossy().into_owned(), e))?;

    let config = match parser::parse_config(&config_content, "rash.config.json") {
        Ok(c) => c,
        Err(entry) => {
            return Err(LoadError::ConfigParseError(entry));
        }
    };

    let mut routes = Vec::new();
    let mut schemas = Vec::new();
    let mut models = Vec::new();
    let mut middleware = Vec::new();
    let mut handlers = Vec::new();

    // Walk directory and collect spec files
    for entry in WalkDir::new(project_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel_path = path
            .strip_prefix(project_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();

        // Skip .rash directory and config (already loaded)
        if rel_path.starts_with(".rash") || rel_path == "rash.config.json" {
            continue;
        }

        let Some(spec_type) = parser::detect_spec_type(path) else {
            continue;
        };

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.push(ErrorEntry::error(
                    E_PARSE_ERROR,
                    format!("Failed to read file: {e}"),
                    &rel_path,
                    "$",
                ));
                continue;
            }
        };

        match spec_type {
            SpecFileType::Config => {
                // Already loaded above
            }
            SpecFileType::Route => match parser::parse_route(&content, &rel_path) {
                Ok(route) => routes.push((rel_path, route)),
                Err(entry) => report.push(entry),
            },
            SpecFileType::Schema => match parser::parse_schema(&content, &rel_path) {
                Ok(schema) => schemas.push((rel_path, schema)),
                Err(entry) => report.push(entry),
            },
            SpecFileType::Model => match parser::parse_model(&content, &rel_path) {
                Ok(model) => models.push((rel_path, model)),
                Err(entry) => report.push(entry),
            },
            SpecFileType::Middleware => match parser::parse_middleware(&content, &rel_path) {
                Ok(mw) => middleware.push((rel_path, mw)),
                Err(entry) => report.push(entry),
            },
            SpecFileType::Handler => match parser::parse_handler(&content, &rel_path) {
                Ok(handler) => handlers.push((rel_path, handler)),
                Err(entry) => report.push(entry),
            },
        }
    }

    let project = LoadedProject {
        root: project_dir.to_path_buf(),
        config,
        routes,
        schemas,
        models,
        middleware,
        handlers,
    };

    Ok((project, report))
}

/// Errors that prevent project loading entirely
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("Project directory not found: {0}")]
    ProjectNotFound(String),

    #[error("rash.config.json not found: {0}")]
    ConfigNotFound(String),

    #[error("Failed to parse rash.config.json: {0:?}")]
    ConfigParseError(ErrorEntry),

    #[error("I/O error reading {0}: {1}")]
    IoError(String, std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_minimal_project(dir: &Path) {
        std::fs::write(
            dir.join("rash.config.json"),
            r#"{
                "version": "1.0.0",
                "name": "test-project",
                "target": { "language": "typescript", "framework": "express", "runtime": "bun" },
                "server": { "port": 3000, "host": "0.0.0.0" }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn test_load_minimal_project() {
        let tmp = TempDir::new().unwrap();
        create_minimal_project(tmp.path());

        let (project, report) = load_project(tmp.path()).unwrap();
        assert!(report.ok);
        assert_eq!(project.config.name, "test-project");
        assert!(project.routes.is_empty());
        assert!(project.schemas.is_empty());
    }

    #[test]
    fn test_load_project_with_route() {
        let tmp = TempDir::new().unwrap();
        create_minimal_project(tmp.path());

        let routes_dir = tmp.path().join("routes");
        std::fs::create_dir_all(&routes_dir).unwrap();
        std::fs::write(
            routes_dir.join("health.route.json"),
            r#"{
                "path": "/health",
                "methods": {
                    "GET": {
                        "operationId": "healthCheck",
                        "handler": { "ref": "health.check" }
                    }
                }
            }"#,
        )
        .unwrap();

        let (project, report) = load_project(tmp.path()).unwrap();
        assert!(report.ok);
        assert_eq!(project.routes.len(), 1);
        assert_eq!(project.routes[0].1.path, "/health");
    }

    #[test]
    fn test_load_project_accumulates_errors() {
        let tmp = TempDir::new().unwrap();
        create_minimal_project(tmp.path());

        let routes_dir = tmp.path().join("routes");
        std::fs::create_dir_all(&routes_dir).unwrap();

        // Valid route
        std::fs::write(
            routes_dir.join("good.route.json"),
            r#"{
                "path": "/good",
                "methods": { "GET": { "handler": { "ref": "good.handler" } } }
            }"#,
        )
        .unwrap();

        // Invalid route
        std::fs::write(routes_dir.join("bad.route.json"), r#"{ invalid json }"#).unwrap();

        let (project, report) = load_project(tmp.path()).unwrap();
        assert!(!report.ok);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].code, E_PARSE_ERROR);
        // Good route was still loaded
        assert_eq!(project.routes.len(), 1);
    }

    #[test]
    fn test_load_nonexistent_directory() {
        let err = load_project(Path::new("/nonexistent/path")).unwrap_err();
        assert!(matches!(err, LoadError::ProjectNotFound(_)));
    }

    #[test]
    fn test_load_missing_config() {
        let tmp = TempDir::new().unwrap();
        let err = load_project(tmp.path()).unwrap_err();
        assert!(matches!(err, LoadError::ConfigNotFound(_)));
    }

    #[test]
    fn test_load_invalid_config() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("rash.config.json"), "not json").unwrap();
        let err = load_project(tmp.path()).unwrap_err();
        assert!(matches!(err, LoadError::ConfigParseError(_)));
    }

    #[test]
    fn test_load_project_with_all_spec_types() {
        let tmp = TempDir::new().unwrap();
        create_minimal_project(tmp.path());

        let schemas_dir = tmp.path().join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();
        std::fs::write(
            schemas_dir.join("user.schema.json"),
            r#"{ "name": "User", "definitions": { "UserBody": { "type": "object" } } }"#,
        )
        .unwrap();

        let models_dir = tmp.path().join("models");
        std::fs::create_dir_all(&models_dir).unwrap();
        std::fs::write(
            models_dir.join("user.model.json"),
            r#"{ "name": "User", "columns": { "id": { "type": "uuid", "primaryKey": true } } }"#,
        )
        .unwrap();

        let mw_dir = tmp.path().join("middleware");
        std::fs::create_dir_all(&mw_dir).unwrap();
        std::fs::write(
            mw_dir.join("auth.middleware.json"),
            r#"{ "name": "auth", "type": "request" }"#,
        )
        .unwrap();

        let handlers_dir = tmp.path().join("handlers");
        std::fs::create_dir_all(&handlers_dir).unwrap();
        std::fs::write(
            handlers_dir.join("users.handler.json"),
            r#"{
                "name": "getUser",
                "async": true,
                "body": [
                    { "type": "ReturnStatement", "tier": 0, "value": { "type": "Literal", "tier": 0, "value": "ok" } }
                ]
            }"#,
        )
        .unwrap();

        let (project, report) = load_project(tmp.path()).unwrap();
        assert!(report.ok);
        assert_eq!(project.schemas.len(), 1);
        assert_eq!(project.models.len(), 1);
        assert_eq!(project.middleware.len(), 1);
        assert_eq!(project.handlers.len(), 1);
    }
}
