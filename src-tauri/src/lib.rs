use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use tauri::{AppHandle, Manager, State};

struct BackendState(Mutex<Option<Child>>);

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DesktopConfig {
    backend_url: String,
    auto_launch_backend: bool,
    output_dir: String,
}

#[derive(Debug, Serialize)]
struct RuntimeDependency {
    name: String,
    ok: bool,
    detail: String,
}

#[tauri::command]
fn backend_url() -> String {
    default_desktop_config().backend_url
}

#[tauri::command]
fn load_desktop_config() -> Result<DesktopConfig, String> {
    let path = desktop_config_path()?;
    if !path.exists() {
        return Ok(default_desktop_config());
    }
    let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| err.to_string())
}

#[tauri::command]
fn save_desktop_config(config: DesktopConfig) -> Result<DesktopConfig, String> {
    let path = desktop_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let text = serde_json::to_string_pretty(&config).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| err.to_string())?;
    Ok(config)
}

#[tauri::command]
fn check_runtime_dependencies() -> Vec<RuntimeDependency> {
    vec![
        check_command("python", "开发模式用于启动 FastAPI；正式版优先使用 sidecar exe"),
        check_command("ffmpeg", "MoviePy 合成 H.264/AAC 视频"),
        check_command("powershell", "Windows TTS 语音生成"),
    ]
}

#[tauri::command]
fn start_backend(app: AppHandle, state: State<'_, BackendState>) -> Result<(), String> {
    let resource_dir = app.path().resource_dir().ok();
    start_backend_inner(&state, resource_dir)
}

#[tauri::command]
fn stop_backend(state: State<'_, BackendState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|_| "后端状态锁定失败".to_string())?;
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

fn start_backend_inner(state: &BackendState, resource_dir: Option<PathBuf>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|_| "后端状态锁定失败".to_string())?;
    if guard.as_mut().and_then(|child| child.try_wait().ok()).flatten().is_none() && guard.is_some() {
        return Ok(());
    }
    if backend_health_ok() {
        return Ok(());
    }

    let root = app_root()?;
    let backend_dir = root.join("backend");
    let sidecar = find_backend_exe(&root, resource_dir.as_ref());
    let mut command = if let Some(sidecar) = sidecar {
        Command::new(sidecar)
    } else {
        let venv_python = if cfg!(windows) {
            root.join(".venv").join("Scripts").join("python.exe")
        } else {
            root.join(".venv").join("bin").join("python")
        };
        let mut cmd = if venv_python.exists() {
            Command::new(venv_python)
        } else {
            Command::new("python")
        };
        cmd.args(["-m", "uvicorn", "app.main:app", "--host", "127.0.0.1", "--port", "8000"]);
        cmd.current_dir(&backend_dir);
        cmd
    };
    if let Some(runtime_home) = runtime_home_dir() {
        command.env("AI_REPORT_FACTORY_HOME", &runtime_home);
        attach_sidecar_log(&mut command, &runtime_home);
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let child = command.spawn().map_err(|err| format!("启动后端失败：{err}"))?;
    *guard = Some(child);
    Ok(())
}

fn default_desktop_config() -> DesktopConfig {
    DesktopConfig {
        backend_url: "http://127.0.0.1:8000".to_string(),
        auto_launch_backend: true,
        output_dir: "outputs".to_string(),
    }
}

fn desktop_config_path() -> Result<PathBuf, String> {
    let base = runtime_home_dir().unwrap_or(app_root()?);
    Ok(base.join("config").join("desktop_config.json"))
}

fn runtime_home_dir() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("AI_REPORT_FACTORY_HOME") {
        return Some(PathBuf::from(value));
    }
    std::env::var_os("LOCALAPPDATA")
        .map(|local_app_data| PathBuf::from(local_app_data).join("AI Report Factory"))
}

fn attach_sidecar_log(command: &mut Command, runtime_home: &PathBuf) {
    let logs_dir = runtime_home.join("logs");
    if fs::create_dir_all(&logs_dir).is_ok() {
        let log_path = logs_dir.join("sidecar.log");
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let stderr = file.try_clone().ok();
            command.stdout(Stdio::from(file));
            if let Some(stderr) = stderr {
                command.stderr(Stdio::from(stderr));
            } else {
                command.stderr(Stdio::null());
            }
            return;
        }
    }
    command.stdout(Stdio::null()).stderr(Stdio::null());
}

fn backend_health_ok() -> bool {
    let mut addrs = match "127.0.0.1:8000".to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(_) => return false,
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(350)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(700)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(700)));
    if stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:8000\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok()
        && response.contains("200 OK")
        && response.contains("\"status\":\"ok\"")
}

fn find_backend_exe(root: &PathBuf, resource_dir: Option<&PathBuf>) -> Option<PathBuf> {
    let mut candidates = vec![
        root.join("backend").join("ai-report-backend.exe"),
        root.join("ai-report-backend.exe"),
        root.join("resources").join("backend").join("ai-report-backend.exe"),
        root.join("resources").join("ai-report-backend.exe"),
        root.join("_up_").join("backend").join("ai-report-backend.exe"),
        root.join("_up_").join("ai-report-backend.exe"),
    ];
    if let Some(resource_dir) = resource_dir {
        candidates.extend([
            resource_dir.join("backend").join("ai-report-backend.exe"),
            resource_dir.join("ai-report-backend.exe"),
            resource_dir.join("_up_").join("backend").join("ai-report-backend.exe"),
            resource_dir.join("_up_").join("ai-report-backend.exe"),
        ]);
    }
    candidates.into_iter().find(|path| path.exists())
}

fn app_root() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("AI_REPORT_FACTORY_HOME") {
        return Ok(PathBuf::from(value));
    }
    #[cfg(debug_assertions)]
    {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "无法定位应用目录".to_string())?
            .to_path_buf());
    }
    #[cfg(not(debug_assertions))]
    {
        let exe = std::env::current_exe().map_err(|err| err.to_string())?;
        return exe
            .parent()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| "无法定位可执行文件目录".to_string());
    }
}

fn check_command(name: &str, hint: &str) -> RuntimeDependency {
    let output = if cfg!(windows) {
        Command::new("cmd").args(["/C", "where", name]).output()
    } else {
        Command::new("sh").args(["-lc", &format!("command -v {name}")]).output()
    };
    match output {
        Ok(result) if result.status.success() => RuntimeDependency {
            name: name.to_string(),
            ok: true,
            detail: String::from_utf8_lossy(&result.stdout).trim().to_string(),
        },
        _ => RuntimeDependency {
            name: name.to_string(),
            ok: false,
            detail: hint.to_string(),
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(BackendState(Mutex::new(None)))
        .setup(|app| {
            let state = app.state::<BackendState>();
            if load_desktop_config().unwrap_or_else(|_| default_desktop_config()).auto_launch_backend {
                let resource_dir = app.path().resource_dir().ok();
                let _ = start_backend_inner(&state, resource_dir);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_url,
            load_desktop_config,
            save_desktop_config,
            check_runtime_dependencies,
            start_backend,
            stop_backend
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<BackendState>();
                let _ = stop_backend(state);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
