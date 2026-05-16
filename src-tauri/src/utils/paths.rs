use std::path::PathBuf;

use crate::utils::errors::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub app_dir: PathBuf,
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub outputs_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn init() -> AppResult<Self> {
        let app_dir = runtime_base_dir()?;
        let config_path = app_dir.join("config.json");
        let db_path = app_dir.join("storage").join("ai_report_factory.sqlite3");
        let outputs_dir = app_dir.join("outputs");
        let logs_dir = app_dir.join("logs");

        std::fs::create_dir_all(&outputs_dir)?;
        std::fs::create_dir_all(&logs_dir)?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(Self {
            app_dir,
            config_path,
            db_path,
            outputs_dir,
            logs_dir,
        })
    }
}

fn runtime_base_dir() -> AppResult<PathBuf> {
    if let Ok(value) = std::env::var("AI_REPORT_FACTORY_HOME") {
        return Ok(PathBuf::from(value));
    }
    dirs::data_local_dir()
        .map(|dir| dir.join("AI Report Factory"))
        .ok_or_else(|| AppError::Message("无法定位用户可写数据目录".to_string()))
}

pub fn normalize_output_dir(value: &str, fallback: &PathBuf) -> PathBuf {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return fallback.clone();
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path
    } else {
        fallback.join(path)
    }
}
