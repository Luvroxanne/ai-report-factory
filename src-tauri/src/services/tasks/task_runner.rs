use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use rusqlite::{params, Connection};

use crate::{
    config::app_config::{AppConfig, TtsProvider},
    db::{
        migrations::migrate,
        models::{now_string, ReportPlan},
    },
    services::{
        ai,
        docx::docx_builder,
        optional::{tts, video},
        ppt::pptx_builder,
    },
    utils::{errors::AppResult, paths::normalize_output_dir},
};

use super::task_status::{FAILED, RUNNING, SUCCESS};

pub fn run_task(db_path: PathBuf, app_outputs_dir: PathBuf, config: AppConfig, task_id: String, style: String, outputs: Vec<String>) {
    if let Err(err) = run_task_inner(db_path.clone(), app_outputs_dir, config, task_id.clone(), style, outputs) {
        if let Ok(conn) = Connection::open(db_path) {
            let _ = update_failure(&conn, &task_id, &err.to_string());
        }
    }
}

fn run_task_inner(db_path: PathBuf, app_outputs_dir: PathBuf, config: AppConfig, task_id: String, style: String, outputs: Vec<String>) -> AppResult<()> {
    let conn = Connection::open(db_path)?;
    migrate(&conn)?;
    let (title, input_text): (String, String) = conn.query_row(
        "SELECT title, COALESCE(input_text,'') FROM tasks WHERE id=?1",
        params![task_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let output_root = normalize_output_dir(&config.output_dir, &app_outputs_dir);
    fs::create_dir_all(&output_root)?;
    let task_dir = output_root.join(&task_id);
    fs::create_dir_all(&task_dir)?;
    let log_path = task_dir.join("generation.log");
    update_step(&conn, &task_id, RUNNING, 5, "准备本地输出目录", Some(&task_dir), Some(&log_path))?;
    append_log(&log_path, "任务启动：Rust 内置后端，无 Python sidecar")?;

    update_step(&conn, &task_id, RUNNING, 18, "生成报告结构", Some(&task_dir), Some(&log_path))?;
    let plan = ai::build_report_plan(&input_text, &title, &style, &config)?;
    let json_path = task_dir.join("storyboard.json");
    fs::write(&json_path, serde_json::to_string_pretty(&plan)?)?;
    append_log(&log_path, &format!("报告结构完成：{} 页，{}", plan.slides.len(), plan.generation_note))?;

    let mut pptx_path = None;
    let mut docx_path = None;
    let mut script_path = None;
    let mut subtitle_path = None;
    let mut audio_path = None;
    let mut video_path = None;

    if wants(&outputs, "pptx") {
        update_step(&conn, &task_id, RUNNING, 40, "生成本地 PPTX", Some(&task_dir), Some(&log_path))?;
        let path = task_dir.join("report.pptx");
        pptx_builder::build_pptx(&plan, &path)?;
        pptx_path = Some(path);
    }

    if wants(&outputs, "docx") {
        update_step(&conn, &task_id, RUNNING, 60, "生成 Word 解说稿", Some(&task_dir), Some(&log_path))?;
        let path = task_dir.join("speaker_script.docx");
        docx_builder::build_docx(&plan, &path)?;
        docx_path = Some(path);
    }

    if wants(&outputs, "script") || wants(&outputs, "txt") || wants(&outputs, "md") {
        update_step(&conn, &task_id, RUNNING, 72, "生成 TXT/MD 讲稿", Some(&task_dir), Some(&log_path))?;
        let md_path = task_dir.join("speaker_script.md");
        let txt_path = task_dir.join("speaker_script.txt");
        fs::write(&md_path, docx_builder::build_markdown(&plan))?;
        fs::write(&txt_path, docx_builder::build_txt(&plan))?;
        script_path = Some(md_path);
    }

    if wants(&outputs, "subtitle") || wants(&outputs, "video") {
        update_step(&conn, &task_id, RUNNING, 82, "生成字幕 SRT", Some(&task_dir), Some(&log_path))?;
        let path = task_dir.join("subtitle.srt");
        fs::write(&path, build_srt(&plan))?;
        subtitle_path = Some(path);
    }

    if wants(&outputs, "audio") || wants(&outputs, "video") {
        update_step(&conn, &task_id, RUNNING, 86, "生成语音旁白", Some(&task_dir), Some(&log_path))?;
        let narration_config = narration_config_for_outputs(&config, wants(&outputs, "video"));
        match tts::generate_narration(&plan, &task_dir, &narration_config) {
            Ok(Some(path)) => {
                append_log(&log_path, &format!("语音旁白生成完成：{}", path.display()))?;
                audio_path = Some(path);
            }
            Ok(None) => {
                if wants(&outputs, "video") {
                    let fallback = windows_sapi_fallback_config(&config);
                    match tts::generate_narration(&plan, &task_dir, &fallback) {
                        Ok(Some(path)) => {
                            append_log(&log_path, &format!("已自动使用 Windows SAPI 生成视频旁白：{}", path.display()))?;
                            audio_path = Some(path);
                        }
                        Ok(None) => append_log(&log_path, "未生成旁白，视频将使用静音音轨")?,
                        Err(err) => append_log(&log_path, &format!("Windows SAPI 旁白生成失败，视频将使用静音音轨：{err}"))?,
                    }
                } else {
                    append_log(&log_path, "TTS 未启用或未选择语音提供方，跳过语音旁白")?;
                }
            }
            Err(err) => {
                if wants(&outputs, "video") {
                    append_log(&log_path, &format!("当前 TTS 生成失败，尝试自动回退 Windows SAPI：{err}"))?;
                    let fallback = windows_sapi_fallback_config(&config);
                    match tts::generate_narration(&plan, &task_dir, &fallback) {
                        Ok(Some(path)) => {
                            append_log(&log_path, &format!("Windows SAPI 回退旁白生成完成：{}", path.display()))?;
                            audio_path = Some(path);
                        }
                        Ok(None) => append_log(&log_path, "Windows SAPI 未生成旁白，视频将使用静音音轨")?,
                        Err(fallback_err) => append_log(&log_path, &format!("Windows SAPI 回退失败，视频将使用静音音轨：{fallback_err}"))?,
                    }
                } else {
                    append_log(&log_path, &format!("TTS 生成失败，主流程继续：{err}"))?;
                }
            }
        }
    }

    if wants(&outputs, "video") {
        update_step(&conn, &task_id, RUNNING, 90, "生成 MP4 视频", Some(&task_dir), Some(&log_path))?;
        if audio_path.is_none() {
            return Err(crate::utils::errors::AppError::Message(
                "已勾选视频，但旁白音频生成失败；为避免输出无声视频，本次任务已停止。请检查配置中心的 TTS 音色，或使用 Windows 默认音色后重试。".into(),
            ));
        }
        if config.enable_video && config.enable_ffmpeg {
            let subtitle_ref = subtitle_path.as_ref().unwrap_or(&json_path);
            match video::generate_video(&plan, &task_dir, subtitle_ref, audio_path.as_deref(), &config) {
                Ok(path) => {
                    append_log(&log_path, &format!("MP4 视频生成完成：{}", path.display()))?;
                    video_path = Some(path);
                }
                Err(err) => {
                    append_log(&log_path, &format!("MP4 视频生成失败：{err}"))?;
                    return Err(err);
                }
            }
        } else {
            let reason = if !config.enable_video {
                "配置中心未开启“可选视频”"
            } else {
                "配置中心未开启“ffmpeg”"
            };
            return Err(crate::utils::errors::AppError::Message(format!(
                "已勾选视频，但{reason}，未生成 MP4；请保存配置后重试"
            )));
        }
    }

    update_step(&conn, &task_id, RUNNING, 94, "写入任务产物索引", Some(&task_dir), Some(&log_path))?;
    let metadata_path = task_dir.join("metadata.json");
    fs::write(&metadata_path, serde_json::to_string_pretty(&serde_json::json!({
        "task_id": task_id,
        "title": plan.title,
        "artifacts": {
            "pptx": pptx_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "docx": docx_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "script": script_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "audio": audio_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "video": video_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "subtitle": subtitle_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "storyboard": json_path.to_string_lossy().to_string()
        }
    }))?)?;

    conn.execute(
        r#"
        UPDATE tasks SET status=?2, progress=100, current_step='生成完成', output_dir=?3,
        pptx_path=?4, docx_path=?5, script_path=?6, video_path=?7, audio_path=?8,
        subtitle_path=?9, json_path=?10, log_path=?11, updated_at=?12, error_message=NULL WHERE id=?1
        "#,
        params![
            task_id,
            SUCCESS,
            task_dir.to_string_lossy(),
            pptx_path.map(|p| p.to_string_lossy().to_string()),
            docx_path.map(|p| p.to_string_lossy().to_string()),
            script_path.map(|p| p.to_string_lossy().to_string()),
            video_path.map(|p| p.to_string_lossy().to_string()),
            audio_path.map(|p| p.to_string_lossy().to_string()),
            subtitle_path.map(|p| p.to_string_lossy().to_string()),
            json_path.to_string_lossy().to_string(),
            log_path.to_string_lossy().to_string(),
            now_string()
        ],
    )?;
    append_log(&log_path, "任务完成")?;
    Ok(())
}

fn wants(outputs: &[String], key: &str) -> bool {
    outputs.is_empty() || outputs.iter().any(|item| item == key)
}

fn narration_config_for_outputs(config: &AppConfig, wants_video: bool) -> AppConfig {
    if config.enable_tts && !matches!(config.tts_provider, TtsProvider::None) {
        return config.clone();
    }
    if wants_video {
        return windows_sapi_fallback_config(config);
    }
    config.clone()
}

fn windows_sapi_fallback_config(config: &AppConfig) -> AppConfig {
    let mut fallback = config.clone();
    fallback.enable_tts = true;
    fallback.tts_provider = TtsProvider::WindowsSapi;
    if fallback.tts_voice.trim().is_empty() || fallback.tts_voice == "xiaozhi_clone" || fallback.tts_voice == "warm_female" || fallback.tts_voice == "clear_male" {
        fallback.tts_voice = "windows_default".into();
    }
    fallback
}

fn update_step(conn: &Connection, task_id: &str, status: &str, progress: i64, step: &str, output_dir: Option<&PathBuf>, log_path: Option<&PathBuf>) -> AppResult<()> {
    conn.execute(
        "UPDATE tasks SET status=?2, progress=?3, current_step=?4, output_dir=COALESCE(?5, output_dir), log_path=COALESCE(?6, log_path), updated_at=?7 WHERE id=?1",
        params![
            task_id,
            status,
            progress,
            step,
            output_dir.map(|p| p.to_string_lossy().to_string()),
            log_path.map(|p| p.to_string_lossy().to_string()),
            now_string()
        ],
    )?;
    Ok(())
}

fn update_failure(conn: &Connection, task_id: &str, message: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE tasks SET status=?2, progress=100, current_step='生成失败', error_message=?3, updated_at=?4 WHERE id=?1",
        params![task_id, FAILED, message, now_string()],
    )?;
    Ok(())
}

fn append_log(path: &PathBuf, line: &str) -> AppResult<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "[{}] {}", now_string(), line)?;
    Ok(())
}

fn build_srt(plan: &ReportPlan) -> String {
    let mut out = String::new();
    let mut second = 0u32;
    for (idx, slide) in plan.slides.iter().enumerate() {
        let duration = slide.estimated_seconds.max(20);
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            idx + 1,
            srt_time(second),
            srt_time(second + duration),
            slide.speaker_note
        ));
        second += duration;
    }
    out
}

fn srt_time(seconds: u32) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    format!("{h:02}:{m:02}:{s:02},000")
}
