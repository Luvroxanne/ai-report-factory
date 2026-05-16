use std::{fs, path::{Path, PathBuf}};

use chrono::{DateTime, Local};
use serde::Serialize;
use walkdir::WalkDir;

use crate::utils::errors::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct LocalFileItem {
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub size: u64,
    pub created_at: String,
    pub previewable: bool,
}

pub fn scan_outputs(output_dir: &Path) -> AppResult<Vec<LocalFileItem>> {
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }
    let mut items = Vec::new();
    for entry in WalkDir::new(output_dir).max_depth(4).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let meta = entry.metadata()?;
        items.push(LocalFileItem {
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            file_type: file_type(path),
            size: meta.len(),
            created_at: meta
                .created()
                .ok()
                .map(|t| DateTime::<Local>::from(t).to_rfc3339())
                .unwrap_or_default(),
            previewable: is_previewable(path),
        });
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

pub fn read_preview(path: &Path) -> AppResult<String> {
    if !is_previewable(path) {
        return Err(AppError::Message("该文件类型不支持内置预览，请使用打开文件".into()));
    }
    let meta = fs::metadata(path)?;
    if meta.len() > 1024 * 1024 {
        return Err(AppError::Message("文件超过 1MB，已阻止预览以避免卡顿".into()));
    }
    Ok(fs::read_to_string(path)?)
}

pub fn file_type(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_ascii_lowercase()
}

pub fn is_previewable(path: &Path) -> bool {
    matches!(file_type(path).as_str(), "txt" | "md" | "json" | "log" | "srt" | "vtt")
}

pub fn ensure_inside_known_roots(path: &Path, roots: &[PathBuf]) -> AppResult<()> {
    let canonical = fs::canonicalize(path)?;
    for root in roots {
        if let Ok(root) = fs::canonicalize(root) {
            if canonical.starts_with(root) {
                return Ok(());
            }
        }
    }
    Err(AppError::Message("拒绝访问非应用输出/日志目录中的文件".into()))
}
