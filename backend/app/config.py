from __future__ import annotations

import os
import sys
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path


def _runtime_project_dir() -> Path:
    if getattr(sys, "frozen", False):
        base = os.getenv("AI_REPORT_FACTORY_HOME")
        if base:
            return Path(base).expanduser()
        local_app_data = os.getenv("LOCALAPPDATA")
        if local_app_data:
            return Path(local_app_data) / "AI Report Factory"
        return Path.home() / "AI Report Factory"
    return Path(__file__).resolve().parents[2]


def _runtime_backend_dir() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parents[1]


def _runtime_storage_dir() -> Path:
    # 打包后 sidecar 通常位于安装目录 / resources 目录，这些目录不一定可写。
    # 任务数据库、上传文件等运行时数据必须放到用户可写目录，避免 exe 启动后后端直接退出。
    if getattr(sys, "frozen", False):
        return _runtime_project_dir() / "storage"
    return _runtime_backend_dir() / "storage"


@dataclass(frozen=True)
class Settings:
    app_name: str = "AI报告工厂"
    api_prefix: str = "/api"

    backend_dir: Path = _runtime_backend_dir()
    project_dir: Path = _runtime_project_dir()

    config_dir: Path = project_dir / "config"
    logs_dir: Path = project_dir / "logs"
    outputs_dir: Path = project_dir / "outputs"

    storage_dir: Path = _runtime_storage_dir()
    input_dir: Path = storage_dir / "inputs"
    db_path: Path = storage_dir / "tasks.sqlite3"

    cors_origins: tuple[str, ...] = (
        "http://localhost:5173",
        "http://127.0.0.1:5173",
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
    )
    cors_origin_regex: str = r"^https?://tauri\.localhost(?::\d+)?$"

    config_path: Path = config_dir / "app_config.json"
    default_style: str = os.getenv("REPORT_STYLE", "official-tech")

    # 环境变量仅作为启动默认值；正式 Token 通过配置层读取，不写死在代码中。
    ai_provider: str = os.getenv("AI_PROVIDER", "ollama")
    openai_base_url: str = os.getenv("OPENAI_BASE_URL", "").rstrip("/")
    openai_api_key: str = os.getenv("OPENAI_API_KEY", "")
    openai_model: str = os.getenv("OPENAI_MODEL", "gpt-4o-mini")
    gemini_base_url: str = os.getenv("GEMINI_BASE_URL", "https://generativelanguage.googleapis.com").rstrip("/")
    gemini_api_key: str = os.getenv("GEMINI_API_KEY", "")
    gemini_model: str = os.getenv("GEMINI_MODEL", "gemini-1.5-flash")
    ollama_base_url: str = os.getenv("OLLAMA_BASE_URL", "").rstrip("/")
    ollama_model: str = os.getenv("OLLAMA_MODEL", "qwen2.5:7b")

    presenton_base_url: str = os.getenv("PRESENTON_BASE_URL", "").rstrip("/")
    presenton_username: str = os.getenv("PRESENTON_USERNAME", "")
    presenton_password: str = os.getenv("PRESENTON_PASSWORD", "")
    presenton_generate_endpoint: str = os.getenv(
        "PRESENTON_GENERATE_ENDPOINT",
        "/api/v1/ppt/presentation/generate",
    )

    wan_base_url: str = os.getenv("WAN_BASE_URL", os.getenv("COMFYUI_BASE_URL", "")).rstrip("/")
    comfyui_base_url: str = os.getenv("COMFYUI_BASE_URL", "").rstrip("/")

    cosyvoice_base_url: str = os.getenv("COSYVOICE_BASE_URL", "").rstrip("/")
    cosyvoice_tts_endpoint: str = os.getenv("COSYVOICE_TTS_ENDPOINT", "/api/tts")


def ensure_runtime_dirs(settings: Settings) -> None:
    for path in (
        settings.config_dir,
        settings.logs_dir,
        settings.outputs_dir,
        settings.storage_dir,
        settings.input_dir,
    ):
        path.mkdir(parents=True, exist_ok=True)


@lru_cache
def get_settings() -> Settings:
    settings = Settings()
    ensure_runtime_dirs(settings)
    return settings
