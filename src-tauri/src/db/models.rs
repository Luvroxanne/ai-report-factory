use chrono::Utc;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub input_file: Option<String>,
    pub input_text: Option<String>,
    pub status: String,
    pub progress: i64,
    pub current_step: String,
    pub output_dir: Option<String>,
    pub pptx_path: Option<String>,
    pub docx_path: Option<String>,
    pub script_path: Option<String>,
    pub video_path: Option<String>,
    pub audio_path: Option<String>,
    pub subtitle_path: Option<String>,
    pub json_path: Option<String>,
    pub log_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub input_file: Option<String>,
    pub input_text: String,
    pub style: Option<String>,
    pub template: Option<String>,
    pub outputs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPlan {
    pub title: String,
    pub subtitle: String,
    pub summary: String,
    pub style: String,
    pub slides: Vec<SlidePlan>,
    pub generation_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidePlan {
    pub title: String,
    pub bullets: Vec<String>,
    pub speaker_note: String,
    pub layout: String,
    pub chapter: String,
    pub estimated_seconds: u32,
}

impl TaskRecord {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            input_file: row.get("input_file")?,
            input_text: row.get("input_text")?,
            status: row.get("status")?,
            progress: row.get("progress")?,
            current_step: row.get("current_step")?,
            output_dir: row.get("output_dir")?,
            pptx_path: row.get("pptx_path")?,
            docx_path: row.get("docx_path")?,
            script_path: row.get("script_path")?,
            video_path: row.get("video_path")?,
            audio_path: row.get("audio_path")?,
            subtitle_path: row.get("subtitle_path")?,
            json_path: row.get("json_path")?,
            log_path: row.get("log_path")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            error_message: row.get("error_message")?,
        })
    }
}

pub fn insert_task(conn: &Connection, task: &TaskRecord) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO tasks (
            id,title,input_file,input_text,status,progress,current_step,output_dir,
            pptx_path,docx_path,script_path,video_path,audio_path,subtitle_path,json_path,log_path,
            created_at,updated_at,error_message
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)
        "#,
        params![
            task.id,
            task.title,
            task.input_file,
            task.input_text,
            task.status,
            task.progress,
            task.current_step,
            task.output_dir,
            task.pptx_path,
            task.docx_path,
            task.script_path,
            task.video_path,
            task.audio_path,
            task.subtitle_path,
            task.json_path,
            task.log_path,
            task.created_at,
            task.updated_at,
            task.error_message,
        ],
    )?;
    Ok(())
}

pub fn now_string() -> String {
    Utc::now().to_rfc3339()
}
