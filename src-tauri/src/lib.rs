mod commands;
mod error;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::project::create_project,
            commands::project::open_project,
            commands::project::close_project,
            commands::project::get_project_tree,
            commands::spec::read_route,
            commands::spec::read_schema,
            commands::spec::read_model,
            commands::spec::read_middleware,
            commands::spec::read_handler,
            commands::spec::write_route,
            commands::spec::write_schema,
            commands::spec::write_model,
            commands::spec::write_middleware,
            commands::spec::write_handler,
            commands::spec::delete_route,
            commands::spec::delete_schema,
            commands::spec::delete_model,
            commands::spec::delete_middleware,
            commands::spec::delete_handler,
            commands::spec::move_route,
            commands::spec::move_schema,
            commands::spec::move_model,
            commands::spec::move_middleware,
            commands::spec::move_handler,
            commands::codegen::validate_project,
            commands::codegen::preview_code,
            commands::codegen::generate_project,
            commands::runtime::detect_runtimes,
            commands::runtime::run_preflight,
            commands::runtime::start_server,
            commands::runtime::stop_server,
            commands::runtime::restart_server,
            commands::runtime::get_server_status,
            commands::runtime::apply_hmu,
            commands::openapi::export_openapi,
            commands::openapi::import_openapi,
            commands::openapi::import_from_code,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
