use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    config::app_config::AppConfig,
    db::models::{ReportPlan, SlidePlan},
    utils::errors::{AppError, AppResult},
};

pub fn status(ffmpeg_enabled: bool, ffmpeg_found: bool) -> (&'static str, bool, String) {
    (
        "ffmpeg",
        ffmpeg_enabled && ffmpeg_found,
        if ffmpeg_enabled {
            if ffmpeg_found {
                "已检测到 ffmpeg，可直接生成 1080P MP4 视频".into()
            } else {
                "已启用视频但未检测到 ffmpeg；完整安装包和 portable zip 会内置 tools/ffmpeg/ffmpeg.exe，也可手动指定路径".into()
            }
        } else {
            "视频合成为可选能力，默认不影响 PPT/DOCX 主流程".into()
        },
    )
}

pub fn ffmpeg_exists(config: &AppConfig) -> bool {
    ffmpeg_executable(config).is_some()
}

pub fn ffmpeg_executable(config: &AppConfig) -> Option<PathBuf> {
    if !config.ffmpeg_path.trim().is_empty() {
        let path = PathBuf::from(config.ffmpeg_path.trim());
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("tools").join("ffmpeg").join("ffmpeg.exe"),
                dir.join("tools").join("ffmpeg").join("bin").join("ffmpeg.exe"),
                dir.join("ffmpeg.exe"),
                dir.join("resources").join("tools").join("ffmpeg").join("ffmpeg.exe"),
                dir.join("resources").join("tools").join("ffmpeg").join("bin").join("ffmpeg.exe"),
                dir.join("_up_").join("tools").join("ffmpeg").join("ffmpeg.exe"),
                dir.join("_up_").join("resources").join("tools").join("ffmpeg").join("ffmpeg.exe"),
            ] {
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "where", "ffmpeg"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .find(|line| !line.trim().is_empty())
                        .map(|line| PathBuf::from(line.trim()))
                } else {
                    None
                }
            })
    } else {
        Command::new("sh")
            .args(["-lc", "command -v ffmpeg"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    (!path.is_empty()).then(|| PathBuf::from(path))
                } else {
                    None
                }
            })
    }
}

pub fn generate_video(plan: &ReportPlan, task_dir: &Path, subtitle_path: &Path, audio_path: Option<&Path>, config: &AppConfig) -> AppResult<PathBuf> {
    if !config.enable_video || !config.enable_ffmpeg {
        return Err(AppError::Message("视频生成未启用：请在配置中心开启视频和 ffmpeg".into()));
    }
    let Some(ffmpeg) = ffmpeg_executable(config) else {
        return Err(AppError::Message("未检测到 ffmpeg：请使用完整安装包或带 tools/ffmpeg/ffmpeg.exe 的 portable zip，或在配置中心指定 ffmpeg 路径".into()));
    };

    let output = task_dir.join("report_video.mp4");
    let duration = total_duration(plan).clamp(8, 900);
    let size = format!("{}x{}", config.video_width.max(1280), config.video_height.max(720));
    let fps = config.video_fps.clamp(12, 60).to_string();
    let text_dir = task_dir.join("video_text");
    fs::create_dir_all(&text_dir)?;

    let filter_with_subtitles = video_filter(plan, &text_dir, Some(subtitle_path))?;
    let first = video_args(&output, audio_path, &size, &fps, duration, "h264_mf", &filter_with_subtitles);
    if let Err(first_err) = run_ffmpeg(&ffmpeg, &first) {
        let filter_without_subtitles = video_filter(plan, &text_dir, None)?;
        let second = video_args(&output, audio_path, &size, &fps, duration, "h264_mf", &filter_without_subtitles);
        if let Err(second_err) = run_ffmpeg(&ffmpeg, &second) {
            let third = video_args(&output, audio_path, &size, &fps, duration, "libopenh264", &filter_without_subtitles);
            if let Err(third_err) = run_ffmpeg(&ffmpeg, &third) {
                let fourth = video_args(&output, audio_path, &size, &fps, duration, "mpeg4", &filter_without_subtitles);
                run_ffmpeg(&ffmpeg, &fourth).map_err(|fourth_err| {
                    AppError::Message(format!(
                        "ffmpeg 生成视频失败。H.264+字幕错误：{first_err}；H.264无字幕错误：{second_err}；OpenH264错误：{third_err}；最终兼容重试错误：{fourth_err}"
                    ))
                })?;
            }
        }
    }

    if !output.exists() || fs::metadata(&output)?.len() < 10_000 {
        return Err(AppError::Message("ffmpeg 执行完成但 MP4 文件无效或过小".into()));
    }
    Ok(output)
}

fn video_args(output: &Path, audio_path: Option<&Path>, size: &str, fps: &str, duration: u32, codec: &str, filter: &str) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        format!("color=c=0x0B1120:s={size}:r={fps}:d={duration}"),
    ];
    if let Some(audio) = audio_path {
        args.extend(["-i".into(), audio.to_string_lossy().to_string()]);
    } else {
        args.extend(["-f".into(), "lavfi".into(), "-t".into(), duration.to_string(), "-i".into(), "anullsrc=channel_layout=stereo:sample_rate=44100".into()]);
    }
    args.extend(["-vf".into(), filter.to_string()]);
    args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "1:a:0".into(), "-t".into(), duration.to_string()]);
    args.extend(["-c:v".into(), codec.into()]);
    match codec {
        "h264_mf" => {
            args.extend(["-b:v".into(), "4500k".into(), "-pix_fmt".into(), "yuv420p".into()]);
        }
        "libopenh264" => {
            args.extend(["-b:v".into(), "4500k".into(), "-pix_fmt".into(), "yuv420p".into()]);
        }
        "libx264" => {
            args.extend(["-preset".into(), "veryfast".into(), "-crf".into(), "23".into(), "-pix_fmt".into(), "yuv420p".into(), "-profile:v".into(), "baseline".into(), "-level".into(), "4.0".into()]);
        }
        "mpeg4" => {
            args.extend(["-q:v".into(), "4".into(), "-tag:v".into(), "mp4v".into()]);
        }
        _ => {}
    }
    args.extend([
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "128k".into(),
        "-movflags".into(),
        "+faststart".into(),
        output.to_string_lossy().to_string(),
    ]);
    args
}

fn video_filter(plan: &ReportPlan, text_dir: &Path, subtitle_path: Option<&Path>) -> AppResult<String> {
    let mut filters = vec!["format=yuv420p".to_string()];
    let mut start = 0u32;
    let font = font_file().map(|p| filter_path(&p));

    for (idx, slide) in plan.slides.iter().enumerate() {
        let duration = slide.estimated_seconds.clamp(8, 60);
        let end = start + duration;
        write_slide_text_files(text_dir, idx, slide)?;
        let title_path = filter_path(&text_dir.join(format!("slide_{idx:02}_title.txt")));
        let body_path = filter_path(&text_dir.join(format!("slide_{idx:02}_body.txt")));
        let footer_path = filter_path(&text_dir.join(format!("slide_{idx:02}_footer.txt")));
        let (bg, accent, title_color, body_color) = palette(&plan.style, idx);
        let enable = format!("between(t\\,{start}\\,{end})");
        filters.push(format!("drawbox=x=0:y=0:w=iw:h=ih:color=0x{bg}@1:t=fill:enable='{enable}'"));
        filters.push(format!("drawbox=x=0:y=0:w=22:h=ih:color=0x{accent}@1:t=fill:enable='{enable}'"));
        filters.push(format!("drawbox=x=90:y=720:w=1740:h=220:color=0x000000@0.20:t=fill:enable='{enable}'"));
        filters.push(drawtext(&title_path, font.as_deref(), 64, title_color, 120, 145, 0, &enable));
        filters.push(drawtext(&body_path, font.as_deref(), 38, body_color, 140, 310, 22, &enable));
        filters.push(drawtext(&footer_path, font.as_deref(), 24, "CBD5E1", 1280, 990, 0, &enable));
        start = end;
    }

    if let Some(path) = subtitle_path {
        if path.exists() {
            filters.push(format!("subtitles='{}':force_style='FontName=Microsoft YaHei,FontSize=22,PrimaryColour=&HFFFFFF&,OutlineColour=&H111111&,BorderStyle=1,Outline=1'", filter_path(path)));
        }
    }
    Ok(filters.join(","))
}

fn write_slide_text_files(text_dir: &Path, idx: usize, slide: &SlidePlan) -> AppResult<()> {
    let title = wrap_text(&slide.title, 24);
    let mut body = String::new();
    body.push_str(&format!("【{}】\n", slide.chapter));
    for bullet in slide.bullets.iter().take(5) {
        body.push_str(&format!("• {}\n", wrap_text(bullet, 30)));
    }
    let footer = format!("AI Report Factory  |  Slide {}", idx + 1);
    fs::write(text_dir.join(format!("slide_{idx:02}_title.txt")), title)?;
    fs::write(text_dir.join(format!("slide_{idx:02}_body.txt")), body)?;
    fs::write(text_dir.join(format!("slide_{idx:02}_footer.txt")), footer)?;
    Ok(())
}

fn drawtext(textfile: &str, fontfile: Option<&str>, size: u32, color: &str, x: u32, y: u32, line_spacing: u32, enable: &str) -> String {
    let font = fontfile.map(|p| format!("fontfile='{p}':")).unwrap_or_default();
    format!(
        "drawtext={font}textfile='{textfile}':fontsize={size}:fontcolor=0x{color}:x={x}:y={y}:line_spacing={line_spacing}:enable='{enable}'"
    )
}

fn total_duration(plan: &ReportPlan) -> u32 {
    plan.slides.iter().map(|slide| slide.estimated_seconds.clamp(8, 60)).sum::<u32>().max(8)
}

fn wrap_text(input: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line_len = 0usize;
    for ch in input.chars() {
        if ch == '\n' {
            out.push(ch);
            line_len = 0;
            continue;
        }
        if line_len >= width && !matches!(ch, '，' | '。' | '；' | '、' | ':' | '：') {
            out.push('\n');
            line_len = 0;
        }
        out.push(ch);
        line_len += 1;
    }
    out
}

fn palette(style: &str, index: usize) -> (&'static str, &'static str, &'static str, &'static str) {
    let list = if style.contains("ivory-business") {
        [
            ("1F2937", "D97706", "FFFFFF", "FDE68A"),
            ("FFFBEB", "92400E", "1F2937", "6B4E16"),
            ("FFFFFF", "B45309", "1F2937", "92400E"),
            ("111827", "FCD34D", "FFFFFF", "FEF3C7"),
            ("FFF7ED", "CA8A04", "431407", "9A3412"),
        ]
    } else if style.contains("emerald-training") {
        [
            ("052E2B", "10B981", "F0FDFA", "99F6E4"),
            ("064E3B", "14B8A6", "ECFDF5", "D1FAE5"),
            ("ECFDF5", "059669", "064E3B", "047857"),
            ("042F2E", "0D9488", "FFFFFF", "CCFBF1"),
            ("F0FDF4", "22C55E", "052E16", "166534"),
        ]
    } else if style.contains("sunset-roadshow") {
        [
            ("2E1065", "F97316", "FFFFFF", "FED7AA"),
            ("431407", "A855F7", "FFF7ED", "FDBA74"),
            ("FFF7ED", "EC4899", "431407", "9A3412"),
            ("4C1D95", "F59E0B", "FFFFFF", "FEF3C7"),
            ("500724", "F97316", "FFFFFF", "FCE7F3"),
        ]
    } else if style.contains("violet-creative") {
        [
            ("1E1B4B", "8B5CF6", "FFFFFF", "DDD6FE"),
            ("312E81", "6366F1", "FFFFFF", "E0E7FF"),
            ("F5F3FF", "A855F7", "312E81", "6D28D9"),
            ("4C1D95", "EC4899", "FFFFFF", "FCE7F3"),
            ("0E7490", "8B5CF6", "FFFFFF", "CFFAFE"),
        ]
    } else {
        [
            ("0F172A", "38BDF8", "FFFFFF", "E0F2FE"),
            ("111827", "2563EB", "FFFFFF", "DBEAFE"),
            ("052E2B", "10B981", "ECFDF5", "D1FAE5"),
            ("1E1B4B", "8B5CF6", "FFFFFF", "EDE9FE"),
            ("3B1D06", "F59E0B", "FFFBEB", "FEF3C7"),
        ]
    };
    list[index % list.len()]
}

fn font_file() -> Option<PathBuf> {
    if cfg!(windows) {
        for path in [
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\arial.ttf",
        ] {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn filter_path(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.to_string_lossy()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

fn run_ffmpeg(ffmpeg: &Path, args: &[String]) -> AppResult<()> {
    let mut command = Command::new(ffmpeg);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    hide_window(&mut command);
    let output = command.output()?;
    if !output.status.success() {
        return Err(AppError::Message(String::from_utf8_lossy(&output.stderr).chars().take(3000).collect()));
    }
    Ok(())
}

fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}
