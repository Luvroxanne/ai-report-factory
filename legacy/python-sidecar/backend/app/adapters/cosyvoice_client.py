from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import requests


@dataclass
class CosyVoiceClient:
    base_url: str = ""
    endpoint: str = "/api/tts"
    timeout: int = 180

    @property
    def enabled(self) -> bool:
        return bool(self.base_url)

    def health(self) -> tuple[bool, str]:
        if not self.enabled:
            return False, "未配置 CosyVoice 地址"
        try:
            resp = requests.get(self.base_url, timeout=8)
            return resp.status_code < 500, f"HTTP {resp.status_code}"
        except Exception as exc:
            return False, str(exc)

    def synthesize(self, text: str, output_path: Path, voice: str = "默认音色") -> Path | None:
        if not self.enabled:
            return None
        output_path.parent.mkdir(parents=True, exist_ok=True)
        resp = requests.post(
            f"{self.base_url}{self.endpoint}",
            json={"text": text, "voice": voice, "format": "wav"},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        output_path.write_bytes(resp.content)
        return output_path
