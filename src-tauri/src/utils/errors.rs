use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("文件系统错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("配置序列化错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("数据库错误：{0}")]
    Db(#[from] rusqlite::Error),
    #[error("网络请求错误：{0}")]
    Http(#[from] reqwest::Error),
    #[error("压缩包生成错误：{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("目录扫描错误：{0}")]
    Walkdir(#[from] walkdir::Error),
    #[error("{0}")]
    Message(String),
}

pub type AppResult<T> = Result<T, AppError>;

pub fn err_to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
