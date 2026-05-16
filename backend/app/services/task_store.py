from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path
from typing import Any


def _now() -> str:
    return datetime.now().isoformat(timespec="seconds")


class TaskStore:
    def __init__(self, db_path: Path):
        self.db_path = db_path
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._init_schema()

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        return conn

    def _init_schema(self) -> None:
        with self._connect() as conn:
            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    original_filename TEXT NOT NULL,
                    input_path TEXT NOT NULL,
                    status TEXT NOT NULL,
                    current_step TEXT NOT NULL DEFAULT '',
                    progress INTEGER NOT NULL DEFAULT 0,
                    project_dir TEXT,
                    ppt_path TEXT,
                    script_path TEXT,
                    video_path TEXT,
                    json_path TEXT,
                    audio_dir TEXT,
                    subtitle_path TEXT,
                    log_path TEXT,
                    metadata_path TEXT,
                    error TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )
                """
            )
            columns = {row[1] for row in conn.execute("PRAGMA table_info(tasks)").fetchall()}
            migrations = {
                "project_dir": "ALTER TABLE tasks ADD COLUMN project_dir TEXT",
                "json_path": "ALTER TABLE tasks ADD COLUMN json_path TEXT",
                "audio_dir": "ALTER TABLE tasks ADD COLUMN audio_dir TEXT",
                "subtitle_path": "ALTER TABLE tasks ADD COLUMN subtitle_path TEXT",
                "log_path": "ALTER TABLE tasks ADD COLUMN log_path TEXT",
                "metadata_path": "ALTER TABLE tasks ADD COLUMN metadata_path TEXT",
            }
            for column, sql in migrations.items():
                if column not in columns:
                    conn.execute(sql)

    def create(self, task_id: str, original_filename: str, input_path: Path) -> dict[str, Any]:
        now = _now()
        with self._connect() as conn:
            conn.execute(
                """
                INSERT INTO tasks (
                    id, original_filename, input_path, status,
                    current_step, progress, created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    task_id,
                    original_filename,
                    str(input_path),
                    "pending",
                    "等待中",
                    0,
                    now,
                    now,
                ),
            )
        return self.get(task_id)

    def update(self, task_id: str, **values: Any) -> dict[str, Any]:
        if not values:
            return self.get(task_id)

        values["updated_at"] = _now()
        keys = list(values.keys())
        sql = ", ".join(f"{key}=?" for key in keys)
        params = [values[key] for key in keys] + [task_id]
        with self._connect() as conn:
            conn.execute(f"UPDATE tasks SET {sql} WHERE id=?", params)
        return self.get(task_id)

    def get(self, task_id: str) -> dict[str, Any]:
        with self._connect() as conn:
            row = conn.execute("SELECT * FROM tasks WHERE id=?", (task_id,)).fetchone()
        if row is None:
            raise KeyError(task_id)
        return dict(row)

    def list_recent(self, limit: int = 20) -> list[dict[str, Any]]:
        with self._connect() as conn:
            rows = conn.execute(
                "SELECT * FROM tasks ORDER BY created_at DESC LIMIT ?",
                (limit,),
            ).fetchall()
        return [dict(row) for row in rows]
