use serde::Serialize;
use tauri::State;

use crate::{services::optional::{tts, video}, utils::paths::normalize_output_dir, AppState};

#[derive(Debug, Serialize)]
pub struct SystemStatusItem {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[tauri::command]
pub async fn get_system_status(state: State<'_, AppState>) -> Result<Vec<SystemStatusItem>, String> {
    let config = state.config.lock().map_err(|_| "配置锁已损坏".to_string())?.clone();
    let output_dir = normalize_output_dir(&config.output_dir, &state.paths.outputs_dir);
    let db_ok = state.paths.db_path.exists();
    let output_ok = std::fs::create_dir_all(&output_dir).is_ok();
    let ffmpeg_found = video::ffmpeg_exists(&config);
    let (tts_name, tts_ok, tts_detail) = tts::status();
    let (ff_name, ff_ok, ff_detail) = video::status(config.enable_ffmpeg, ffmpeg_found);
    Ok(vec![
        SystemStatusItem { name: "Rust 内置后端".into(), ok: true, detail: "Tauri command 已加载，无 Python sidecar".into() },
        SystemStatusItem { name: "SQLite 数据库".into(), ok: db_ok, detail: state.paths.db_path.to_string_lossy().to_string() },
        SystemStatusItem { name: "输出目录权限".into(), ok: output_ok, detail: output_dir.to_string_lossy().to_string() },
        SystemStatusItem { name: "配置文件".into(), ok: state.paths.config_path.exists(), detail: state.paths.config_path.to_string_lossy().to_string() },
        SystemStatusItem { name: tts_name.into(), ok: tts_ok, detail: tts_detail.into() },
        SystemStatusItem { name: ff_name.into(), ok: ff_ok, detail: ff_detail },
    ])
}
