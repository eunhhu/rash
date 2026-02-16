use serde::Deserialize;
use serde_json::Value;
use tauri::State;

use rash_codegen::CodeGenerator;
use rash_ir::convert::convert_project;
use rash_spec::types::common::{Framework, Language};
use rash_spec::types::error::ValidationReport;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn validate_project(state: State<'_, AppState>) -> Result<ValidationReport, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    let report = rash_valid::validator::validate(&open.project);
    Ok(report)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCodeArgs {
    pub language: Language,
    pub framework: Framework,
}

#[tauri::command]
pub fn preview_code(
    args: PreviewCodeArgs,
    state: State<'_, AppState>,
) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;

    let ir = convert_project(&open.project)
        .map_err(|e| AppError::CodegenError(e.to_string()))?;

    let generator = CodeGenerator::new(args.language, args.framework)?;
    let generated = generator.generate(&ir)?;

    let files: serde_json::Map<String, Value> = generated
        .files()
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    Ok(Value::Object(files))
}

#[tauri::command]
pub fn generate_project(
    output_dir: String,
    language: Language,
    framework: Framework,
    state: State<'_, AppState>,
) -> Result<Value, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;

    let ir = convert_project(&open.project)
        .map_err(|e| AppError::CodegenError(e.to_string()))?;

    let generator = CodeGenerator::new(language, framework)?;
    let generated = generator.generate(&ir)?;

    let out_path = std::path::PathBuf::from(&output_dir);
    generated
        .write_to_disk(&out_path)
        .map_err(|e| AppError::IoError(e.to_string()))?;

    Ok(serde_json::json!({
        "outputDir": output_dir,
        "fileCount": generated.file_count(),
    }))
}
