from __future__ import annotations

import copy
import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.parse import urlencode

import requests


@dataclass
class ComfyUIClient:
    base_url: str = ""
    timeout: int = 240

    @property
    def enabled(self) -> bool:
        return bool(self.base_url)

    def health(self) -> tuple[bool, str]:
        if not self.enabled:
            return False, "未配置 Wan2.2/ComfyUI 地址"
        try:
            resp = requests.get(f"{self.base_url}/system_stats", timeout=8)
            return resp.ok, "Wan2.2/ComfyUI 可用" if resp.ok else f"HTTP {resp.status_code}"
        except Exception as exc:
            return False, str(exc)

    def submit_prompt(self, workflow: dict[str, Any]) -> dict[str, Any] | None:
        if not self.enabled:
            return None
        resp = requests.post(f"{self.base_url}/prompt", json={"prompt": workflow}, timeout=self.timeout)
        resp.raise_for_status()
        return resp.json()

    def generate_video_segments(
        self,
        *,
        slides: list[dict[str, Any]],
        output_dir: Path,
        workflow_template_path: str = "",
        poll_timeout_seconds: int = 600,
        logger: Any | None = None,
    ) -> list[dict[str, Any]]:
        """调用 Wan2.2/ComfyUI 生成视频片段。

        当 workflow_template_path 指向 ComfyUI workflow JSON 时，会替换
        {{prompt}}、{{title}}、{{page}} 占位符并提交；失败时返回空列表继续本地兜底。
        """
        if not self.enabled:
            return []
        if not workflow_template_path:
            if logger:
                logger.info("Wan2.2 未配置 workflow_template_path，跳过动态视频生成")
            return []
        template_path = Path(workflow_template_path).expanduser()
        if not template_path.exists():
            if logger:
                logger.warning("Wan2.2 workflow 模板不存在，跳过动态视频生成", path=template_path)
            return []

        output_dir.mkdir(parents=True, exist_ok=True)
        template = json.loads(template_path.read_text(encoding="utf-8"))
        segments: list[dict[str, Any]] = []
        jobs: list[dict[str, Any]] = []
        for index, slide in enumerate(slides, start=1):
            prompt = str(slide.get("visual_prompt") or slide.get("title") or f"page {index}")
            context = {"prompt": prompt, "title": str(slide.get("title") or ""), "page": f"{index:02d}"}
            workflow = self._replace_placeholders(copy.deepcopy(template), context)
            try:
                submitted = self.submit_prompt(workflow) or {}
                prompt_id = str(submitted.get("prompt_id") or submitted.get("id") or "")
                jobs.append({"page": index, "prompt_id": prompt_id, "title": context["title"]})
                if not prompt_id:
                    continue
                history = self._wait_history(prompt_id, poll_timeout_seconds)
                media = self._download_first_media(prompt_id, history, output_dir / f"page_{index:02d}")
                if media:
                    item = {"page": index, "path": str(media), "prompt_id": prompt_id, "engine": "wan2.2"}
                    segments.append(item)
                    if logger:
                        logger.info("Wan2.2 视频片段生成完成", page=index, path=media)
            except Exception as exc:
                if logger:
                    logger.warning("Wan2.2 视频片段生成失败，继续本地兜底", page=index, reason=exc)
        (output_dir / "wan_jobs.json").write_text(json.dumps(jobs, ensure_ascii=False, indent=2), encoding="utf-8")
        return segments

    def save_placeholder_workflow(self, prompt: str, output_path: Path) -> Path:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            "Wan2.2/ComfyUI 提示词占位文件，配置 workflow_template_path 后可自动提交。\n\n" + prompt.strip() + "\n",
            encoding="utf-8",
        )
        return output_path

    def _wait_history(self, prompt_id: str, timeout_seconds: int) -> dict[str, Any]:
        deadline = time.time() + max(10, timeout_seconds)
        while time.time() < deadline:
            resp = requests.get(f"{self.base_url}/history/{prompt_id}", timeout=20)
            if resp.ok:
                data = resp.json()
                if data.get(prompt_id):
                    return data[prompt_id]
            time.sleep(2)
        raise TimeoutError(f"Wan2.2/ComfyUI prompt timeout: {prompt_id}")

    def _download_first_media(self, prompt_id: str, history: dict[str, Any], prefix: Path) -> Path | None:
        outputs = history.get("outputs") or {}
        for output in outputs.values():
            for key in ("videos", "gifs", "images"):
                for media in output.get(key, []) or []:
                    filename = media.get("filename")
                    if not filename:
                        continue
                    params = urlencode(
                        {
                            "filename": filename,
                            "subfolder": media.get("subfolder", ""),
                            "type": media.get("type", "output"),
                        }
                    )
                    resp = requests.get(f"{self.base_url}/view?{params}", timeout=self.timeout)
                    resp.raise_for_status()
                    suffix = Path(filename).suffix or (".mp4" if key in {"videos", "gifs"} else ".png")
                    path = prefix.with_suffix(suffix)
                    path.write_bytes(resp.content)
                    return path
        return None

    def _replace_placeholders(self, value: Any, context: dict[str, str]) -> Any:
        if isinstance(value, str):
            for key, item in context.items():
                value = value.replace("{{" + key + "}}", item)
            return value
        if isinstance(value, list):
            return [self._replace_placeholders(item, context) for item in value]
        if isinstance(value, dict):
            return {key: self._replace_placeholders(item, context) for key, item in value.items()}
        return value
