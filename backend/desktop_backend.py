from __future__ import annotations

import os

import uvicorn

from app.main import app as fastapi_app


def main() -> None:
    os.environ.setdefault("REPORT_STYLE", "official-tech")
    uvicorn.run(fastapi_app, host="127.0.0.1", port=8000, log_level="warning")


if __name__ == "__main__":
    main()
