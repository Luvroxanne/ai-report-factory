use std::path::{Path, PathBuf};

use tauri::State;

use crate::{services::files::file_indexer::{ensure_inside_known_roots, read_preview, scan_outputs, LocalFileItem}, utils::{errors::err_to_string, paths::normalize_output_dir}, AppState};

#[tauri::command]
pub fn scan_output_files(state: State<'_, AppState>) -> Result<Vec<LocalFileItem>, String> {
    let config = state.config.lock().map_err(|_| "配置锁已损坏".to_string())?.clone();
    let output_dir = normalize_output_dir(&config.output_dir, &state.paths.outputs_dir);
    scan_outputs(&output_dir).map_err(err_to_string)
}

#[tauri::command]
pub fn preview_file(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let path = PathBuf::from(path);
    let config = state.config.lock().map_err(|_| "配置锁已损坏".to_string())?.clone();
    let output_dir = normalize_output_dir(&config.output_dir, &state.paths.outputs_dir);
    ensure_inside_known_roots(&path, &[output_dir, state.paths.logs_dir.clone()]).map_err(err_to_string)?;
    read_preview(&path).map_err(err_to_string)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    open::that(Path::new(&path)).map_err(err_to_string)?;
    Ok(())
}

#[tauri::command]
pub fn open_in_folder(path: String) -> Result<(), String> {
    let target = PathBuf::from(path);
    let folder = if target.is_dir() { target } else { target.parent().unwrap_or(Path::new(".")).to_path_buf() };
    open::that(folder).map_err(err_to_string)?;
    Ok(())
}
