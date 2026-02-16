use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// Sanitize a name for use as a filename component.
/// Only allows alphanumeric, hyphen, underscore, and dot characters.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

/// Ensure that the resolved path stays within the given root directory.
fn ensure_contained(root: &std::path::Path, child: &std::path::Path) -> Result<PathBuf, AppError> {
    let canonical_root = root.canonicalize().map_err(|e| AppError::IoError(e.to_string()))?;
    let canonical_child = child.canonicalize().map_err(|e| AppError::IoError(e.to_string()))?;
    if !canonical_child.starts_with(&canonical_root) {
        return Err(AppError::IoError(format!(
            "path traversal blocked: {} escapes {}",
            canonical_child.display(),
            canonical_root.display()
        )));
    }
    Ok(canonical_child)
}

/// Write a file and verify it stays within the root directory.
fn write_contained(
    root: &std::path::Path,
    dir: &std::path::Path,
    filename: &str,
    content: &str,
) -> Result<String, AppError> {
    let safe_name = sanitize_filename(filename);
    let path = dir.join(&safe_name);
    std::fs::write(&path, content)?;
    ensure_contained(root, &path)?;
    Ok(safe_name)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub files_created: Vec<String>,
    pub warnings: Vec<String>,
}

/// Export the current project to OpenAPI 3.1 JSON string.
#[tauri::command]
pub fn export_openapi(state: State<'_, AppState>) -> Result<String, AppError> {
    let guard = state.project.lock().map_err(|e| AppError::IoError(e.to_string()))?;
    let open = guard.as_ref().ok_or(AppError::NoProject)?;

    let routes: Vec<_> = open.project.routes.iter().map(|(_, r)| r.clone()).collect();
    let schemas: Vec<_> = open.project.schemas.iter().map(|(_, s)| s.clone()).collect();
    let middleware: Vec<_> = open.project.middleware.iter().map(|(_, m)| m.clone()).collect();

    let doc = rash_openapi::export_openapi(
        &open.project.config,
        &routes,
        &schemas,
        &middleware,
    )
    .map_err(|e| AppError::InvalidSpec(e.to_string()))?;

    serde_json::to_string_pretty(&doc).map_err(|e| AppError::InvalidSpec(e.to_string()))
}

/// Import an OpenAPI 3.1 JSON string into a target project directory.
#[tauri::command]
pub fn import_openapi(
    openapi_json: String,
    target_dir: String,
    _state: State<'_, AppState>,
) -> Result<ImportResult, AppError> {
    let result = rash_openapi::import_openapi(&openapi_json)
        .map_err(|e| AppError::InvalidSpec(e.to_string()))?;

    let target = PathBuf::from(&target_dir);
    if !target.exists() {
        std::fs::create_dir_all(&target)?;
    }

    let mut files_created = Vec::new();

    // Write config
    let safe = write_contained(&target, &target, "rash.config.json", &serde_json::to_string_pretty(&result.config)?)?;
    files_created.push(safe);

    // Write routes
    let routes_dir = target.join("routes");
    if !result.routes.is_empty() {
        std::fs::create_dir_all(&routes_dir)?;
    }
    for route in &result.routes {
        let raw_name = format!("{}.route.json", route.path.trim_start_matches('/').replace('/', "_"));
        let safe = write_contained(&target, &routes_dir, &raw_name, &serde_json::to_string_pretty(route)?)?;
        files_created.push(format!("routes/{safe}"));
    }

    // Write schemas
    let schemas_dir = target.join("schemas");
    if !result.schemas.is_empty() {
        std::fs::create_dir_all(&schemas_dir)?;
    }
    for schema in &result.schemas {
        let raw_name = format!("{}.schema.json", schema.name.to_lowercase());
        let safe = write_contained(&target, &schemas_dir, &raw_name, &serde_json::to_string_pretty(schema)?)?;
        files_created.push(format!("schemas/{safe}"));
    }

    // Write middleware
    let mw_dir = target.join("middleware");
    if !result.middleware.is_empty() {
        std::fs::create_dir_all(&mw_dir)?;
    }
    for mw in &result.middleware {
        let raw_name = format!("{}.middleware.json", mw.name);
        let safe = write_contained(&target, &mw_dir, &raw_name, &serde_json::to_string_pretty(mw)?)?;
        files_created.push(format!("middleware/{safe}"));
    }

    // Write handlers
    let handlers_dir = target.join("handlers");
    if !result.handlers.is_empty() {
        std::fs::create_dir_all(&handlers_dir)?;
    }
    for handler in &result.handlers {
        let raw_name = format!("{}.handler.json", handler.name);
        let safe = write_contained(&target, &handlers_dir, &raw_name, &serde_json::to_string_pretty(handler)?)?;
        files_created.push(format!("handlers/{safe}"));
    }

    Ok(ImportResult {
        files_created,
        warnings: result.warnings,
    })
}

/// Import from existing source code by reverse-parsing.
#[tauri::command]
pub fn import_from_code(
    source_path: String,
    target_dir: String,
    _state: State<'_, AppState>,
) -> Result<ImportResult, AppError> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(AppError::FileNotFound(source_path));
    }

    let source_code = std::fs::read_to_string(&source)?;
    let file_name = source
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();

    let result = rash_openapi::reverse_parse(&source_code, &file_name)
        .map_err(|e| AppError::InvalidSpec(e.to_string()))?;

    let target = PathBuf::from(&target_dir);
    if !target.exists() {
        std::fs::create_dir_all(&target)?;
    }

    let mut files_created = Vec::new();

    // Write routes
    let routes_dir = target.join("routes");
    if !result.routes.is_empty() {
        std::fs::create_dir_all(&routes_dir)?;
    }
    for route in &result.routes {
        let raw_name = format!("{}.route.json", route.path.trim_start_matches('/').replace('/', "_"));
        let safe = write_contained(&target, &routes_dir, &raw_name, &serde_json::to_string_pretty(route)?)?;
        files_created.push(format!("routes/{safe}"));
    }

    // Write schemas
    let schemas_dir = target.join("schemas");
    if !result.schemas.is_empty() {
        std::fs::create_dir_all(&schemas_dir)?;
    }
    for schema in &result.schemas {
        let raw_name = format!("{}.schema.json", schema.name.to_lowercase());
        let safe = write_contained(&target, &schemas_dir, &raw_name, &serde_json::to_string_pretty(schema)?)?;
        files_created.push(format!("schemas/{safe}"));
    }

    // Write middleware
    let mw_dir = target.join("middleware");
    if !result.middleware.is_empty() {
        std::fs::create_dir_all(&mw_dir)?;
    }
    for mw in &result.middleware {
        let raw_name = format!("{}.middleware.json", mw.name);
        let safe = write_contained(&target, &mw_dir, &raw_name, &serde_json::to_string_pretty(mw)?)?;
        files_created.push(format!("middleware/{safe}"));
    }

    // Write handlers
    let handlers_dir = target.join("handlers");
    if !result.handlers.is_empty() {
        std::fs::create_dir_all(&handlers_dir)?;
    }
    for handler in &result.handlers {
        let raw_name = format!("{}.handler.json", handler.name);
        let safe = write_contained(&target, &handlers_dir, &raw_name, &serde_json::to_string_pretty(handler)?)?;
        files_created.push(format!("handlers/{safe}"));
    }

    Ok(ImportResult {
        files_created,
        warnings: result.warnings,
    })
}
