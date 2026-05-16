use serde_json::json;

use crate::{config::app_config::AppConfig, db::models::ReportPlan, services::ai::{parse_plan_or_fallback, prompt}, utils::errors::AppResult};

pub fn generate_plan(input: &str, title: &str, style: &str, config: &AppConfig) -> AppResult<ReportPlan> {
    let url = format!("{}/api/generate", config.ollama_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_seconds))
        .build()?;
    let resp: serde_json::Value = client
        .post(url)
        .json(&json!({
            "model": config.model_name,
            "prompt": prompt(input, title, style),
            "stream": false
        }))
        .send()?
        .error_for_status()?
        .json()?;
    let content = resp["response"].as_str().unwrap_or_default();
    Ok(parse_plan_or_fallback(content, input, title, style, "Ollama provider"))
}
