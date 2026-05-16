use serde::{Deserialize, Serialize};

use crate::{utils::errors::AppResult, utils::paths::AppPaths};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    OpenAiCompatible,
    Gemini,
    Ollama,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsProvider {
    None,
    WindowsSapi,
    OpenAiCompatible,
    FishSpeech,
}

impl Default for TtsProvider {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub ai_provider: AiProvider,
    pub api_base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub ollama_url: String,
    pub output_dir: String,
    pub enable_tts: bool,
    pub enable_video: bool,
    pub enable_local_fallback: bool,
    pub enable_ffmpeg: bool,
    pub ffmpeg_path: String,
    pub request_timeout_seconds: u64,
    pub tts_provider: TtsProvider,
    pub tts_voice: String,
    pub tts_model: String,
    pub tts_base_url: String,
    pub tts_api_key: String,
    pub video_width: u32,
    pub video_height: u32,
    pub video_fps: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai_provider: AiProvider::Local,
            api_base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model_name: "gpt-4o-mini".to_string(),
            ollama_url: "http://127.0.0.1:11434".to_string(),
            output_dir: String::new(),
            enable_tts: true,
            enable_video: true,
            enable_local_fallback: true,
            enable_ffmpeg: true,
            ffmpeg_path: String::new(),
            request_timeout_seconds: 60,
            tts_provider: TtsProvider::WindowsSapi,
            tts_voice: "windows_default".to_string(),
            tts_model: "tts-1".to_string(),
            tts_base_url: "http://127.0.0.1:8080/v1".to_string(),
            tts_api_key: String::new(),
            video_width: 1920,
            video_height: 1080,
            video_fps: 24,
        }
    }
}

pub fn load_or_default(paths: &AppPaths) -> AppResult<AppConfig> {
    if !paths.config_path.exists() {
        let mut config = AppConfig::default();
        config.output_dir = paths.outputs_dir.to_string_lossy().to_string();
        save(paths, &config)?;
        return Ok(config);
    }

    let text = std::fs::read_to_string(&paths.config_path).unwrap_or_default();
    match serde_json::from_str::<AppConfig>(&text) {
        Ok(mut config) => {
            if config.output_dir.trim().is_empty() {
                config.output_dir = paths.outputs_dir.to_string_lossy().to_string();
            }
            Ok(config)
        }
        Err(_) => {
            let broken = paths.config_path.with_extension("broken.json");
            let _ = std::fs::rename(&paths.config_path, broken);
            let mut config = AppConfig::default();
            config.output_dir = paths.outputs_dir.to_string_lossy().to_string();
            save(paths, &config)?;
            Ok(config)
        }
    }
}

pub fn save(paths: &AppPaths, config: &AppConfig) -> AppResult<()> {
    if let Some(parent) = paths.config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&paths.config_path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}
