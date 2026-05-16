use std::{fs, path::Path, process::Command};

use ai_report_factory_lib::{
    config::app_config::{self, AiProvider, AppConfig, TtsProvider},
    db::{
        migrations::open_and_migrate,
        models::{insert_task, now_string, ReportPlan, TaskRecord},
    },
    services::{
        files::file_indexer::{read_preview, scan_outputs},
        optional::tts,
        optional::video,
        tasks::{task_runner, task_status::SUCCESS},
    },
    utils::paths::AppPaths,
};
use rusqlite::params;
use uuid::Uuid;
use zip::ZipArchive;

#[test]
fn local_generation_creates_real_outputs_and_sqlite_history() {
    let root = std::env::temp_dir().join(format!("ai-report-factory-verify-{}", Uuid::new_v4()));
    let storage = root.join("storage");
    let outputs = root.join("outputs");
    fs::create_dir_all(&storage).expect("create storage");
    fs::create_dir_all(&outputs).expect("create outputs");
    let db_path = storage.join("tasks.sqlite3");

    let conn = open_and_migrate(&db_path).expect("migrate sqlite");
    let task_id = Uuid::new_v4().to_string();
    let now = now_string();
    let task = TaskRecord {
        id: task_id.clone(),
        title: "真实性验收报告".into(),
        input_file: Some("acceptance.md".into()),
        input_text: Some(
            r#"# 真实性验收

## 架构目标
- Vue 3 + TypeScript + Tauri 2 + Rust 内置后端
- 不使用 Python sidecar
- 不依赖 Presenton、CosyVoice、Wan2.2、ComfyUI

## 本地能力
- 生成 PPTX
- 生成 DOCX
- 生成 TXT/MD 讲稿
- 生成字幕和分镜 JSON
- 保存 SQLite 历史

## 发布能力
- GitHub Actions 构建 portable exe
- 生成 SHA256 校验
- tag 自动发布 Release
"#
            .into(),
        ),
        status: "pending".into(),
        progress: 0,
        current_step: "等待验收生成".into(),
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
    insert_task(&conn, &task).expect("insert task");
    drop(conn);

    let config = AppConfig {
        ai_provider: AiProvider::Local,
        output_dir: outputs.to_string_lossy().to_string(),
        enable_local_fallback: true,
        enable_tts: false,
        enable_video: false,
        ..AppConfig::default()
    };

    task_runner::run_task(
        db_path.clone(),
        outputs.clone(),
        config,
        task_id.clone(),
        "official-tech".into(),
        vec!["pptx".into(), "docx".into(), "script".into(), "subtitle".into(), "json".into()],
    );

    let conn = open_and_migrate(&db_path).expect("reopen sqlite");
    let saved: TaskRecord = conn
        .query_row("SELECT * FROM tasks WHERE id=?1", params![task_id], TaskRecord::from_row)
        .expect("read saved task");
    assert_eq!(saved.status, SUCCESS);
    assert_eq!(saved.progress, 100);
    assert!(saved.error_message.is_none(), "task error: {:?}", saved.error_message);

    let pptx = saved.pptx_path.as_deref().expect("pptx_path");
    let docx = saved.docx_path.as_deref().expect("docx_path");
    let script = saved.script_path.as_deref().expect("script_path");
    let subtitle = saved.subtitle_path.as_deref().expect("subtitle_path");
    let storyboard = saved.json_path.as_deref().expect("json_path");
    let log = saved.log_path.as_deref().expect("log_path");

    assert_real_pptx(Path::new(pptx));
    assert_real_docx(Path::new(docx));
    assert_text_contains(Path::new(script), "真实性验收报告");
    assert_text_contains(&Path::new(script).with_extension("txt"), "真实性验收报告");
    assert_text_contains(Path::new(subtitle), "真实性验收报告");
    assert_text_contains(Path::new(log), "任务完成");

    let plan_text = fs::read_to_string(storyboard).expect("read storyboard");
    let plan: ReportPlan = serde_json::from_str(&plan_text).expect("storyboard is legal JSON");
    assert!(plan.slides.len() >= 4, "expected at least title, toc, content, summary slides");

    let indexed = scan_outputs(&outputs).expect("scan outputs");
    assert!(indexed.iter().any(|f| f.name == "report.pptx" && f.size > 1_000 && f.file_type == "pptx"));
    assert!(indexed.iter().any(|f| f.name == "speaker_script.docx" && f.size > 700 && f.file_type == "docx"));
    assert!(indexed.iter().any(|f| f.name == "storyboard.json" && f.previewable));
    assert!(read_preview(Path::new(storyboard)).expect("preview storyboard").contains("真实性验收报告"));
    assert!(read_preview(Path::new(log)).expect("preview log").contains("任务完成"));

    println!("TEST_TASK_ID={}", saved.id);
    println!("TEST_DB={}", db_path.display());
    println!("TEST_OUTPUT_DIR={}", saved.output_dir.unwrap_or_default());
    println!("TEST_PPTX={pptx}");
    println!("TEST_DOCX={docx}");
    println!("TEST_SCRIPT={script}");
    println!("TEST_SUBTITLE={subtitle}");
    println!("TEST_STORYBOARD={storyboard}");
}

#[cfg(windows)]
#[test]
fn windows_sapi_tts_generates_real_wav() {
    let root = std::env::temp_dir().join(format!("ai-report-factory-tts-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create tts dir");
    let plan = ReportPlan {
        title: "小智音色测试".into(),
        subtitle: "语音旁白".into(),
        summary: "验证 Windows SAPI 能生成真实 WAV".into(),
        style: "voice".into(),
        slides: vec![ai_report_factory_lib::db::models::SlidePlan {
            title: "第一页".into(),
            bullets: vec!["语音生成".into()],
            speaker_note: "这是一段用于视频旁白的测试语音。".into(),
            layout: "content".into(),
            chapter: "测试".into(),
            estimated_seconds: 20,
        }],
        generation_note: "tts-test".into(),
    };
    let config = AppConfig {
        enable_tts: true,
        tts_provider: TtsProvider::WindowsSapi,
        tts_voice: "windows_default".into(),
        ..AppConfig::default()
    };
    let wav = tts::generate_narration(&plan, &root, &config)
        .expect("generate tts")
        .expect("wav path");
    assert!(wav.exists(), "wav missing: {}", wav.display());
    assert!(fs::metadata(&wav).expect("wav metadata").len() > 1024, "wav too small");
}

#[test]
fn config_is_saved_to_user_writable_home_and_recovers_when_broken() {
    let root = std::env::temp_dir().join(format!("ai-report-factory-config-{}", Uuid::new_v4()));
    std::env::set_var("AI_REPORT_FACTORY_HOME", &root);
    let paths = AppPaths::init().expect("init app paths");

    let mut config = app_config::load_or_default(&paths).expect("load default config");
    assert!(paths.config_path.exists());
    assert!(config.enable_local_fallback);
    assert!(config.enable_tts);
    assert!(config.enable_video);
    assert!(config.enable_ffmpeg);
    assert!(matches!(config.tts_provider, TtsProvider::WindowsSapi));
    assert!(matches!(config.ai_provider, AiProvider::Local));

    config.model_name = "local-test-model".into();
    config.output_dir = paths.outputs_dir.to_string_lossy().to_string();
    app_config::save(&paths, &config).expect("save config");
    let reloaded = app_config::load_or_default(&paths).expect("reload config");
    assert_eq!(reloaded.model_name, "local-test-model");

    fs::write(&paths.config_path, "{ broken json").expect("write broken config");
    let recovered = app_config::load_or_default(&paths).expect("recover default config");
    assert!(matches!(recovered.ai_provider, AiProvider::Local));
    assert!(paths.config_path.with_extension("broken.json").exists());
}

#[test]
fn bundled_ffmpeg_can_create_real_1080p_mp4_when_available() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ffmpeg = manifest_dir
        .parent()
        .expect("repo root")
        .join("resources")
        .join("tools")
        .join("ffmpeg")
        .join("ffmpeg.exe");
    if !ffmpeg.exists() {
        eprintln!("skip video generation test, bundled ffmpeg not found: {}", ffmpeg.display());
        return;
    }

    let root = std::env::temp_dir().join(format!("ai-report-factory-video-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create video dir");
    let subtitle = root.join("subtitle.srt");
    fs::write(
        &subtitle,
        "1\n00:00:00,000 --> 00:00:03,000\nAI Report Factory 1080P 视频合成测试\n\n",
    )
    .expect("write subtitle");
    let tone = root.join("tone.wav");
    let tone_status = Command::new(&ffmpeg)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=8",
            "-c:a",
            "pcm_s16le",
            &tone.to_string_lossy(),
        ])
        .status()
        .expect("create tone wav");
    assert!(tone_status.success(), "failed to create tone wav");
    let plan = ReportPlan {
        title: "1080P 视频测试".into(),
        subtitle: "内置 ffmpeg".into(),
        summary: "验证无需用户安装 ffmpeg 也能生成 MP4".into(),
        style: "video".into(),
        slides: vec![ai_report_factory_lib::db::models::SlidePlan {
            title: "第一页".into(),
            bullets: vec!["内置 ffmpeg".into()],
            speaker_note: "AI Report Factory 正在生成真实 MP4 视频。".into(),
            layout: "content".into(),
            chapter: "测试".into(),
            estimated_seconds: 3,
        }],
        generation_note: "video-test".into(),
    };
    let config = AppConfig {
        enable_video: true,
        enable_ffmpeg: true,
        ffmpeg_path: ffmpeg.to_string_lossy().to_string(),
        video_width: 1920,
        video_height: 1080,
        video_fps: 24,
        ..AppConfig::default()
    };
    let mp4 = video::generate_video(&plan, &root, &subtitle, Some(&tone), &config).expect("generate mp4");
    assert!(mp4.exists(), "mp4 missing: {}", mp4.display());
    assert!(fs::metadata(&mp4).expect("mp4 metadata").len() > 10_000, "mp4 too small");
    let bytes = fs::read(&mp4).expect("read mp4");
    assert!(bytes.windows(4).any(|w| w == b"ftyp"), "mp4 missing ftyp box");
    let info = Command::new(&ffmpeg)
        .args(["-hide_banner", "-i", &mp4.to_string_lossy()])
        .output()
        .expect("ffmpeg inspect mp4");
    let stderr = String::from_utf8_lossy(&info.stderr);
    assert!(stderr.contains("Video: h264"), "mp4 should use Windows-compatible H.264, got: {stderr}");
    assert!(stderr.contains("Audio: aac"), "mp4 should use AAC audio, got: {stderr}");
    let audio = Command::new(&ffmpeg)
        .args(["-v", "error", "-i", &mp4.to_string_lossy(), "-map", "0:a:0", "-t", "1", "-f", "s16le", "-"])
        .output()
        .expect("extract mp4 audio");
    assert!(audio.status.success(), "extract audio failed");
    assert!(audio.stdout.iter().any(|b| *b != 0), "mp4 audio should not be silent");
}

fn assert_real_pptx(path: &Path) {
    assert!(path.exists(), "pptx missing: {}", path.display());
    assert!(fs::metadata(path).expect("pptx metadata").len() > 1_000, "pptx too small");
    let file = fs::File::open(path).expect("open pptx");
    let mut zip = ZipArchive::new(file).expect("pptx zip");
    zip.by_name("[Content_Types].xml").expect("pptx content types");
    zip.by_name("ppt/presentation.xml").expect("ppt presentation");
    zip.by_name("ppt/slides/slide1.xml").expect("ppt title slide");
    zip.by_name("ppt/slides/slide2.xml").expect("ppt toc slide");
    zip.by_name("ppt/slides/slide3.xml").expect("ppt content slide");
    assert!(zip.len() >= 8, "pptx should contain multiple OOXML parts");
}

fn assert_real_docx(path: &Path) {
    assert!(path.exists(), "docx missing: {}", path.display());
    assert!(fs::metadata(path).expect("docx metadata").len() > 700, "docx too small");
    let file = fs::File::open(path).expect("open docx");
    let mut zip = ZipArchive::new(file).expect("docx zip");
    zip.by_name("[Content_Types].xml").expect("docx content types");
    let mut document = String::new();
    use std::io::Read;
    zip.by_name("word/document.xml")
        .expect("docx document")
        .read_to_string(&mut document)
        .expect("read docx xml");
    assert!(document.contains("真实性验收报告"));
    assert!(document.contains("解说稿"));
}

fn assert_text_contains(path: &Path, needle: &str) {
    assert!(path.exists(), "text file missing: {}", path.display());
    let text = fs::read_to_string(path).expect("read text file");
    assert!(text.contains(needle), "{} does not contain {}", path.display(), needle);
}
