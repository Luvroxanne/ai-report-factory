from __future__ import annotations

import importlib.util
import platform
import shutil
import subprocess
from typing import Any


class DependencyService:
    def check(self) -> list[dict[str, Any]]:
        return [
            self._python_package("python-pptx", "pptx"),
            self._python_package("python-docx", "docx"),
            self._python_package("Pillow", "PIL"),
            self._python_package("MoviePy", "moviepy"),
            self._binary("ffmpeg", "视频合成/转码"),
            self._windows_tts(),
        ]

    def _python_package(self, name: str, module: str) -> dict[str, Any]:
        ok = importlib.util.find_spec(module) is not None
        return {"name": name, "ok": ok, "detail": "可用" if ok else "缺失"}

    def _binary(self, name: str, detail: str) -> dict[str, Any]:
        path = shutil.which(name)
        if path:
            return {"name": name, "ok": True, "detail": path}
        if name == "ffmpeg":
            try:
                import imageio_ffmpeg

                exe = imageio_ffmpeg.get_ffmpeg_exe()
                if exe:
                    return {"name": name, "ok": True, "detail": f"imageio-ffmpeg: {exe}"}
            except Exception:
                pass
        return {"name": name, "ok": False, "detail": f"缺少{detail}"}

    def _windows_tts(self) -> dict[str, Any]:
        if platform.system().lower() != "windows":
            return {"name": "Windows TTS", "ok": False, "detail": "仅支持 Windows 环境"}
        powershell = shutil.which("powershell") or shutil.which("powershell.exe")
        if not powershell:
            return {"name": "Windows TTS", "ok": False, "detail": "缺少 powershell"}
        try:
            result = subprocess.run(
                [powershell, "-NoProfile", "-Command", "Add-Type -AssemblyName System.Speech; 'ok'"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
            )
            ok = result.returncode == 0 and "ok" in result.stdout
            return {"name": "Windows TTS", "ok": ok, "detail": "可用" if ok else result.stderr[:160]}
        except Exception as exc:
            return {"name": "Windows TTS", "ok": False, "detail": str(exc)}
