use serde_json::Value;
use tauri::State;

use rash_spec::index::build_index;
use rash_spec::loader;
use rash_spec::types::error::ValidationReport;
use rash_spec::types::handler::HandlerSpec;
use rash_spec::types::middleware::MiddlewareSpec;
use rash_spec::types::model::ModelSpec;
use rash_spec::types::route::RouteSpec;
use rash_spec::types::schema::SchemaSpec;

use crate::error::AppError;
use crate::state::AppState;

// ── Read commands ──

#[tauri::command]
pub fn read_route(file_path: String, state: State<'_, AppState>) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    let full_path = open.root.join(&file_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|_| AppError::FileNotFound(file_path))?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

#[tauri::command]
pub fn read_schema(file_path: String, state: State<'_, AppState>) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    let full_path = open.root.join(&file_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|_| AppError::FileNotFound(file_path))?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

#[tauri::command]
pub fn read_model(file_path: String, state: State<'_, AppState>) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    let full_path = open.root.join(&file_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|_| AppError::FileNotFound(file_path))?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

#[tauri::command]
pub fn read_middleware(file_path: String, state: State<'_, AppState>) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    let full_path = open.root.join(&file_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|_| AppError::FileNotFound(file_path))?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

#[tauri::command]
pub fn read_handler(file_path: String, state: State<'_, AppState>) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    let full_path = open.root.join(&file_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|_| AppError::FileNotFound(file_path))?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

// ── Write commands ──

fn write_spec_file<T: serde::de::DeserializeOwned + serde::Serialize>(
    state: &State<'_, AppState>,
    file_path: &str,
    value: Value,
) -> Result<ValidationReport, AppError> {
    let mut guard = state.project.lock().unwrap();
    let open = guard.as_mut().ok_or(AppError::NoProject)?;

    // Validate by deserializing into the typed struct
    let _typed: T = serde_json::from_value(value.clone())
        .map_err(|e| AppError::InvalidSpec(e.to_string()))?;

    // Write pretty JSON
    let full_path = open.root.join(file_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json_str = serde_json::to_string_pretty(&value)?;
    std::fs::write(&full_path, &json_str)?;

    // Reload project and rebuild index
    let (loaded, load_report) = loader::load_project(&open.root)?;
    let (index, _) = build_index(&loaded);
    open.project = loaded;
    open.index = index;

    // Run validation
    let valid_report = rash_valid::validator::validate(&open.project);
    let mut report = load_report;
    report.merge(valid_report);
    Ok(report)
}

#[tauri::command]
pub fn write_route(
    file_path: String,
    value: Value,
    state: State<'_, AppState>,
) -> Result<ValidationReport, AppError> {
    write_spec_file::<RouteSpec>(&state, &file_path, value)
}

#[tauri::command]
pub fn write_schema(
    file_path: String,
    value: Value,
    state: State<'_, AppState>,
) -> Result<ValidationReport, AppError> {
    write_spec_file::<SchemaSpec>(&state, &file_path, value)
}

#[tauri::command]
pub fn write_model(
    file_path: String,
    value: Value,
    state: State<'_, AppState>,
) -> Result<ValidationReport, AppError> {
    write_spec_file::<ModelSpec>(&state, &file_path, value)
}

#[tauri::command]
pub fn write_middleware(
    file_path: String,
    value: Value,
    state: State<'_, AppState>,
) -> Result<ValidationReport, AppError> {
    write_spec_file::<MiddlewareSpec>(&state, &file_path, value)
}

#[tauri::command]
pub fn write_handler(
    file_path: String,
    value: Value,
    state: State<'_, AppState>,
) -> Result<ValidationReport, AppError> {
    write_spec_file::<HandlerSpec>(&state, &file_path, value)
}

// ── Delete commands ──

fn delete_spec_file(
    state: &State<'_, AppState>,
    file_path: &str,
) -> Result<(), AppError> {
    let mut guard = state.project.lock().unwrap();
    let open = guard.as_mut().ok_or(AppError::NoProject)?;
    let full_path = open.root.join(file_path);

    if !full_path.exists() {
        return Err(AppError::FileNotFound(file_path.to_string()));
    }

    std::fs::remove_file(&full_path)?;

    // Reload project and rebuild index
    let (loaded, _) = loader::load_project(&open.root)?;
    let (index, _) = build_index(&loaded);
    open.project = loaded;
    open.index = index;

    Ok(())
}

#[tauri::command]
pub fn delete_route(file_path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    delete_spec_file(&state, &file_path)
}

#[tauri::command]
pub fn delete_schema(file_path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    delete_spec_file(&state, &file_path)
}

#[tauri::command]
pub fn delete_model(file_path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    delete_spec_file(&state, &file_path)
}

#[tauri::command]
pub fn delete_middleware(file_path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    delete_spec_file(&state, &file_path)
}

#[tauri::command]
pub fn delete_handler(file_path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    delete_spec_file(&state, &file_path)
}
