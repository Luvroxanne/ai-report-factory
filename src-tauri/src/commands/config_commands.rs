use tauri::State;

use crate::{
    config::app_config::{self, AppConfig},
    services::optional::tts::{self, VoiceOption},
    utils::{errors::err_to_string, paths::normalize_output_dir},
    AppState,
};

#[tauri::command]
pub fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    state.config.lock().map(|cfg| cfg.clone()).map_err(|_| "配置锁已损坏".to_string())
}

#[tauri::command]
pub fn save_app_config(state: State<'_, AppState>, mut config: AppConfig) -> Result<AppConfig, String> {
    if config.output_dir.trim().is_empty() {
        config.output_dir = state.paths.outputs_dir.to_string_lossy().to_string();
    }
    let output_dir = normalize_output_dir(&config.output_dir, &state.paths.outputs_dir);
    std::fs::create_dir_all(output_dir).map_err(err_to_string)?;
    app_config::save(&state.paths, &config).map_err(err_to_string)?;
    *state.config.lock().map_err(|_| "配置锁已损坏".to_string())? = config.clone();
    Ok(config)
}

#[tauri::command]
pub fn reset_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let mut config = AppConfig::default();
    config.output_dir = state.paths.outputs_dir.to_string_lossy().to_string();
    app_config::save(&state.paths, &config).map_err(err_to_string)?;
    *state.config.lock().map_err(|_| "配置锁已损坏".to_string())? = config.clone();
    Ok(config)
}

#[tauri::command]
pub fn list_tts_voices() -> Vec<VoiceOption> {
    tts::voice_options()
}
