from __future__ import annotations

import shutil
import subprocess
from pathlib import Path
from typing import Any
from uuid import uuid4


def task_safe_suffix() -> str:
    return uuid4().hex[:12]


class VideoService:
    WIDTH = 1920
    HEIGHT = 1080

    def compose(
        self,
        *,
        task_id: str,
        plan: dict[str, Any],
        scripts: list[dict[str, Any]],
        audio_files: list[dict[str, Any]],
        output_dir: Path,
        frames_dir: Path,
        video_segments: list[dict[str, Any]] | None = None,
        logger: Any | None = None,
        progress_callback: Any | None = None,
    ) -> dict[str, Any]:
        output_path = output_dir / "final_1080p.mp4"
        frames_dir.mkdir(parents=True, exist_ok=True)

        if progress_callback:
            progress_callback("正在渲染 1080P 页面帧", 88)
        frames = self._render_frames(plan, scripts, frames_dir)
        if progress_callback:
            progress_callback("正在合成 1080P 视频片段", 92)
        segments = video_segments or []
        if segments:
            self._compose_with_moviepy(frames, audio_files, output_path, segments)
        else:
            self._compose_with_ffmpeg(frames, audio_files, output_path, logger=logger, progress_callback=progress_callback)
        subtitle_path = output_dir / "subtitles.srt"
        self._save_subtitle(scripts, audio_files, subtitle_path)
        if logger:
            logger.info("1080P 视频合成完成", path=output_path, subtitle=subtitle_path)
        return {"video_path": output_path, "subtitle_path": subtitle_path, "frames": frames}

    def _render_frames(self, plan: dict[str, Any], scripts: list[dict[str, Any]], frames_dir: Path) -> list[Path]:
        from PIL import Image, ImageDraw

        frames: list[Path] = []
        fonts = {
            "hero": self._font(78),
            "title": self._font(64),
            "subtitle": self._font(34),
            "bullet": self._font(37),
            "caption": self._font(27),
            "small": self._font(23),
            "number": self._font(32),
        }
        theme = self._theme(plan.get("style", "official-tech"))
        slides = plan.get("slides", []) or []
        for index, slide in enumerate(slides, start=1):
            img = self._base_frame(theme, index)
            draw = ImageDraw.Draw(img, "RGBA")
            layout = slide.get("layout") or "content"
            script = scripts[index - 1] if index - 1 < len(scripts) else {}
            if layout == "cover" or index == 1:
                self._draw_cover(draw, plan, slide, script, fonts, theme)
            elif layout == "agenda":
                self._draw_agenda(draw, slide, script, fonts, theme, index, len(slides))
            elif layout == "section":
                self._draw_section(draw, slide, script, fonts, theme, index, len(slides))
            elif layout == "summary" or index == len(slides):
                self._draw_summary(draw, slide, script, fonts, theme, index, len(slides))
            else:
                self._draw_content(draw, slide, script, fonts, theme, index, len(slides))

            self._draw_subtitle_bar(draw, script, fonts, theme)
            frame_path = frames_dir / f"frame_{index:02d}.png"
            img.save(frame_path, quality=95)
            frames.append(frame_path)
        return frames

    def _theme(self, style: str) -> dict[str, tuple[int, int, int]]:
        if style == "training":
            return {"bg1": (249, 247, 242), "bg2": (230, 222, 207), "panel": (255, 255, 255), "text": (34, 42, 55), "muted": (91, 105, 126), "accent": (205, 126, 43), "accent2": (40, 118, 152), "line": (226, 218, 204)}
        if style == "roadshow":
            return {"bg1": (13, 12, 26), "bg2": (42, 29, 48), "panel": (29, 26, 48), "text": (252, 247, 232), "muted": (190, 181, 163), "accent": (242, 194, 88), "accent2": (232, 85, 74), "line": (89, 72, 112)}
        return {"bg1": (4, 14, 30), "bg2": (9, 32, 62), "panel": (10, 36, 70), "text": (244, 248, 255), "muted": (159, 184, 216), "accent": (47, 214, 188), "accent2": (47, 140, 255), "line": (73, 148, 235)}

    def _base_frame(self, theme: dict[str, tuple[int, int, int]], index: int):
        from PIL import Image, ImageDraw, ImageFilter

        width, height = self.WIDTH, self.HEIGHT
        b1 = theme["bg1"]
        b2 = theme["bg2"]
        gradient = Image.new("RGB", (1, height), theme["bg1"])
        px = gradient.load()
        for y in range(height):
            ratio = y / height
            r = int(b1[0] * (1 - ratio) + b2[0] * ratio)
            g = int(b1[1] * (1 - ratio) + b2[1] * ratio)
            b = int(b1[2] * (1 - ratio) + b2[2] * ratio)
            px[0, y] = (r, g, b)
        img = gradient.resize((width, height))
        glow = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        glow_draw = ImageDraw.Draw(glow, "RGBA")
        glow_draw.ellipse((1260, -160, 2060, 620), fill=theme["accent2"] + (42,))
        glow = glow.filter(ImageFilter.GaussianBlur(70))
        img = Image.alpha_composite(img.convert("RGBA"), glow).convert("RGB")
        draw = ImageDraw.Draw(img, "RGBA")
        draw.rectangle((0, 0, 34, height), fill=theme["accent2"] + (255,))
        draw.polygon([(1305, 68), (1900, 68), (1795, 330), (1200, 330)], fill=theme["panel"] + (155,))
        draw.ellipse((1490, 690, 1935, 1135), outline=theme["accent"] + (155,), width=7)
        draw.ellipse((1530, 730, 1895, 1095), outline=theme["accent2"] + (110,), width=3)
        draw.line((120, 930, 1780, 930), fill=theme["line"] + (155,), width=3)
        for x in range(160, 1800, 160):
            draw.line((x, 940, x + 70, 940), fill=theme["accent"] + (150,), width=3)
        return img

    def _draw_cover(self, draw: Any, plan: dict[str, Any], slide: dict[str, Any], script: dict[str, Any], fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]]) -> None:
        title = plan.get("title") or slide.get("title") or "AI报告"
        subtitle = plan.get("subtitle") or "PPT · 解说稿 · 语音 · 1080P 视频一体生成"
        summary = plan.get("summary") or "无 Token 也可完整跑通；配置 AI Provider 后可提升内容质量。"
        draw.text((135, 115), "AI REPORT FACTORY", font=fonts["small"], fill=theme["accent"])
        draw.text((135, 185), title, font=fonts["hero"], fill=theme["text"])
        draw.text((142, 315), subtitle, font=fonts["subtitle"], fill=theme["accent"])
        draw.rounded_rectangle((135, 430, 1250, 585), radius=28, fill=theme["panel"] + (215,), outline=theme["line"] + (120,), width=2)
        for line_no, line in enumerate(self._wrap_text(summary, 38, 2)):
            draw.text((176, 466 + line_no * 46), line, font=fonts["small"], fill=theme["muted"])
        tags = ["Presenton", "CosyVoice", "Wan2.2", "Windows TTS", "H.264"]
        for i, tag in enumerate(tags):
            x = 142 + i * 196
            draw.rounded_rectangle((x, 720, x + 162, 774), radius=22, fill=theme["accent"] + (230,))
            draw.text((x + 24, 735), tag, font=fonts["small"], fill=theme["bg1"])
        draw.text((1360, 835), "DELIVERABLE", font=fonts["subtitle"], fill=theme["accent"])
        draw.text((1360, 890), "DESKTOP APP", font=fonts["subtitle"], fill=theme["text"])

    def _draw_agenda(self, draw: Any, slide: dict[str, Any], script: dict[str, Any], fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]], index: int, total: int) -> None:
        self._draw_header(draw, slide.get("title", "目录"), fonts, theme, index, total)
        for i, item in enumerate(slide.get("bullets", [])[:5], start=1):
            y = 238 + (i - 1) * 122
            draw.rounded_rectangle((150, y, 1630, y + 82), radius=28, fill=theme["panel"] + (220,), outline=theme["line"] + (110,), width=2)
            draw.text((195, y + 20), f"{i:02d}", font=fonts["number"], fill=theme["accent"])
            draw.text((310, y + 21), str(item), font=fonts["subtitle"], fill=theme["text"])

    def _draw_section(self, draw: Any, slide: dict[str, Any], script: dict[str, Any], fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]], index: int, total: int) -> None:
        chapter = slide.get("chapter") or f"第 {index} 部分"
        title = slide.get("title", "章节")
        draw.text((135, 170), str(chapter).upper(), font=fonts["subtitle"], fill=theme["accent"])
        draw.text((135, 260), str(title), font=fonts["hero"], fill=theme["text"])
        draw.rounded_rectangle((135, 460, 1370, 620), radius=30, fill=theme["panel"] + (220,), outline=theme["line"] + (110,), width=2)
        text = " / ".join(str(x) for x in slide.get("bullets", [])[:3]) or "聚焦关键目标、核心内容与落地路径"
        for line_no, line in enumerate(self._wrap_text(text, 36, 2)):
            draw.text((178, 498 + line_no * 46), line, font=fonts["subtitle"], fill=theme["muted"])
        self._draw_page_no(draw, fonts, theme, index, total)

    def _draw_content(self, draw: Any, slide: dict[str, Any], script: dict[str, Any], fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]], index: int, total: int) -> None:
        self._draw_header(draw, slide.get("title", "内容页"), fonts, theme, index, total)
        bullets = [str(item) for item in slide.get("bullets", [])[:6]] or ["明确目标", "梳理路径", "形成闭环"]
        for i, bullet in enumerate(bullets):
            col = i % 2
            row = i // 2
            x = 130 + col * 825
            y = 252 + row * 155
            draw.rounded_rectangle((x, y, x + 760, y + 118), radius=26, fill=theme["panel"] + (222,), outline=theme["line"] + (95,), width=2)
            draw.ellipse((x + 28, y + 32, x + 82, y + 86), fill=theme["accent"] + (235,))
            draw.text((x + 44, y + 39), str(i + 1), font=fonts["small"], fill=theme["bg1"])
            for line_no, line in enumerate(self._wrap_text(bullet, 25, 2)):
                draw.text((x + 112, y + 25 + line_no * 42), line, font=fonts["bullet"], fill=theme["text"])

    def _draw_summary(self, draw: Any, slide: dict[str, Any], script: dict[str, Any], fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]], index: int, total: int) -> None:
        self._draw_header(draw, slide.get("title", "总结与下一步"), fonts, theme, index, total)
        bullets = [str(item) for item in slide.get("bullets", [])[:4]]
        for i, bullet in enumerate(bullets):
            y = 250 + i * 132
            draw.rounded_rectangle((150, y, 1520, y + 88), radius=30, fill=theme["panel"] + (222,), outline=theme["line"] + (100,), width=2)
            draw.text((195, y + 22), "✓", font=fonts["subtitle"], fill=theme["accent"])
            draw.text((270, y + 24), bullet, font=fonts["subtitle"], fill=theme["text"])
        draw.text((1355, 835), "THANK YOU", font=fonts["subtitle"], fill=theme["accent"])

    def _draw_header(self, draw: Any, title: str, fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]], index: int, total: int) -> None:
        draw.text((120, 86), str(title), font=fonts["title"], fill=theme["text"])
        self._draw_page_no(draw, fonts, theme, index, total)

    def _draw_page_no(self, draw: Any, fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]], index: int, total: int) -> None:
        draw.text((1620, 110), f"{index:02d} / {total:02d}", font=fonts["number"], fill=theme["accent"])
        draw.text((120, 996), "AI报告工厂 · PPT / 讲稿 / 语音 / 视频 统一归档", font=fonts["small"], fill=theme["muted"])

    def _draw_subtitle_bar(self, draw: Any, script: dict[str, Any], fonts: dict[str, Any], theme: dict[str, tuple[int, int, int]]) -> None:
        note = str(script.get("text") or "")
        if not note:
            return
        draw.rounded_rectangle((180, 822, 1740, 934), radius=32, fill=(0, 0, 0, 155), outline=theme["accent"] + (90,), width=2)
        for line_no, line in enumerate(self._wrap_text(note, 42, 2)):
            draw.text((225, 846 + line_no * 36), line, font=fonts["caption"], fill=(245, 248, 255))

    def _wrap_text(self, text: str, max_chars: int, max_lines: int) -> list[str]:
        text = " ".join(str(text).replace("\n", " ").split())
        if not text:
            return []
        lines: list[str] = []
        current = ""
        for ch in text:
            current += ch
            if len(current) >= max_chars or ch in "。；？！;?!":
                lines.append(current.strip())
                current = ""
                if len(lines) >= max_lines:
                    break
        if current and len(lines) < max_lines:
            lines.append(current.strip())
        if len(lines) == max_lines and len("".join(lines)) < len(text):
            lines[-1] = lines[-1].rstrip("。；？！;?! ") + "…"
        return lines

    def _font(self, size: int):
        from PIL import ImageFont

        candidates = [
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/simhei.ttf",
            "C:/Windows/Fonts/Deng.ttf",
        ]
        for candidate in candidates:
            if Path(candidate).exists():
                return ImageFont.truetype(candidate, size=size)
        return ImageFont.load_default()

    def _compose_with_moviepy(self, frames: list[Path], audio_manifest: list[dict[str, Any]], output_path: Path, video_segments: list[dict[str, Any]]) -> None:
        try:
            from moviepy.editor import AudioFileClip, ImageClip, VideoFileClip, concatenate_videoclips
        except Exception:
            from moviepy import AudioFileClip, ImageClip, VideoFileClip, concatenate_videoclips

        segment_by_page = {int(item.get("page") or 0): Path(str(item.get("path"))) for item in video_segments if item.get("path")}
        clips = []
        audio_clips = []
        media_clips = []
        for page, (frame, audio_info) in enumerate(zip(frames, audio_manifest), start=1):
            audio_path = Path(str(audio_info["path"]))
            duration = float(audio_info.get("duration") or 3.0)
            audio_clip = AudioFileClip(str(audio_path))
            segment = segment_by_page.get(page)
            if segment and segment.exists() and segment.suffix.lower() in {".mp4", ".mov", ".webm", ".avi", ".mkv"}:
                media_clip = self._fit_video_clip(VideoFileClip(str(segment)))
            else:
                media_clip = ImageClip(str(frame))
                media_clip = self._apply_subtle_motion(media_clip, duration)
            media_clip = self._with_duration(media_clip, duration)
            media_clip = self._with_audio(media_clip, audio_clip)
            clips.append(media_clip)
            audio_clips.append(audio_clip)
            media_clips.append(media_clip)

        final = None
        temp_audiofile = output_path.parent / f"{output_path.stem}_{task_safe_suffix()}_audio.m4a"
        try:
            final = concatenate_videoclips(clips, method="compose")
            output_path.parent.mkdir(parents=True, exist_ok=True)
            final.write_videofile(
                str(output_path),
                fps=24,
                codec="libx264",
                audio_codec="aac",
                preset="medium",
                temp_audiofile=str(temp_audiofile),
                remove_temp=False,
                ffmpeg_params=["-pix_fmt", "yuv420p", "-movflags", "+faststart"],
                logger=None,
            )
        finally:
            if final is not None:
                final.close()
            for clip in clips:
                clip.close()
            for audio_clip in audio_clips:
                audio_clip.close()
            try:
                if temp_audiofile.exists():
                    temp_audiofile.unlink()
            except OSError:
                pass

    def _compose_with_ffmpeg(
        self,
        frames: list[Path],
        audio_manifest: list[dict[str, Any]],
        output_path: Path,
        *,
        logger: Any | None = None,
        progress_callback: Any | None = None,
    ) -> None:
        ffmpeg = self._ffmpeg_exe()
        output_path.parent.mkdir(parents=True, exist_ok=True)
        segment_dir = output_path.parent / "local_video_segments"
        segment_dir.mkdir(parents=True, exist_ok=True)
        segments: list[Path] = []
        pairs = list(zip(frames, audio_manifest))
        total = max(1, len(pairs))

        for index, (frame, audio_info) in enumerate(pairs, start=1):
            audio_path = Path(str(audio_info["path"]))
            duration = max(0.8, float(audio_info.get("duration") or 3.0))
            segment_path = segment_dir / f"segment_{index:02d}.mp4"
            self._run_ffmpeg(
                [
                    ffmpeg,
                    "-y",
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-loop",
                    "1",
                    "-framerate",
                    "24",
                    "-i",
                    str(frame),
                    "-i",
                    str(audio_path),
                    "-t",
                    f"{duration:.3f}",
                    "-vf",
                    f"scale={self.WIDTH}:{self.HEIGHT}:flags=lanczos,format=yuv420p",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-tune",
                    "stillimage",
                    "-r",
                    "24",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "128k",
                    "-shortest",
                    str(segment_path),
                ]
            )
            segments.append(segment_path)
            if progress_callback:
                progress_callback(f"正在合成视频片段 {index}/{total}", 92 + int(index * 5 / total))

        concat_file = segment_dir / "concat.txt"
        concat_file.write_text(
            "\n".join(f"file '{self._ffmpeg_concat_path(path)}'" for path in segments),
            encoding="utf-8",
        )
        self._run_ffmpeg(
            [
                ffmpeg,
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                str(concat_file),
                "-c",
                "copy",
                "-movflags",
                "+faststart",
                str(output_path),
            ]
        )
        if logger:
            logger.info("本地 ffmpeg 快速合成完成", segments=len(segments), output=output_path)

    def _ffmpeg_exe(self) -> str:
        try:
            import imageio_ffmpeg

            exe = imageio_ffmpeg.get_ffmpeg_exe()
            if exe:
                return exe
        except Exception:
            pass
        exe = shutil.which("ffmpeg")
        if exe:
            return exe
        raise RuntimeError("未找到 ffmpeg，无法合成视频")

    def _run_ffmpeg(self, cmd: list[str]) -> None:
        result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="ignore")
        if result.returncode != 0:
            detail = (result.stderr or result.stdout or "").strip()
            raise RuntimeError(f"ffmpeg 执行失败：{detail[-800:]}")

    def _ffmpeg_concat_path(self, path: Path) -> str:
        return str(path.resolve()).replace("\\", "/").replace("'", "'\\''")

    def _fit_video_clip(self, clip: Any) -> Any:
        try:
            if hasattr(clip, "resize"):
                return clip.resize((self.WIDTH, self.HEIGHT))
            if hasattr(clip, "resized"):
                return clip.resized((self.WIDTH, self.HEIGHT))
        except Exception:
            return clip
        return clip

    def _apply_subtle_motion(self, clip: Any, duration: float) -> Any:
        try:
            if hasattr(clip, "fadein"):
                clip = clip.fadein(min(0.35, duration / 4)).fadeout(min(0.28, duration / 5))
        except Exception:
            return clip
        return clip

    def _with_duration(self, clip: Any, duration: float) -> Any:
        if hasattr(clip, "with_duration"):
            return clip.with_duration(duration)
        return clip.set_duration(duration)

    def _with_audio(self, clip: Any, audio: Any) -> Any:
        if hasattr(clip, "with_audio"):
            return clip.with_audio(audio)
        return clip.set_audio(audio)

    def _save_subtitle(self, scripts: list[dict[str, Any]], audio_manifest: list[dict[str, Any]], output_path: Path) -> None:
        lines: list[str] = []
        cursor = 0.0
        for idx, (script, audio) in enumerate(zip(scripts, audio_manifest), start=1):
            duration = float(audio.get("duration") or 3.0)
            start = cursor
            end = cursor + duration
            lines.extend(
                [
                    str(idx),
                    f"{self._srt_time(start)} --> {self._srt_time(end)}",
                    str(script.get("text") or ""),
                    "",
                ]
            )
            cursor = end
        output_path.write_text("\n".join(lines), encoding="utf-8")

    def _srt_time(self, seconds: float) -> str:
        millis = int((seconds - int(seconds)) * 1000)
        total = int(seconds)
        h = total // 3600
        m = (total % 3600) // 60
        s = total % 60
        return f"{h:02d}:{m:02d}:{s:02d},{millis:03d}"
