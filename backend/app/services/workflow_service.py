from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from app.adapters.comfyui_client import ComfyUIClient
from app.adapters.cosyvoice_client import CosyVoiceClient
from app.adapters.llm_client import LLMClient
from app.adapters.presenton_client import PresentonClient
from app.config import Settings
from app.services.app_config_service import AppConfigService
from app.services.document_service import DocumentService
from app.services.ppt_service import PPTService
from app.services.script_service import ScriptService
from app.services.task_logger import TaskLogger
from app.services.task_store import TaskStore
from app.services.video_service import VideoService
from app.services.voice_service import VoiceService


class WorkflowService:
    def __init__(self, settings: Settings, store: TaskStore, config_service: AppConfigService | None = None):
        self.settings = settings
        self.store = store
        self.config_service = config_service or AppConfigService(settings)
        self.document_service = DocumentService()
        self.llm = LLMClient()
        self.script_service = ScriptService()
        self.video_service = VideoService()

    def run(self, task_id: str, style: str | None = None) -> None:
        logger: TaskLogger | None = None
        task_dir: Path | None = None
        try:
            task = self.store.get(task_id)
            input_path = Path(task["input_path"])
            config = self.config_service.load()
            style = style or self.settings.default_style
            output_root = self.config_service.output_root(config)
            task_dir = output_root / task_id
            task_dir.mkdir(parents=True, exist_ok=True)
            log_path = task_dir / "generation.log"
            logger = TaskLogger(log_path)
            self.store.update(task_id, project_dir=str(task_dir), log_path=str(log_path))
            logger.info("任务开始", task_id=task_id, input=input_path, style=style)

            self._step(task_id, "parsing", "正在解析输入材料", 8)
            text = self.document_service.compact(self.document_service.read_text(input_path))
            logger.info("材料解析完成", chars=len(text))

            self._step(task_id, "generating_ppt", "正在生成报告结构 JSON", 22)
            plan = self.llm.build_report_plan(text, style, config, logger=logger)
            json_path = task_dir / "report_plan.json"
            json_path.write_text(json.dumps(plan, ensure_ascii=False, indent=2), encoding="utf-8")
            self.store.update(task_id, json_path=str(json_path))
            logger.info("结构 JSON 已生成", path=json_path, slides=len(plan.get("slides", [])), provider=plan.get("generation", {}).get("provider"), fallback=plan.get("generation", {}).get("fallback"))

            presenton_conf = config.get("services", {}).get("presenton", {})
            presenton = PresentonClient(
                str(presenton_conf.get("base_url") or "").rstrip("/"),
                str(presenton_conf.get("endpoint") or "/api/v1/ppt/presentation/generate"),
                str(presenton_conf.get("username") or ""),
                str(presenton_conf.get("password") or ""),
            )
            ppt_service = PPTService(presenton)
            self._step(task_id, "generating_ppt", "正在生成 PPTX 文件", 42)
            ppt_path = ppt_service.generate(
                task_id=task_id,
                text=text,
                plan=plan,
                output_dir=task_dir,
                logger=logger,
            )
            self.store.update(task_id, ppt_path=str(ppt_path))

            self._step(task_id, "generating_script", "正在生成 Word 解说稿", 56)
            scripts = self.script_service.build_scripts(plan)
            script_path = self.script_service.save(scripts, task_dir / "speaker_script.docx", report_title=f"{plan.get('title', 'AI报告')}解说稿")
            self.store.update(task_id, script_path=str(script_path))
            logger.info("解说稿 DOCX 已生成", path=script_path)

            cosy_conf = config.get("services", {}).get("cosyvoice", {})
            voice_service = VoiceService(CosyVoiceClient(
                str(cosy_conf.get("base_url") or "").rstrip("/"),
                str(cosy_conf.get("endpoint") or "/api/tts"),
            ))
            self._step(task_id, "generating_voice", "正在生成语音片段", 72)
            audio_manifest = voice_service.generate_pack(task_id, scripts, task_dir, logger=logger)
            audio_dir = task_dir / "audio"
            self.store.update(task_id, audio_dir=str(audio_dir))

            self._step(task_id, "generating_video", "正在准备 Wan2.2 分镜提示词", 80)
            wan_conf = config.get("services", {}).get("wan", {})
            comfyui = ComfyUIClient(str(wan_conf.get("base_url") or "").rstrip("/"))
            video_prompt_path = task_dir / "wan2.2_prompts.txt"
            comfyui.save_placeholder_workflow(self._build_video_prompts(plan), video_prompt_path)
            logger.info("Wan2.2 提示词已生成", path=video_prompt_path)

            wan_segments = comfyui.generate_video_segments(
                slides=plan.get("slides", []),
                output_dir=task_dir / "wan_segments",
                workflow_template_path=str(wan_conf.get("workflow_template_path") or ""),
                poll_timeout_seconds=int(wan_conf.get("poll_timeout_seconds") or 600),
                logger=logger,
            )
            if wan_segments:
                (task_dir / "wan_segments.json").write_text(json.dumps(wan_segments, ensure_ascii=False, indent=2), encoding="utf-8")

            self._step(task_id, "generating_video", "正在准备 1080P H.264 视频合成", 86)
            video_result = self.video_service.compose(
                task_id=task_id,
                plan=plan,
                scripts=scripts,
                audio_files=audio_manifest,
                output_dir=task_dir,
                frames_dir=task_dir / "frames",
                video_segments=wan_segments,
                logger=logger,
                progress_callback=lambda step, progress: self._step(task_id, "generating_video", step, progress),
            )
            video_path = Path(video_result["video_path"])
            subtitle_path = Path(video_result["subtitle_path"])

            metadata_path = task_dir / "metadata.json"
            metadata = self._metadata(task_id, task, plan, scripts, audio_manifest, {
                "ppt_path": ppt_path,
                "script_path": script_path,
                "video_path": video_path,
                "subtitle_path": subtitle_path,
                "json_path": json_path,
                "log_path": log_path,
                "video_prompt_path": video_prompt_path,
            }, wan_segments=wan_segments)
            metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2), encoding="utf-8")

            self.store.update(
                task_id,
                status="completed",
                current_step="完成",
                progress=100,
                video_path=str(video_path),
                subtitle_path=str(subtitle_path),
                metadata_path=str(metadata_path),
            )
            logger.info("任务完成", metadata=metadata_path)
        except Exception as exc:
            if logger:
                logger.error("任务失败", error=exc)
            self.store.update(
                task_id,
                status="failed",
                current_step="失败",
                error=str(exc),
            )

    def _step(self, task_id: str, status: str, step: str, progress: int) -> None:
        self.store.update(
            task_id,
            status=status,
            current_step=step,
            progress=progress,
            error=None,
        )

    def _build_video_prompts(self, plan: dict[str, Any]) -> str:
        lines: list[str] = []
        for index, slide in enumerate(plan.get("slides", []), start=1):
            prompt = slide.get("visual_prompt") or f"business presentation page {index}, clean motion graphics"
            lines.append(f"Page {index:02d}：{slide.get('title', '')}\n{prompt}\n")
        return "\n".join(lines)

    def _metadata(self, task_id: str, task: dict[str, Any], plan: dict[str, Any], scripts: list[dict[str, Any]], audio_manifest: list[dict[str, Any]], paths: dict[str, Path], wan_segments: list[dict[str, Any]] | None = None) -> dict[str, Any]:
        return {
            "task_id": task_id,
            "original_filename": task.get("original_filename"),
            "title": plan.get("title"),
            "style": plan.get("style"),
            "slides": len(plan.get("slides", [])),
            "scripts": scripts,
            "audio_manifest": audio_manifest,
            "wan_segments": wan_segments or [],
            "artifacts": {key: str(value) for key, value in paths.items()},
            "status": "completed",
        }
