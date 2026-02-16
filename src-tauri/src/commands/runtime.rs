use std::collections::HashMap;

use tauri::{AppHandle, Emitter, State};

use rash_runtime::preflight::PreflightReport;
use rash_runtime::preflight_checker::PreflightChecker;
use rash_runtime::process_manager::{ProcessManager, ServerConfig, ServerStatus};
use rash_runtime::runtime_detect::{DetectedRuntime, RuntimeDetector};

use crate::error::AppError;
use crate::state::{AppState, RuntimeState};

#[tauri::command]
pub async fn detect_runtimes() -> Result<Vec<DetectedRuntime>, AppError> {
    Ok(RuntimeDetector::detect_installed())
}

#[tauri::command]
pub async fn run_preflight(state: State<'_, AppState>) -> Result<PreflightReport, AppError> {
    let guard = state.project.lock().unwrap();
    let open = guard.as_ref().ok_or(AppError::NoProject)?;
    Ok(PreflightChecker::run(&open.project.config, &open.root))
}

#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u16, AppError> {
    // 1. Extract project config while holding the sync lock briefly
    let server_config = {
        let guard = state.project.lock().unwrap();
        let open = guard.as_ref().ok_or(AppError::NoProject)?;
        let config = &open.project.config;

        ServerConfig {
            language: config.target.language,
            framework: config.target.framework,
            runtime: config.target.runtime,
            port: config.server.port,
            host: config.server.host.clone(),
            output_dir: open.root.join(
                config
                    .codegen
                    .as_ref()
                    .map(|c| c.out_dir.as_str())
                    .unwrap_or("./dist"),
            ),
            env_vars: HashMap::new(),
        }
    };

    // 2. Stop existing process if any
    {
        let mut rt_guard = state.runtime.lock().await;
        if let Some(ref mut rt_state) = *rt_guard {
            let _ = rt_state.process_manager.stop().await;
        }
        *rt_guard = None;
    }

    // 3. Create new ProcessManager and start
    let (mut pm, log_rx, status_rx) = ProcessManager::new();

    let port = pm
        .start(&server_config)
        .await
        .map_err(|e| AppError::RuntimeError(e.to_string()))?;

    // 4. Store RuntimeState
    {
        let mut rt_guard = state.runtime.lock().await;
        *rt_guard = Some(RuntimeState {
            process_manager: pm,
            status_rx,
        });
    }

    // 5. Spawn log forwarding task — owns log_rx, no reference to state
    let app_clone = app.clone();
    tokio::spawn(async move {
        let mut log_rx = log_rx;
        while let Some(log) = log_rx.recv().await {
            let _ = app_clone.emit("server:log", &log);
        }
    });

    Ok(port)
}

#[tauri::command]
pub async fn stop_server(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut rt_guard = state.runtime.lock().await;
    let rt_state = rt_guard
        .as_mut()
        .ok_or(AppError::RuntimeError("no server is running".into()))?;

    rt_state
        .process_manager
        .stop()
        .await
        .map_err(|e| AppError::RuntimeError(e.to_string()))?;

    *rt_guard = None;
    Ok(())
}

#[tauri::command]
pub async fn restart_server(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u16, AppError> {
    // Stop first (ignore error if not running)
    {
        let mut rt_guard = state.runtime.lock().await;
        if let Some(ref mut rt_state) = *rt_guard {
            let _ = rt_state.process_manager.stop().await;
        }
        *rt_guard = None;
    }

    // Start again
    start_server(app, state).await
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> Result<ServerStatus, AppError> {
    let rt_guard = state.runtime.lock().await;
    match rt_guard.as_ref() {
        Some(rt_state) => Ok(rt_state.process_manager.status()),
        None => Ok(ServerStatus::Stopped),
    }
}
