pub mod commands;
pub mod config;
pub mod db;
pub mod services;
pub mod utils;

use std::sync::Mutex;

use config::app_config::{load_or_default, AppConfig};
use db::migrations::open_and_migrate;
use rusqlite::Connection;
use serde::Serialize;
use tauri::Manager;
use utils::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub db: Mutex<Connection>,
    pub config: Mutex<AppConfig>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: T,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(false)
        .try_init()
        .ok();

    tauri::Builder::default()
        .setup(|app| {
            let paths = AppPaths::init().map_err(|err| err.to_string())?;
            let config = load_or_default(&paths).map_err(|err| err.to_string())?;
            let db = open_and_migrate(&paths.db_path).map_err(|err| err.to_string())?;
            app.manage(AppState {
                paths,
                db: Mutex::new(db),
                config: Mutex::new(config),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config_commands::get_app_config,
            commands::config_commands::save_app_config,
            commands::config_commands::reset_app_config,
            commands::config_commands::list_tts_voices,
            commands::ai_commands::test_ai_connection,
            commands::task_commands::create_task,
            commands::task_commands::list_tasks,
            commands::task_commands::get_task,
            commands::task_commands::delete_task,
            commands::task_commands::rerun_task,
            commands::file_commands::scan_output_files,
            commands::file_commands::preview_file,
            commands::file_commands::open_path,
            commands::file_commands::open_in_folder,
            commands::system_commands::get_system_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
