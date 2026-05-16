use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    config::app_config::{AppConfig, TtsProvider},
    db::models::ReportPlan,
    utils::errors::{AppError, AppResult},
};

#[derive(Debug, Clone, Serialize)]
pub struct VoiceOption {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub detail: String,
}

pub fn status() -> (&'static str, bool, &'static str) {
    ("TTS", true, "支持 Windows SAPI 本地语音；音色列表会读取本机已安装语音，也保留 OpenAI/Fish Speech 等可选增强入口")
}

pub fn voice_options() -> Vec<VoiceOption> {
    let mut voices = vec![VoiceOption {
        id: "windows_default".into(),
        label: "Windows 默认音色".into(),
        provider: "windows_sapi".into(),
        detail: "使用系统默认 SAPI 语音，不需要网络或额外下载".into(),
    }];
    voices.extend(installed_windows_sapi_voices());
    if voices.len() == 1 {
        voices.extend([
            VoiceOption { id: "Microsoft Huihui Desktop".into(), label: "中文女声 Huihui".into(), provider: "windows_sapi".into(), detail: "如果系统未安装会自动退回默认音色".into() },
            VoiceOption { id: "Microsoft Yaoyao Desktop".into(), label: "中文女声 Yaoyao".into(), provider: "windows_sapi".into(), detail: "如果系统未安装会自动退回默认音色".into() },
            VoiceOption { id: "Microsoft Kangkang Desktop".into(), label: "中文男声 Kangkang".into(), provider: "windows_sapi".into(), detail: "如果系统未安装会自动退回默认音色".into() },
        ]);
    }
    voices.extend([
        VoiceOption { id: "xiaozhi_clone".into(), label: "小智克隆音色（可选增强）".into(), provider: "fish_speech".into(), detail: "需使用你有授权的 Fish Speech/小智克隆服务；不会作为默认主流程依赖".into() },
        VoiceOption { id: "warm_female".into(), label: "温暖女声".into(), provider: "open_ai_compatible".into(), detail: "传给 OpenAI 兼容 TTS 服务的 voice 参数".into() },
        VoiceOption { id: "clear_male".into(), label: "清晰男声".into(), provider: "open_ai_compatible".into(), detail: "传给 OpenAI 兼容 TTS 服务的 voice 参数".into() },
    ]);
    voices
}

#[derive(Debug, Deserialize)]
struct SapiVoiceInfo {
    name: String,
    culture: String,
    gender: String,
    age: String,
}

fn installed_windows_sapi_voices() -> Vec<VoiceOption> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$items = $synth.GetInstalledVoices() | Where-Object { $_.Enabled } | ForEach-Object {
  $info = $_.VoiceInfo
  [pscustomobject]@{
    name = $info.Name
    culture = $info.Culture.Name
    gender = $info.Gender.ToString()
    age = $info.Age.ToString()
  }
}
$items | ConvertTo-Json -Compress
"#;
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide_window(&mut command);
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let items = if value.is_array() {
        serde_json::from_value::<Vec<SapiVoiceInfo>>(value).unwrap_or_default()
    } else {
        serde_json::from_value::<SapiVoiceInfo>(value).map(|item| vec![item]).unwrap_or_default()
    };
    items
        .into_iter()
        .map(|item| VoiceOption {
            id: item.name.clone(),
            label: item.name,
            provider: "windows_sapi".into(),
            detail: format!("本机已安装 · {} · {} · {}", item.culture, item.gender, item.age),
        })
        .collect()
}

pub fn generate_narration(plan: &ReportPlan, task_dir: &Path, config: &AppConfig) -> AppResult<Option<PathBuf>> {
    if !config.enable_tts || matches!(config.tts_provider, TtsProvider::None) {
        return Ok(None);
    }
    let narration_path = task_dir.join("narration.txt");
    fs::write(&narration_path, narration_text(plan))?;
    let wav_path = task_dir.join("narration.wav");
    match config.tts_provider {
        TtsProvider::None => Ok(None),
        TtsProvider::WindowsSapi => {
            generate_windows_sapi(&narration_path, &wav_path, &config.tts_voice)?;
            Ok(Some(wav_path))
        }
        TtsProvider::OpenAiCompatible | TtsProvider::FishSpeech => {
            generate_http_tts(&narration_path, &wav_path, config)?;
            Ok(Some(wav_path))
        }
    }
}

fn narration_text(plan: &ReportPlan) -> String {
    let mut text = String::new();
    text.push_str(&format!("大家好，今天汇报的主题是《{}》。\n", plan.title));
    for (index, slide) in plan.slides.iter().enumerate() {
        text.push_str(&format!("第 {} 页，{}。{}\n", index + 1, slide.title, slide.speaker_note));
    }
    text.push_str("以上就是本次报告内容。");
    text
}

fn generate_windows_sapi(text_path: &Path, wav_path: &Path, voice: &str) -> AppResult<()> {
    if !cfg!(windows) {
        return Err(AppError::Message("Windows SAPI TTS 仅支持 Windows".into()));
    }
    let script_path = wav_path.with_extension("tts.ps1");
    let script = r#"
param(
  [string]$TextPath,
  [string]$OutPath,
  [string]$Voice
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
if ($Voice -and $Voice -ne "windows_default") {
  try { $synth.SelectVoice($Voice) } catch { Write-Host "Voice not found, fallback to default: $Voice" }
}
$synth.Volume = 100
$synth.Rate = 0
$text = Get-Content -Raw -Encoding UTF8 $TextPath
$synth.SetOutputToWaveFile($OutPath)
$synth.Speak($text)
$synth.Dispose()
"#;
    fs::write(&script_path, script)?;
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.to_string_lossy(),
            "-TextPath",
            &text_path.to_string_lossy(),
            "-OutPath",
            &wav_path.to_string_lossy(),
            "-Voice",
            voice,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_window(&mut command);
    let output = command.output()?;
    if !output.status.success() || !wav_path.exists() || fs::metadata(wav_path)?.len() < 512 {
        return Err(AppError::Message(format!(
            "Windows SAPI 语音生成失败：{}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn generate_http_tts(text_path: &Path, wav_path: &Path, config: &AppConfig) -> AppResult<()> {
    let text = fs::read_to_string(text_path)?;
    let base = config.tts_base_url.trim_end_matches('/');
    let url = if base.ends_with("/audio/speech") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{base}/audio/speech")
    } else {
        format!("{base}/v1/audio/speech")
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_seconds.max(60)))
        .build()?;
    let mut request = client.post(url).json(&json!({
        "model": config.tts_model,
        "input": text,
        "voice": config.tts_voice,
        "response_format": "wav"
    }));
    if !config.tts_api_key.trim().is_empty() {
        request = request.bearer_auth(&config.tts_api_key);
    }
    let bytes = request.send()?.error_for_status()?.bytes()?;
    if bytes.len() < 512 {
        return Err(AppError::Message("TTS 服务返回内容过小，未生成有效 WAV".into()));
    }
    let mut file = fs::File::create(wav_path)?;
    file.write_all(&bytes)?;
    Ok(())
}

fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}
