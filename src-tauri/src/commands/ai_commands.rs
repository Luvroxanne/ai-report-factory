use serde::Serialize;
use tauri::State;

use crate::{config::app_config::{AiProvider, AppConfig}, AppState};

#[derive(Debug, Serialize)]
pub struct ProviderTestResult {
    pub ok: bool,
    pub provider: String,
    pub message: String,
}

#[tauri::command]
pub async fn test_ai_connection(state: State<'_, AppState>, config: Option<AppConfig>) -> Result<ProviderTestResult, String> {
    let config = match config {
        Some(config) => config,
        None => state.config.lock().map_err(|_| "配置锁已损坏".to_string())?.clone(),
    };
    let provider = format!("{:?}", config.ai_provider);
    if matches!(config.ai_provider, AiProvider::Local) {
        return Ok(ProviderTestResult { ok: true, provider, message: "本地规则兜底可用，无需网络".into() });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_seconds.min(30)))
        .build()
        .map_err(|err| err.to_string())?;

    let result = match config.ai_provider {
        AiProvider::OpenAiCompatible => {
            if config.api_key.trim().is_empty() {
                Err("缺少 API Key".to_string())
            } else {
                client
                    .get(format!("{}/models", config.api_base_url.trim_end_matches('/')))
                    .bearer_auth(&config.api_key)
                    .send()
                    .await
                    .map(|r| format!("HTTP {}", r.status()))
                    .map_err(|err| err.to_string())
            }
        }
        AiProvider::Gemini => {
            if config.api_key.trim().is_empty() {
                Err("缺少 Gemini API Key".to_string())
            } else {
                client
                    .get(format!("{}/v1beta/models?key={}", config.api_base_url.trim_end_matches('/'), config.api_key))
                    .send()
                    .await
                    .map(|r| format!("HTTP {}", r.status()))
                    .map_err(|err| err.to_string())
            }
        }
        AiProvider::Ollama => client
            .get(format!("{}/api/tags", config.ollama_url.trim_end_matches('/')))
            .send()
            .await
            .map(|r| format!("HTTP {}", r.status()))
            .map_err(|err| err.to_string()),
        AiProvider::Local => unreachable!(),
    };

    Ok(match result {
        Ok(message) => ProviderTestResult { ok: true, provider, message },
        Err(message) => ProviderTestResult { ok: false, provider, message },
    })
}
