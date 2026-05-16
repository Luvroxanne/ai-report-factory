from __future__ import annotations

import base64
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import requests


@dataclass
class PresentonClient:
    base_url: str = ""
    endpoint: str = "/api/v1/ppt/presentation/generate"
    username: str = ""
    password: str = ""
    timeout: int = 240

    @property
    def enabled(self) -> bool:
        return bool(self.base_url)

    def health(self) -> tuple[bool, str]:
        if not self.enabled:
            return False, "未配置 Presenton 地址"
        try:
            resp = requests.get(self.base_url, timeout=8)
            return resp.status_code < 500, f"HTTP {resp.status_code}"
        except Exception as exc:
            return False, str(exc)

    def generate_presentation(
        self,
        *,
        content: str,
        slides_markdown: list[str],
        instructions: str,
        output_path: Path,
    ) -> Path | None:
        if not self.enabled:
            return None

        url = f"{self.base_url}{self.endpoint}"
        auth = (self.username, self.password) if self.username or self.password else None
        resp = requests.post(
            url,
            json={
                "content": content,
                "slides_markdown": slides_markdown,
                "instructions": instructions,
                "tone": "professional",
            },
            auth=auth,
            timeout=self.timeout,
        )
        resp.raise_for_status()

        output_path.parent.mkdir(parents=True, exist_ok=True)
        content_type = resp.headers.get("content-type", "")
        if "presentation" in content_type or resp.content[:2] == b"PK":
            output_path.write_bytes(resp.content)
            return output_path

        payload: dict[str, Any] = resp.json()
        for key in ("pptx_base64", "presentation_base64", "file_base64"):
            if payload.get(key):
                output_path.write_bytes(base64.b64decode(payload[key]))
                return output_path

        for key in ("pptx_url", "download_url", "file_url"):
            if payload.get(key):
                file_url = payload[key]
                if file_url.startswith("/"):
                    file_url = f"{self.base_url}{file_url}"
                file_resp = requests.get(file_url, auth=auth, timeout=self.timeout)
                file_resp.raise_for_status()
                output_path.write_bytes(file_resp.content)
                return output_path

        return None
