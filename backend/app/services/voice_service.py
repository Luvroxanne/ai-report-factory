from __future__ import annotations

import json
import math
import platform
import subprocess
import wave
from pathlib import Path
from typing import Any

from app.adapters.cosyvoice_client import CosyVoiceClient


class VoiceService:
    def __init__(self, cosyvoice: CosyVoiceClient):
        self.cosyvoice = cosyvoice

    def generate_pack(self, task_id: str, scripts: list[dict[str, Any]], output_dir: Path, logger: Any | None = None) -> list[dict[str, Any]]:
        task_dir = output_dir / "audio"
        task_dir.mkdir(parents=True, exist_ok=True)

        manifest: list[dict[str, Any]] = []
        for item in scripts:
            page = int(item["page"])
            output_path = task_dir / f"page_{page:02d}.wav"
            text = str(item["text"])
            engine = "silent"

            try:
                generated = self.cosyvoice.synthesize(text, output_path)
                if generated and self._valid_wav(generated):
                    engine = "cosyvoice"
                    if logger:
                        logger.info("CosyVoice 语音生成完成", page=page, path=generated)
                    manifest.append(self._manifest_item(page, generated, engine))
                    continue
            except Exception as exc:
                if logger:
                    logger.warning("CosyVoice 不可用，切换 Windows TTS", page=page, reason=exc)

            if self._make_windows_tts_wav(text, output_path):
                engine = "windows_tts"
                if logger:
                    logger.info("Windows TTS 语音生成完成", page=page, path=output_path)
                manifest.append(self._manifest_item(page, output_path, engine))
                continue

            seconds = float(item.get("estimated_seconds") or max(4.0, min(12.0, len(text) / 18.0)))
            self._make_silent_wav(output_path, seconds=seconds)
            if logger:
                logger.warning("语音生成失败，使用静音兜底", page=page, seconds=seconds)
            manifest.append(self._manifest_item(page, output_path, engine))

        manifest_path = output_dir / "audio_manifest.json"
        manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
        return manifest

    def _manifest_item(self, page: int, path: Path, engine: str) -> dict[str, Any]:
        return {
            "page": page,
            "path": str(path),
            "duration": round(self._audio_duration(path), 3),
            "engine": engine,
        }

    def _make_windows_tts_wav(self, text: str, output_path: Path) -> bool:
        if platform.system().lower() != "windows":
            return False

        work_dir = output_path.parent
        work_dir.mkdir(parents=True, exist_ok=True)
        text_path = work_dir / f"{output_path.stem}.txt"
        script_path = work_dir / f"{output_path.stem}_tts.ps1"
        text_path.write_text(text, encoding="utf-8-sig")

        ps_script = f"""
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$text = [System.IO.File]::ReadAllText('{self._ps_escape(text_path)}', [System.Text.Encoding]::UTF8)
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$voice = $synth.GetInstalledVoices() |
  Where-Object {{ $_.VoiceInfo.Culture.Name -like 'zh-*' }} |
  Select-Object -First 1
if ($voice) {{
  $synth.SelectVoice($voice.VoiceInfo.Name)
}}
$synth.Rate = 0
$synth.Volume = 95
$synth.SetOutputToWaveFile('{self._ps_escape(output_path)}')
$synth.Speak($text)
$synth.Dispose()
"""
        script_path.write_text(ps_script, encoding="utf-8-sig")

        try:
            subprocess.run(
                ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script_path)],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=90,
            )
            return self._valid_wav(output_path)
        except Exception:
            return False

    def _ps_escape(self, path: Path) -> str:
        return str(path).replace("'", "''")

    def _valid_wav(self, path: Path) -> bool:
        if not path.exists() or path.stat().st_size <= 1024:
            return False
        try:
            with wave.open(str(path), "rb") as wf:
                return wf.getnframes() > 0 and wf.getframerate() > 0
        except Exception:
            return False

    def _make_silent_wav(self, path: Path, seconds: float, sample_rate: int = 16000) -> None:
        total_frames = int(math.ceil(seconds * sample_rate))
        path.parent.mkdir(parents=True, exist_ok=True)
        with wave.open(str(path), "wb") as wf:
            wf.setnchannels(1)
            wf.setsampwidth(2)
            wf.setframerate(sample_rate)
            wf.writeframes(b"\x00\x00" * total_frames)

    def _audio_duration(self, audio_path: Path) -> float:
        with wave.open(str(audio_path), "rb") as wf:
            frames = wf.getnframes()
            rate = wf.getframerate()
            return max(1.0, frames / float(rate))
