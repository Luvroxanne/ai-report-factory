from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

from app.config import Settings

_SECRET_KEYS = {"api_key", "token", "password", "secret"}


class AppConfigService:
    def __init__(self, settings: Settings):
        self.settings = settings
        self.path = settings.config_path

    def default_config(self) -> dict[str, Any]:
        return {
            "output_dir": str(self.settings.outputs_dir),
            "ai": {
                "active_provider": self.settings.ai_provider,
                "timeout_seconds": 90,
                "retries": 2,
                "providers": {
                    "openai": {
                        "base_url": self.settings.openai_base_url or "https://api.openai.com/v1",
                        "api_key": self.settings.openai_api_key,
                        "model": self.settings.openai_model,
                    },
                    "gemini": {
                        "base_url": self.settings.gemini_base_url,
                        "api_key": self.settings.gemini_api_key,
                        "model": self.settings.gemini_model,
                    },
                    "ollama": {
                        "base_url": self.settings.ollama_base_url,
                        "model": self.settings.ollama_model,
                    },
                    "local": {
                        "base_url": "",
                        "api_key": "",
                        "model": "",
                    },
                },
            },
            "services": {
                "presenton": {
                    "base_url": self.settings.presenton_base_url,
                    "endpoint": self.settings.presenton_generate_endpoint,
                    "username": self.settings.presenton_username,
                    "password": self.settings.presenton_password,
                },
                "cosyvoice": {
                    "base_url": self.settings.cosyvoice_base_url,
                    "endpoint": self.settings.cosyvoice_tts_endpoint,
                },
                "wan": {
                    "base_url": self.settings.wan_base_url or self.settings.comfyui_base_url,
                    "mode": "comfyui",
                    "workflow_template_path": "",
                    "poll_timeout_seconds": 600,
                },
            },
            "video": {
                "width": 1920,
                "height": 1080,
                "fps": 24,
                "codec": "libx264",
                "audio_codec": "aac",
                "enable_subtitles": True,
            },
            "desktop": {
                "auto_launch_backend": True,
                "backend_url": "http://127.0.0.1:8000",
            },
        }

    def load(self) -> dict[str, Any]:
        config = self.default_config()
        if self.path.exists():
            try:
                user_config = json.loads(self.path.read_text(encoding="utf-8"))
                self._deep_merge(config, user_config)
            except Exception:
                # 空密钥表示保留已有配置，避免设置页回传掩码后覆盖真实 Token。
                pass
        return config

    def masked(self) -> dict[str, Any]:
        return self._mask(copy.deepcopy(self.load()))

    def save(self, incoming: dict[str, Any]) -> dict[str, Any]:
        current = self.load()
        clean = copy.deepcopy(incoming)
        self._drop_empty_secrets(clean, current)
        self._deep_merge(current, clean)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps(current, ensure_ascii=False, indent=2), encoding="utf-8")
        return self.masked()

    def output_root(self, config: dict[str, Any] | None = None) -> Path:
        value = (config or self.load()).get("output_dir") or str(self.settings.outputs_dir)
        path = Path(str(value)).expanduser()
        if not path.is_absolute():
            path = self.settings.project_dir / path
        path.mkdir(parents=True, exist_ok=True)
        return path

    def _deep_merge(self, base: dict[str, Any], extra: dict[str, Any]) -> None:
        for key, value in extra.items():
            if isinstance(value, dict) and isinstance(base.get(key), dict):
                self._deep_merge(base[key], value)
            else:
                base[key] = value

    def _mask(self, value: Any) -> Any:
        if isinstance(value, dict):
            result: dict[str, Any] = {}
            for key, item in value.items():
                if key.lower() in _SECRET_KEYS:
                    result[key] = self._mask_secret(item)
                    result[f"{key}_configured"] = bool(item)
                else:
                    result[key] = self._mask(item)
            return result
        if isinstance(value, list):
            return [self._mask(item) for item in value]
        return value

    def _mask_secret(self, value: Any) -> str:
        text = str(value or "")
        if not text:
            return ""
        if len(text) <= 8:
            return "********"
        return f"{text[:3]}****{text[-4:]}"

    def _drop_empty_secrets(self, incoming: Any, current: Any) -> None:
        if not isinstance(incoming, dict):
            return
        for key in list(incoming.keys()):
            value = incoming[key]
            current_value = current.get(key) if isinstance(current, dict) else None
            if key.lower() in _SECRET_KEYS and (value is None or str(value).strip() in {"", "********"}):
                incoming.pop(key)
                continue
            if isinstance(value, dict):
                self._drop_empty_secrets(value, current_value if isinstance(current_value, dict) else {})
