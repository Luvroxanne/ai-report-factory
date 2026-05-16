use serde_json::json;

use crate::{config::app_config::AppConfig, db::models::ReportPlan, services::ai::{parse_plan_or_fallback, prompt}, utils::errors::{AppError, AppResult}};

pub fn generate_plan(input: &str, title: &str, style: &str, config: &AppConfig) -> AppResult<ReportPlan> {
    if config.api_key.trim().is_empty() {
        return Err(AppError::Message("OpenAI 兼容接口缺少 API Key".into()));
    }
    let url = format!("{}/chat/completions", config.api_base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_seconds))
        .build()?;
    let resp: serde_json::Value = client
        .post(url)
        .bearer_auth(&config.api_key)
        .json(&json!({
            "model": config.model_name,
            "messages": [
                {"role": "system", "content": "你是资深商业分析师、PPT导演、Word解说稿作者和视频分镜导演组成的报告生成Agent。只输出可解析JSON。"},
                {"role": "user", "content": prompt(input, title, style)}
            ],
            "temperature": 0.4
        }))
        .send()?
        .error_for_status()?
        .json()?;
    let content = resp["choices"][0]["message"]["content"].as_str().unwrap_or_default();
    Ok(parse_plan_or_fallback(content, input, title, style, "OpenAI compatible provider"))
}
