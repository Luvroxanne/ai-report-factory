from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass
from typing import Any

import requests


@dataclass
class LLMClient:
    timeout: int = 90
    retries: int = 2

    def build_report_plan(self, text: str, style: str, config: dict[str, Any] | None = None, logger: Any | None = None) -> dict[str, Any]:
        config = config or {}
        ai_config = config.get("ai", {}) if isinstance(config, dict) else {}
        provider = str(ai_config.get("active_provider") or "ollama").lower()
        self.timeout = int(ai_config.get("timeout_seconds") or self.timeout)
        self.retries = int(ai_config.get("retries") or self.retries)
        if provider != "local":
            try:
                if provider == "openai":
                    plan = self._build_with_openai(text, style, ai_config)
                    return self._mark_generation(plan, provider, False)
                if provider == "gemini":
                    plan = self._build_with_gemini(text, style, ai_config)
                    return self._mark_generation(plan, provider, False)
                if provider == "ollama":
                    plan = self._build_with_ollama(text, style, ai_config)
                    return self._mark_generation(plan, provider, False)
            except Exception as exc:
                if logger:
                    logger.warning("AI 大纲生成失败，切换本地规则兜底", provider=provider, reason=exc)
        plan = self._build_with_rules(text, style)
        return self._mark_generation(plan, "local", provider != "local", fallback_from=provider if provider != "local" else "")

    def test_provider(self, provider: str, config: dict[str, Any]) -> tuple[bool, str]:
        ai_config = config.get("ai", {}) if isinstance(config, dict) else {}
        providers = ai_config.get("providers", {})
        item = providers.get(provider, {}) if isinstance(providers, dict) else {}
        try:
            if provider == "openai":
                api_key = str(item.get("api_key") or "")
                base_url = self._trim(str(item.get("base_url") or "https://api.openai.com/v1"))
                model = str(item.get("model") or "gpt-4o-mini")
                if not api_key:
                    return False, "OpenAI 兼容 Provider 未配置 API Token"
                payload = {
                    "model": model,
                    "messages": [{"role": "user", "content": "请只回复 OK"}],
                    "max_tokens": 8,
                }
                self._post_openai_chat(f"{base_url}/chat/completions", payload, {"Authorization": f"Bearer {api_key}"}, timeout=20)
                return True, "Token 可用，chat/completions 可调用"
            if provider == "gemini":
                api_key = str(item.get("api_key") or "")
                base_url = self._trim(str(item.get("base_url") or "https://generativelanguage.googleapis.com"))
                if not api_key:
                    return False, "Gemini 未配置 API Token"
                response = requests.get(f"{base_url}/v1beta/models?key={api_key}", timeout=15)
                return response.ok, "Token 可用" if response.ok else f"接口返回 {response.status_code}"
            if provider == "ollama":
                base_url = self._trim(str(item.get("base_url") or ""))
                if not base_url:
                    return False, "Ollama 未配置服务地址"
                response = requests.get(f"{base_url}/api/tags", timeout=8)
                return response.ok, "Ollama 服务可用" if response.ok else f"接口返回 {response.status_code}"
            return True, "本地规则引擎无需 Token"
        except Exception as exc:
            return False, str(exc)

    def _build_with_openai(self, text: str, style: str, ai_config: dict[str, Any]) -> dict[str, Any]:
        item = ai_config.get("providers", {}).get("openai", {})
        api_key = str(item.get("api_key") or "")
        if not api_key:
            raise RuntimeError("OpenAI compatible API token is empty")
        base_url = self._trim(str(item.get("base_url") or "https://api.openai.com/v1"))
        model = str(item.get("model") or "gpt-4o-mini")
        payload = {
            "model": model,
            "messages": [
                {"role": "system", "content": "你是资深商业汇报策划师，只输出严格 JSON。"},
                {"role": "user", "content": self._prompt(text, style)},
            ],
            "temperature": 0.45,
            "response_format": {"type": "json_object"},
        }
        response = self._post_openai_chat(f"{base_url}/chat/completions", payload, {"Authorization": f"Bearer {api_key}"})
        content = response.get("choices", [{}])[0].get("message", {}).get("content", "")
        return self._normalize(json.loads(self._extract_json(content)), style)

    def _build_with_gemini(self, text: str, style: str, ai_config: dict[str, Any]) -> dict[str, Any]:
        item = ai_config.get("providers", {}).get("gemini", {})
        api_key = str(item.get("api_key") or "")
        if not api_key:
            raise RuntimeError("Gemini API token is empty")
        base_url = self._trim(str(item.get("base_url") or "https://generativelanguage.googleapis.com"))
        model = str(item.get("model") or "gemini-1.5-flash")
        payload = {"contents": [{"parts": [{"text": self._prompt(text, style)}]}], "generationConfig": {"response_mime_type": "application/json", "temperature": 0.45}}
        response = self._post_json(f"{base_url}/v1beta/models/{model}:generateContent?key={api_key}", payload)
        content = response.get("candidates", [{}])[0].get("content", {}).get("parts", [{}])[0].get("text", "")
        return self._normalize(json.loads(self._extract_json(content)), style)

    def _build_with_ollama(self, text: str, style: str, ai_config: dict[str, Any]) -> dict[str, Any]:
        item = ai_config.get("providers", {}).get("ollama", {})
        base_url = self._trim(str(item.get("base_url") or ""))
        if not base_url:
            raise RuntimeError("Ollama base url is empty")
        model = str(item.get("model") or "qwen2.5:7b")
        response = self._post_json(f"{base_url}/api/generate", {"model": model, "prompt": self._prompt(text, style), "format": "json", "stream": False})
        return self._normalize(json.loads(self._extract_json(response.get("response", ""))), style)

    def _post_openai_chat(self, url: str, payload: dict[str, Any], headers: dict[str, str], timeout: int | None = None) -> dict[str, Any]:
        attempts: list[dict[str, Any]] = []
        attempts.append(dict(payload))
        without_response_format = dict(payload)
        without_response_format.pop("response_format", None)
        attempts.append(without_response_format)
        minimal = dict(without_response_format)
        minimal.pop("temperature", None)
        minimal.pop("max_tokens", None)
        attempts.append(minimal)

        errors: list[str] = []
        for item in attempts:
            try:
                return self._post_json(url, item, headers=headers, timeout=timeout)
            except Exception as exc:
                errors.append(str(exc))
        raise RuntimeError("OpenAI 兼容接口调用失败：" + " | ".join(errors[-3:]))

    def _post_json(self, url: str, payload: dict[str, Any], headers: dict[str, str] | None = None, timeout: int | None = None) -> dict[str, Any]:
        last_exc: Exception | None = None
        for attempt in range(max(1, self.retries + 1)):
            try:
                response = requests.post(url, json=payload, headers=headers, timeout=timeout or self.timeout)
                if not response.ok:
                    raise RuntimeError(f"HTTP {response.status_code}: {response.text[:800]}")
                try:
                    return response.json()
                except ValueError as exc:
                    raise RuntimeError(f"接口未返回 JSON：{response.text[:300]}") from exc
            except Exception as exc:
                last_exc = exc
                if attempt < self.retries:
                    time.sleep(0.8 * (attempt + 1))
        raise RuntimeError(str(last_exc) if last_exc else "AI request failed")

    def _mark_generation(self, plan: dict[str, Any], provider: str, fallback: bool, fallback_from: str = "") -> dict[str, Any]:
        plan["generation"] = {
            "provider": provider,
            "fallback": fallback,
            "fallback_from": fallback_from,
        }
        return plan

    def _prompt(self, text: str, style: str) -> str:
        return f"""
请把以下材料升级为可直接生成正式商业汇报 PPT、解说稿和视频分镜的 JSON。
要求：
1. 输出 8-10 页，必须包含 cover、agenda、section、content、summary 五类版式。
2. 每页包含 title、bullets、speaker_note、visual_prompt、layout、chapter、estimated_seconds。
3. bullets 用短句；speaker_note 要自然口播；visual_prompt 面向 Wan2.2/视频生成。
4. 风格：{style}。语言：中文。只输出 JSON，不要 Markdown。

材料：
{text[:14000]}
""".strip()

    def _build_with_rules(self, text: str, style: str) -> dict[str, Any]:
        lines = [line.strip() for line in text.splitlines() if line.strip()]
        title = self._guess_title(lines)
        sections = self._split_sections(lines)
        if not sections:
            paragraphs = [p.strip() for p in re.split(r"\n\s*\n", text) if p.strip()]
            sections = [(f"重点 {idx + 1}", [p]) for idx, p in enumerate(paragraphs[:5])]
        summary = self._summary(text)
        agenda = [self._clean_heading(item[0]) for item in sections[:5]]
        if len(agenda) < 4:
            agenda.extend(["现状洞察", "方案设计", "落地路径", "价值总结"][len(agenda):4])
        slides: list[dict[str, Any]] = [
            {"layout": "cover", "chapter": "", "title": title, "bullets": ["自动生成 PPTX", "同步输出解说稿 DOCX", "适配 1080P 视频合成"], "speaker_note": f"大家好，今天汇报的主题是《{title}》。接下来我会围绕背景、关键内容、落地路径和预期价值展开说明。", "visual_prompt": f"cinematic business presentation opening, topic {title}, clean technology style, 16:9", "estimated_seconds": 22},
            {"layout": "agenda", "chapter": "目录", "title": "汇报目录", "bullets": agenda[:5], "speaker_note": "本次汇报分为几个部分：先看背景和目标，再说明核心内容与实施路径，最后总结价值和下一步安排。", "visual_prompt": "minimal business agenda page, structured cards, blue cyan accent, 16:9", "estimated_seconds": 18},
        ]
        for index, (heading, body) in enumerate(sections[:5], start=1):
            chapter = self._clean_heading(heading)
            slides.append({"layout": "section" if index in {1, 4} else "content", "chapter": f"第 {index} 部分", "title": chapter, "bullets": self._pick_bullets(body) or [chapter, "明确目标与关键约束", "形成可执行的推进动作"], "speaker_note": self._speaker_note(chapter, body), "visual_prompt": self._visual_prompt(chapter), "estimated_seconds": 35})
        slides.append({"layout": "summary", "chapter": "总结", "title": "总结与下一步", "bullets": ["围绕核心目标形成统一叙事", "通过自动化链路提升内容生产效率", "保留 AI/本地兜底双路径，降低交付风险", "后续可继续接入 Presenton、CosyVoice 与 Wan2.2"], "speaker_note": "最后做一个总结：本方案先保证端到端稳定可用，再逐步增强生成质量。即使没有外部 Token，也能完整输出 PPT、解说稿、音频、字幕和视频。", "visual_prompt": "executive summary closing slide, premium business technology, 16:9", "estimated_seconds": 28})
        return self._normalize({"title": title, "subtitle": "自动生成的商业汇报成片方案", "summary": summary, "style": style, "slides": slides[:10]}, style)

    def _normalize(self, data: dict[str, Any], style: str) -> dict[str, Any]:
        normalized_slides: list[dict[str, Any]] = []
        for slide in data.get("slides") or []:
            if not isinstance(slide, dict):
                continue
            bullets = slide.get("bullets") or []
            if isinstance(bullets, str):
                bullets = [bullets]
            layout = str(slide.get("layout") or "content").lower()
            if layout not in {"cover", "agenda", "section", "content", "summary"}:
                layout = "content"
            normalized_slides.append({"layout": layout, "chapter": self._clean_text(str(slide.get("chapter") or ""), 40), "title": self._clean_text(str(slide.get("title") or "未命名页面"), 80), "bullets": [self._clean_text(str(item), 120) for item in bullets if str(item).strip()][:6], "speaker_note": self._clean_text(str(slide.get("speaker_note") or ""), 900), "visual_prompt": self._clean_text(str(slide.get("visual_prompt") or ""), 500), "estimated_seconds": int(slide.get("estimated_seconds") or 0)})
        if not normalized_slides:
            normalized_slides = self._build_with_rules("", style)["slides"]
        normalized_slides[0]["layout"] = "cover"
        if len(normalized_slides) > 1 and normalized_slides[1]["layout"] == "content":
            normalized_slides[1]["layout"] = "agenda"
        if len(normalized_slides) > 2 and not any(item.get("layout") == "section" for item in normalized_slides):
            normalized_slides[2]["layout"] = "section"
            normalized_slides[2]["chapter"] = normalized_slides[2].get("chapter") or "章节过渡"
        normalized_slides[-1]["layout"] = "summary"
        return {"title": self._clean_text(str(data.get("title") or normalized_slides[0]["title"] or "AI汇报工厂"), 120), "subtitle": self._clean_text(str(data.get("subtitle") or "商业汇报自动生成方案"), 160), "summary": self._clean_text(str(data.get("summary") or ""), 650), "style": str(data.get("style") or style), "slides": normalized_slides[:10]}

    def _guess_title(self, lines: list[str]) -> str:
        for line in lines[:10]:
            clean = re.sub(r"^[#\s]+", "", line).strip(" -*#")
            if 4 <= len(clean) <= 60 and not clean.startswith(("http://", "https://")):
                return clean
        return "AI汇报工厂生成方案"

    def _split_sections(self, lines: list[str]) -> list[tuple[str, list[str]]]:
        sections: list[tuple[str, list[str]]] = []
        current_heading = ""
        current_body: list[str] = []
        heading_pattern = re.compile(r"^(#{1,3}\s+|[一二三四五六七八九十]+[、.．]|\d+[、.．]|第[一二三四五六七八九十\d]+[章节部分])")
        for line in lines:
            if heading_pattern.match(line) and len(line) <= 90:
                if current_heading:
                    sections.append((current_heading, current_body))
                current_heading = line
                current_body = []
            elif current_heading:
                current_body.append(line)
        if current_heading:
            sections.append((current_heading, current_body))
        return sections

    def _pick_bullets(self, body: list[str]) -> list[str]:
        bullets: list[str] = []
        for line in body:
            line = re.sub(r"^[\-*>•●\d\.、\s]+", "", line).strip()
            if not line or line.startswith(("http://", "https://")):
                continue
            for part in re.split(r"[。；;]\s*", line):
                part = part.strip()
                if 6 <= len(part) <= 120 and part not in bullets:
                    bullets.append(self._clean_text(part, 90))
                if len(bullets) >= 5:
                    break
            if len(bullets) >= 5:
                break
        return bullets

    def _clean_heading(self, heading: str) -> str:
        heading = re.sub(r"^[#\s]+", "", heading)
        heading = re.sub(r"^([一二三四五六七八九十]+|\d+)[、.．\s]*", "", heading)
        heading = re.sub(r"^第[一二三四五六七八九十\d]+[章节部分][：:\s]*", "", heading)
        return self._clean_text(heading, 60) or "关键内容"

    def _speaker_note(self, heading: str, body: list[str]) -> str:
        bullets = self._pick_bullets(body)
        point_text = "、".join(bullets[:3]) if bullets else "这一部分的关键目标、主要内容和落地动作"
        return f"这一页重点说明{heading}。我们需要关注{point_text}。通过把内容拆成清晰的结构，可以让听众更快理解重点，并为后续执行形成共识。"

    def _visual_prompt(self, heading: str) -> str:
        return f"professional business presentation video scene, topic {heading}, subtle motion graphics, blue cyan palette, clean cards, 16:9"

    def _summary(self, text: str) -> str:
        clean = re.sub(r"\s+", " ", text).strip()
        if not clean:
            return "系统根据输入材料自动生成汇报结构，并输出 PPT、解说稿、音频、字幕和 1080P 视频。"
        return clean[:260] + ("……" if len(clean) > 260 else "")

    def _clean_text(self, text: str, limit: int) -> str:
        text = re.sub(r"\s+", " ", text).strip()
        return text[:limit] + ("……" if len(text) > limit else "")

    def _extract_json(self, content: str) -> str:
        content = content.strip()
        if content.startswith("```"):
            content = re.sub(r"^```(?:json)?", "", content).strip()
            content = re.sub(r"```$", "", content).strip()
        match = re.search(r"\{.*\}", content, re.S)
        return match.group(0) if match else content

    def _trim(self, url: str) -> str:
        return url.rstrip("/")
