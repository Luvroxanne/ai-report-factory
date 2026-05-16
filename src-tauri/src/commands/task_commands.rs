use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::{
    db::models::{insert_task, now_string, CreateTaskRequest, TaskRecord},
    services::tasks::{task_runner, task_status::PENDING},
    utils::errors::err_to_string,
    AppState,
};

#[tauri::command]
pub fn create_task(state: State<'_, AppState>, request: CreateTaskRequest) -> Result<TaskRecord, String> {
    if request.input_text.trim().is_empty() {
        return Err("请输入文本或上传 md/txt 文件".into());
    }
    let id = Uuid::new_v4().to_string();
    let now = now_string();
    let task = TaskRecord {
        id: id.clone(),
        title: request.title.trim().to_string(),
        input_file: request.input_file.clone(),
        input_text: Some(request.input_text.clone()),
        status: PENDING.into(),
        progress: 0,
        current_step: "等待生成".into(),
        output_dir: None,
        pptx_path: None,
        docx_path: None,
        script_path: None,
        video_path: None,
        audio_path: None,
        subtitle_path: None,
        json_path: None,
        log_path: None,
        created_at: now.clone(),
        updated_at: now,
        error_message: None,
    };
    {
        let conn = state.db.lock().map_err(|_| "数据库锁已损坏".to_string())?;
        insert_task(&conn, &task).map_err(err_to_string)?;
    }
    let db_path = state.paths.db_path.clone();
    let outputs_dir = state.paths.outputs_dir.clone();
    let config = state.config.lock().map_err(|_| "配置锁已损坏".to_string())?.clone();
    let base_style = request.style.unwrap_or_else(|| "agent-pro".into());
    let template = request.template.unwrap_or_else(|| "aurora-tech".into());
    let style = format!("{base_style}|template={template}");
    let outputs = request.outputs.unwrap_or_else(|| vec!["pptx".into(), "docx".into(), "script".into(), "subtitle".into(), "json".into()]);
    std::thread::spawn(move || task_runner::run_task(db_path, outputs_dir, config, id, style, outputs));
    Ok(task)
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>, search: Option<String>, status: Option<String>) -> Result<Vec<TaskRecord>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT * FROM tasks
             WHERE (?1 IS NULL OR title LIKE '%' || ?1 || '%' OR input_file LIKE '%' || ?1 || '%')
             AND (?2 IS NULL OR status=?2)
             ORDER BY created_at DESC LIMIT 200",
        )
        .map_err(err_to_string)?;
    let rows = stmt
        .query_map(params![search.filter(|s| !s.trim().is_empty()), status.filter(|s| !s.trim().is_empty())], TaskRecord::from_row)
        .map_err(err_to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err_to_string)
}

#[tauri::command]
pub fn get_task(state: State<'_, AppState>, id: String) -> Result<TaskRecord, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    conn.query_row("SELECT * FROM tasks WHERE id=?1", params![id], TaskRecord::from_row)
        .map_err(err_to_string)
}

#[tauri::command]
pub fn delete_task(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁已损坏".to_string())?;
    conn.execute("DELETE FROM tasks WHERE id=?1", params![id]).map_err(err_to_string)?;
    Ok(())
}

#[tauri::command]
pub fn rerun_task(state: State<'_, AppState>, id: String) -> Result<TaskRecord, String> {
    let old = get_task(state.clone(), id)?;
    create_task(
        state,
        CreateTaskRequest {
            title: old.title,
            input_file: old.input_file,
            input_text: old.input_text.unwrap_or_default(),
            style: Some("agent-pro".into()),
            template: Some("aurora-tech".into()),
            outputs: None,
        },
    )
}
