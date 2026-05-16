from __future__ import annotations

import shutil
import uuid
from pathlib import Path
from typing import Any

from fastapi import BackgroundTasks, FastAPI, File, Form, HTTPException, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from app.adapters.llm_client import LLMClient
from app.config import get_settings
from app.schemas import AppConfigPayload, ProviderTestRequest, ProviderTestResponse, TaskCreateResponse, TaskStatus, TaskView
from app.services.app_config_service import AppConfigService
from app.services.dependency_service import DependencyService
from app.services.task_store import TaskStore
from app.services.workflow_service import WorkflowService
from app.version import APP_VERSION

settings = get_settings()
store = TaskStore(settings.db_path)
config_service = AppConfigService(settings)
workflow = WorkflowService(settings, store, config_service)
dependency_service = DependencyService()

app = FastAPI(title=settings.app_name, version=APP_VERSION)
app.add_middleware(
    CORSMiddleware,
    allow_origins=list(settings.cors_origins),
    allow_origin_regex=settings.cors_origin_regex,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount("/storage", StaticFiles(directory=str(settings.storage_dir)), name="storage")
app.mount("/outputs", StaticFiles(directory=str(settings.outputs_dir)), name="outputs")


@app.get("/api/health")
def health() -> dict[str, str]:
    return {"status": "ok", "name": settings.app_name, "version": APP_VERSION}


@app.get("/api/config")
def get_config() -> dict[str, Any]:
    return {"config": config_service.masked()}


@app.put("/api/config")
def save_config(payload: AppConfigPayload) -> dict[str, Any]:
    return {"config": config_service.save(payload.config)}


@app.post("/api/config/test", response_model=ProviderTestResponse)
def test_provider(payload: ProviderTestRequest) -> ProviderTestResponse:
    config = config_service.load()
    if payload.config:
        _deep_merge(config, payload.config)
    provider = payload.provider or str(config.get("ai", {}).get("active_provider") or "local")
    ok, message = LLMClient().test_provider(provider, config)
    return ProviderTestResponse(ok=ok, provider=provider, message=message)


@app.get("/api/dependencies")
def dependencies() -> dict[str, Any]:
    return {"items": dependency_service.check()}


@app.post("/api/tasks", response_model=TaskCreateResponse)
async def create_task(
    background_tasks: BackgroundTasks,
    file: UploadFile = File(...),
    style: str = Form("official-tech"),
) -> TaskCreateResponse:
    original_name = file.filename or "input.md"
    suffix = Path(original_name).suffix.lower()
    if suffix not in {".md", ".txt"}:
        raise HTTPException(status_code=400, detail="仅支持上传 .md / .txt 文件")

    task_id = uuid.uuid4().hex
    safe_name = f"{task_id}{suffix}"
    input_path = settings.input_dir / safe_name
    with input_path.open("wb") as out:
        shutil.copyfileobj(file.file, out)

    store.create(task_id, original_name, input_path)
    background_tasks.add_task(workflow.run, task_id, style)
    return TaskCreateResponse(task_id=task_id, status=TaskStatus.pending)


@app.get("/api/tasks", response_model=list[TaskView])
def list_tasks() -> list[dict[str, Any]]:
    return store.list_recent()


@app.get("/api/tasks/{task_id}", response_model=TaskView)
def get_task(task_id: str) -> dict[str, Any]:
    try:
        return store.get(task_id)
    except KeyError:
        raise HTTPException(status_code=404, detail="任务不存在") from None


@app.get("/api/tasks/{task_id}/download/{kind}")
def download(task_id: str, kind: str) -> FileResponse:
    try:
        task = store.get(task_id)
    except KeyError:
        raise HTTPException(status_code=404, detail="任务不存在") from None

    path_key = {
        "ppt": "ppt_path",
        "script": "script_path",
        "video": "video_path",
        "json": "json_path",
        "subtitle": "subtitle_path",
        "log": "log_path",
        "metadata": "metadata_path",
    }.get(kind)
    if not path_key:
        raise HTTPException(status_code=400, detail="不支持的下载类型，仅支持 ppt/script/video/json/subtitle/log/metadata")

    path_value = task.get(path_key)
    if not path_value:
        raise HTTPException(status_code=404, detail="产物尚未生成")

    file_path = Path(path_value)
    if not file_path.exists():
        raise HTTPException(status_code=404, detail="产物文件不存在")

    return FileResponse(
        str(file_path),
        filename=file_path.name,
        media_type="application/octet-stream",
    )


def _deep_merge(base: dict[str, Any], extra: dict[str, Any]) -> None:
    for key, value in extra.items():
        if isinstance(value, dict) and isinstance(base.get(key), dict):
            _deep_merge(base[key], value)
        else:
            base[key] = value
