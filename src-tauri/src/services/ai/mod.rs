pub mod fallback;
pub mod gemini;
pub mod ollama;
pub mod openai_compatible;

use crate::{
    config::app_config::{AiProvider, AppConfig},
    db::models::{ReportPlan, SlidePlan},
    utils::errors::AppResult,
};

pub fn build_report_plan(input: &str, title: &str, style: &str, config: &AppConfig) -> AppResult<ReportPlan> {
    if matches!(config.ai_provider, AiProvider::Local) {
        return Ok(enhance_plan(
            fallback::build_local_plan(input, title, style, "local fallback"),
            input,
            title,
            style,
            "local fallback",
        ));
    }

    let remote = match config.ai_provider {
        AiProvider::OpenAiCompatible => openai_compatible::generate_plan(input, title, style, config),
        AiProvider::Gemini => gemini::generate_plan(input, title, style, config),
        AiProvider::Ollama => ollama::generate_plan(input, title, style, config),
        AiProvider::Local => unreachable!(),
    };

    match remote {
        Ok(plan) => Ok(enhance_plan(plan, input, title, style, "AI provider + local quality pass")),
        Err(err) if config.enable_local_fallback => Ok(enhance_plan(
            fallback::build_local_plan(input, title, style, &format!("AI 调用失败，已启用本地规则兜底：{err}")),
            input,
            title,
            style,
            "AI failed + local fallback",
        )),
        Err(err) => Err(err),
    }
}

pub fn prompt(input: &str, title: &str, style: &str) -> String {
    format!(
        "你现在是一个由四个专家组成的本地报告生成 Agent：\n\
        1) 战略分析师：提炼问题、结论、证据和行动建议；\n\
        2) PPT 导演：把信息改写成可演示的页面，不堆字；\n\
        3) Word 解说稿作者：为每页写自然、完整、有转场的口播稿；\n\
        4) 视频分镜导演：为每页给出清晰章节、画面节奏和预计时长。\n\n\
        请基于资料生成高质量中文报告结构，主题：{title}，风格：{style}。\n\
        要求：\n\
        - 只返回 JSON，不要 Markdown 代码块，不要解释。\n\
        - 至少 9 页：封面、执行摘要、目录、指标卡片、时间线、对比分析、流程方案、行动建议、总结。\n\
        - 每页 bullets 3-5 条，每条 12-28 个中文字符，像专业 PPT 要点，不要复制原文长句。\n\
        - speaker_note 必须是完整口播稿，80-160 个中文字符，包含讲解逻辑和自然转场。\n\
        - layout 只能使用 cover、executive_summary、toc、metric_cards、timeline、comparison、process、insight_cards、recommendation、summary。\n\
        - estimated_seconds 取 20-60 秒。\n\
        JSON 字段：title, subtitle, summary, style, generation_note, slides。\n\
        slides 每项字段：title, bullets, speaker_note, layout, chapter, estimated_seconds。\n\n\
        资料：\n{input}"
    )
}

pub fn parse_plan_or_fallback(text: &str, input: &str, title: &str, style: &str, note: &str) -> ReportPlan {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let plan = serde_json::from_str::<ReportPlan>(cleaned)
        .unwrap_or_else(|_| fallback::build_local_plan(input, title, style, note));
    enhance_plan(plan, input, title, style, note)
}

pub fn enhance_plan(mut plan: ReportPlan, input: &str, title: &str, style: &str, note: &str) -> ReportPlan {
    if plan.title.trim().is_empty() {
        plan.title = if title.trim().is_empty() { infer_title(input) } else { title.trim().into() };
    }
    if plan.subtitle.trim().is_empty() {
        plan.subtitle = "专业 Agent 增强报告".into();
    }
    if plan.summary.trim().is_empty() {
        plan.summary = summary_from_input(input);
    }
    plan.style = style.to_string();
    if plan.generation_note.trim().is_empty() {
        plan.generation_note = note.to_string();
    }

    if plan.slides.is_empty() {
        plan = fallback::build_local_plan(input, &plan.title, style, note);
    }

    let backup = fallback::build_local_plan(input, &plan.title, style, "local quality supplement");
    while plan.slides.len() < 9 {
        let idx = plan.slides.len();
        if let Some(extra) = backup.slides.get(idx).cloned() {
            plan.slides.push(extra);
        } else {
            plan.slides.push(generic_slide(idx, &plan.title));
        }
    }

    let total = plan.slides.len();
    let input_points = extract_points(input);
    for idx in 0..total {
        plan.slides[idx].layout = quality_layout(idx, total, &plan.slides[idx].layout);
        if plan.slides[idx].title.trim().is_empty() {
            plan.slides[idx].title = default_title(idx, total).into();
        }
        plan.slides[idx].title = clean(&plan.slides[idx].title, 30);
        plan.slides[idx].chapter = if plan.slides[idx].chapter.trim().is_empty() {
            chapter_for(idx, total).into()
        } else {
            clean(&plan.slides[idx].chapter, 20)
        };
        plan.slides[idx].bullets = normalize_bullets(&plan.slides[idx].bullets, &input_points, idx);
        if plan.slides[idx].speaker_note.chars().count() < 80 {
            plan.slides[idx].speaker_note = build_speaker_note(&plan.slides, idx);
        }
        plan.slides[idx].speaker_note = clean_sentence(&plan.slides[idx].speaker_note, 220);
        plan.slides[idx].estimated_seconds = plan.slides[idx].estimated_seconds.clamp(24, 60);
    }
    plan
}

fn quality_layout(index: usize, total: usize, current: &str) -> String {
    if index == 0 {
        return "cover".into();
    }
    if index == 1 {
        return "executive_summary".into();
    }
    if index == 2 {
        return "toc".into();
    }
    if index + 1 == total {
        return "summary".into();
    }
    if index + 2 == total {
        return "recommendation".into();
    }
    match current {
        "metric_cards" | "timeline" | "comparison" | "process" | "insight_cards" => current.to_string(),
        _ => ["metric_cards", "timeline", "comparison", "process", "insight_cards"][(index - 3) % 5].to_string(),
    }
}

fn normalize_bullets(existing: &[String], input_points: &[String], slide_idx: usize) -> Vec<String> {
    let mut out: Vec<String> = existing
        .iter()
        .map(|s| clean(s, 28))
        .filter(|s| !s.is_empty())
        .collect();
    let seed = [
        "先给结论，再说明依据",
        "突出影响范围和优先级",
        "明确资源投入和节奏",
        "沉淀可复用的方法论",
        "用数据和案例支撑判断",
        "形成下一步行动闭环",
    ];
    let mut cursor = slide_idx;
    while out.len() < 4 {
        let candidate = input_points
            .get(cursor % input_points.len().max(1))
            .cloned()
            .unwrap_or_else(|| seed[cursor % seed.len()].to_string());
        out.push(format!("{}：{}", seed[cursor % seed.len()].chars().take(4).collect::<String>(), clean(&candidate, 20)));
        cursor += 1;
    }
    out.dedup();
    out.into_iter().take(5).collect()
}

fn build_speaker_note(slides: &[SlidePlan], idx: usize) -> String {
    let slide = &slides[idx];
    let points = slide.bullets.iter().take(3).cloned().collect::<Vec<_>>().join("；");
    let next = slides.get(idx + 1).map(|s| format!("接下来会进入“{}”，继续把这个判断落到更具体的方案上。", s.title)).unwrap_or_else(|| "这一页也可以作为结尾，引导大家确认共识和后续动作。".into());
    format!("这一页我们重点讲“{}”。建议先用一句话说明本页结论，再展开三个关键点：{}。这些内容的作用，是帮助听众快速判断重点、理解依据，并知道下一步应该关注什么。{next}", slide.title, points)
}

fn extract_points(input: &str) -> Vec<String> {
    let mut points: Vec<String> = input
        .lines()
        .map(|line| line.trim().trim_start_matches(&['#', '-', '*', '•', ' '][..]).trim())
        .filter(|line| !line.is_empty())
        .map(|line| clean(line, 32))
        .filter(|line| !line.is_empty())
        .collect();
    if points.is_empty() {
        points = input
            .split(&['。', '；', ';', '\n'][..])
            .map(|s| clean(s, 32))
            .filter(|s| !s.is_empty())
            .collect();
    }
    if points.is_empty() {
        points.push("围绕主题形成结构化分析".into());
    }
    points
}

fn generic_slide(index: usize, title: &str) -> SlidePlan {
    SlidePlan {
        title: default_title(index, 9).into(),
        bullets: vec![
            format!("围绕《{}》提炼关键判断", clean(title, 16)),
            "补充证据链和影响分析".into(),
            "拆解可执行的推进动作".into(),
            "明确阶段目标和验收标准".into(),
        ],
        speaker_note: String::new(),
        layout: "insight_cards".into(),
        chapter: chapter_for(index, 9).into(),
        estimated_seconds: 42,
    }
}

fn default_title(index: usize, total: usize) -> &'static str {
    if index == 0 {
        "报告封面"
    } else if index == 1 {
        "执行摘要"
    } else if index == 2 {
        "目录"
    } else if index + 2 == total {
        "行动建议"
    } else if index + 1 == total {
        "总结与下一步"
    } else {
        ["关键指标", "推进时间线", "对比分析", "流程方案", "核心洞察"][(index - 3) % 5]
    }
}

fn chapter_for(index: usize, total: usize) -> &'static str {
    if index <= 2 {
        "开场"
    } else if index + 2 >= total {
        "落地"
    } else {
        "分析"
    }
}

fn infer_title(input: &str) -> String {
    extract_points(input).first().cloned().unwrap_or_else(|| "AI 报告".into())
}

fn summary_from_input(input: &str) -> String {
    let text = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        "围绕输入资料生成结构化报告，覆盖结论、依据、方案和下一步。".into()
    } else {
        clean(&text, 90)
    }
}

fn clean(input: &str, limit: usize) -> String {
    let mut text = input
        .replace('\t', " ")
        .replace("**", "")
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > limit {
        text = text.chars().take(limit).collect();
    }
    text.trim_matches(&['-', '*', '•', '：', ':', '；', ';', '。', ' '][..]).to_string()
}

fn clean_sentence(input: &str, limit: usize) -> String {
    let mut text = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() > limit {
        text = text.chars().take(limit).collect::<String>();
        text.push('。');
    }
    text
}
