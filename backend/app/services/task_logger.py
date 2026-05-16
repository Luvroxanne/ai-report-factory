from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Any


class TaskLogger:
    def __init__(self, path: Path):
        self.path = path
        self.path.parent.mkdir(parents=True, exist_ok=True)

    def info(self, message: str, **extra: Any) -> None:
        self._write("INFO", message, extra)

    def warning(self, message: str, **extra: Any) -> None:
        self._write("WARN", message, extra)

    def error(self, message: str, **extra: Any) -> None:
        self._write("ERROR", message, extra)

    def _write(self, level: str, message: str, extra: dict[str, Any]) -> None:
        ts = datetime.now().isoformat(timespec="seconds")
        suffix = ""
        if extra:
            safe = {key: self._safe_value(value) for key, value in extra.items()}
            suffix = " " + " ".join(f"{key}={value}" for key, value in safe.items())
        with self.path.open("a", encoding="utf-8") as handle:
            handle.write(f"[{ts}] [{level}] {message}{suffix}\n")

    def _safe_value(self, value: Any) -> str:
        text = str(value)
        lowered = text.lower()
        if "sk-" in lowered or "token" in lowered or "api_key" in lowered:
            return "***"
        return text.replace("\n", " ")[:500]
