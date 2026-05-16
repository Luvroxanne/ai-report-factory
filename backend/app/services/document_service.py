from __future__ import annotations

import re
from pathlib import Path


class DocumentService:
    supported_suffixes = {".md", ".txt"}

    def read_text(self, path: Path) -> str:
        suffix = path.suffix.lower()
        if suffix not in self.supported_suffixes:
            raise ValueError(f"仅支持 Markdown/TXT 文件，当前后缀：{suffix}")

        raw = path.read_bytes()
        for encoding in ("utf-8-sig", "utf-8", "gb18030", "gbk"):
            try:
                text = raw.decode(encoding)
                break
            except UnicodeDecodeError:
                continue
        else:
            text = raw.decode("utf-8", errors="ignore")

        text = text.replace("\r\n", "\n").replace("\r", "\n")
        text = re.sub(r"\n{3,}", "\n\n", text)
        return text.strip()

    def compact(self, text: str, max_chars: int = 16000) -> str:
        text = re.sub(r"[ \t]+", " ", text)
        text = re.sub(r"\n{3,}", "\n\n", text)
        return text[:max_chars]
