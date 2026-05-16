use serde_json::json;

use crate::{config::app_config::AppConfig, db::models::ReportPlan, services::ai::{parse_plan_or_fallback, prompt}, utils::errors::{AppError, AppResult}};

pub fn generate_plan(input: &str, title: &str, style: &str, config: &AppConfig) -> AppResult<ReportPlan> {
    if config.api_key.trim().is_empty() {
        return Err(AppError::Message("Gemini 缺少 API Key".into()));
    }
    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        config.api_base_url.trim_end_matches('/'),
        config.model_name,
        config.api_key
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_seconds))
        .build()?;
    let resp: serde_json::Value = client
        .post(url)
        .json(&json!({"contents": [{"parts": [{"text": prompt(input, title, style)}]}]}))
        .send()?
        .error_for_status()?
        .json()?;
    let content = resp["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or_default();
    Ok(parse_plan_or_fallback(content, input, title, style, "Gemini provider"))
}
