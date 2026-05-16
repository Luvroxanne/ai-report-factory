from __future__ import annotations

from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class TaskStatus(str, Enum):
    pending = "pending"
    parsing = "parsing"
    generating_ppt = "generating_ppt"
    generating_script = "generating_script"
    generating_voice = "generating_voice"
    generating_video = "generating_video"
    completed = "completed"
    failed = "failed"


class TaskCreateResponse(BaseModel):
    task_id: str
    status: TaskStatus


class TaskView(BaseModel):
    id: str
    original_filename: str
    status: TaskStatus
    current_step: str = ""
    progress: int = Field(0, ge=0, le=100)
    project_dir: str | None = None
    ppt_path: str | None = None
    script_path: str | None = None
    video_path: str | None = None
    json_path: str | None = None
    audio_dir: str | None = None
    subtitle_path: str | None = None
    log_path: str | None = None
    metadata_path: str | None = None
    error: str | None = None
    created_at: str
    updated_at: str


class SlidePlan(BaseModel):
    title: str
    bullets: list[str] = Field(default_factory=list)
    visual_prompt: str = ""
    speaker_note: str = ""
    layout: str = "content"
    chapter: str = ""
    estimated_seconds: int = 0


class ReportPlan(BaseModel):
    title: str
    subtitle: str = ""
    summary: str = ""
    style: str = "official-tech"
    slides: list[SlidePlan]
    raw: dict[str, Any] = Field(default_factory=dict)


class AppConfigPayload(BaseModel):
    config: dict[str, Any]


class ProviderTestRequest(BaseModel):
    provider: str | None = None
    config: dict[str, Any] | None = None


class ProviderTestResponse(BaseModel):
    ok: bool
    provider: str
    message: str


class DependencyStatus(BaseModel):
    name: str
    ok: bool
    detail: str = ""
