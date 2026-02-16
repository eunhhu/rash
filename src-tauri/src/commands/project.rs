use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use rash_spec::index::build_index;
use rash_spec::loader;

use crate::error::AppError;
use crate::state::{AppState, OpenProject};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTree {
    pub name: String,
    pub path: String,
    pub config: serde_json::Value,
    pub nodes: Vec<TreeNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub path: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectArgs {
    pub name: String,
    pub path: String,
    pub language: String,
    pub framework: String,
    pub runtime: String,
}

#[tauri::command]
pub fn create_project(
    args: CreateProjectArgs,
    state: State<'_, AppState>,
) -> Result<ProjectTree, AppError> {
    let project_dir = PathBuf::from(&args.path).join(&args.name);
    std::fs::create_dir_all(&project_dir)?;

    // Create subdirectories
    for dir in &["routes", "schemas", "models", "middleware", "handlers"] {
        std::fs::create_dir_all(project_dir.join(dir))?;
    }

    // Write config
    let config = serde_json::json!({
        "version": "1.0.0",
        "name": args.name,
        "target": {
            "language": args.language,
            "framework": args.framework,
            "runtime": args.runtime
        },
        "server": {
            "port": 3000,
            "host": "0.0.0.0"
        }
    });

    let config_str = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::InvalidSpec(e.to_string()))?;
    std::fs::write(project_dir.join("rash.config.json"), &config_str)?;

    // Open the newly created project
    open_project(project_dir.to_string_lossy().into_owned(), state)
}

#[tauri::command]
pub fn open_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<ProjectTree, AppError> {
    let project_dir = PathBuf::from(&path);
    let (loaded, _report) = loader::load_project(&project_dir)?;
    let (index, _index_errors) = build_index(&loaded);

    let tree = build_project_tree(&loaded, &project_dir);

    let mut guard = state.project.lock().unwrap();
    *guard = Some(OpenProject {
        root: project_dir,
        project: loaded,
        index,
    });

    Ok(tree)
}

#[tauri::command]
pub fn close_project(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut guard = state.project.lock().unwrap();
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn get_project_tree(state: State<'_, AppState>) -> Result<ProjectTree, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    Ok(build_project_tree(&open.project, &open.root))
}

fn build_project_tree(project: &loader::LoadedProject, root: &Path) -> ProjectTree {
    let config_value = serde_json::to_value(&project.config).unwrap_or_default();

    let mut route_children: Vec<TreeNode> = project
        .routes
        .iter()
        .map(|(file, route)| TreeNode {
            id: format!("route:{}", route.path),
            label: route.path.clone(),
            kind: "route".to_string(),
            path: Some(file.clone()),
            children: vec![],
        })
        .collect();
    route_children.sort_by(|a, b| a.label.cmp(&b.label));

    let mut schema_children: Vec<TreeNode> = project
        .schemas
        .iter()
        .map(|(file, schema)| TreeNode {
            id: format!("schema:{}", schema.name),
            label: schema.name.clone(),
            kind: "schema".to_string(),
            path: Some(file.clone()),
            children: vec![],
        })
        .collect();
    schema_children.sort_by(|a, b| a.label.cmp(&b.label));

    let mut model_children: Vec<TreeNode> = project
        .models
        .iter()
        .map(|(file, model)| TreeNode {
            id: format!("model:{}", model.name),
            label: model.name.clone(),
            kind: "model".to_string(),
            path: Some(file.clone()),
            children: vec![],
        })
        .collect();
    model_children.sort_by(|a, b| a.label.cmp(&b.label));

    let mut mw_children: Vec<TreeNode> = project
        .middleware
        .iter()
        .map(|(file, mw)| TreeNode {
            id: format!("middleware:{}", mw.name),
            label: mw.name.clone(),
            kind: "middleware".to_string(),
            path: Some(file.clone()),
            children: vec![],
        })
        .collect();
    mw_children.sort_by(|a, b| a.label.cmp(&b.label));

    let mut handler_children: Vec<TreeNode> = project
        .handlers
        .iter()
        .map(|(file, handler)| TreeNode {
            id: format!("handler:{}", handler.name),
            label: handler.name.clone(),
            kind: "handler".to_string(),
            path: Some(file.clone()),
            children: vec![],
        })
        .collect();
    handler_children.sort_by(|a, b| a.label.cmp(&b.label));

    let nodes = vec![
        TreeNode {
            id: "section:routes".to_string(),
            label: "Routes".to_string(),
            kind: "section".to_string(),
            path: None,
            children: route_children,
        },
        TreeNode {
            id: "section:schemas".to_string(),
            label: "Schemas".to_string(),
            kind: "section".to_string(),
            path: None,
            children: schema_children,
        },
        TreeNode {
            id: "section:models".to_string(),
            label: "Models".to_string(),
            kind: "section".to_string(),
            path: None,
            children: model_children,
        },
        TreeNode {
            id: "section:middleware".to_string(),
            label: "Middleware".to_string(),
            kind: "section".to_string(),
            path: None,
            children: mw_children,
        },
        TreeNode {
            id: "section:handlers".to_string(),
            label: "Handlers".to_string(),
            kind: "section".to_string(),
            path: None,
            children: handler_children,
        },
    ];

    ProjectTree {
        name: project.config.name.clone(),
        path: root.to_string_lossy().into_owned(),
        config: config_value,
        nodes,
    }
}
