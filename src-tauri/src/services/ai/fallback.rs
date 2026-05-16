use crate::db::models::{ReportPlan, SlidePlan};

pub fn build_local_plan(input: &str, title: &str, style: &str, note: &str) -> ReportPlan {
    let title = if title.trim().is_empty() {
        infer_title(input)
    } else {
        clean_text(title, 42)
    };
    let sections = extract_sections(input);
    let thesis = summarize(input);
    let agenda = agenda_titles(&sections);

    let mut slides = Vec::new();
    slides.push(SlidePlan {
        title: title.clone(),
        bullets: vec![
            "面向决策的结构化汇报".into(),
            "从现状、洞察到行动路径".into(),
            "本地生成 PPT、讲稿与视频".into(),
        ],
        speaker_note: format!("大家好，今天汇报的主题是《{title}》。这份报告会先给出核心结论，再展开关键依据和落地建议，帮助大家快速形成统一判断。"),
        layout: "cover".into(),
        chapter: "开场".into(),
        estimated_seconds: 24,
    });

    slides.push(SlidePlan {
        title: "执行摘要".into(),
        bullets: executive_bullets(&sections, &thesis),
        speaker_note: format!("先看执行摘要。本次材料的核心信息可以概括为：{thesis}。接下来我会围绕问题背景、关键发现和行动建议逐层展开。"),
        layout: "executive_summary".into(),
        chapter: "开场".into(),
        estimated_seconds: 36,
    });

    slides.push(SlidePlan {
        title: "目录".into(),
        bullets: agenda.clone(),
        speaker_note: format!("本次汇报分为{}个部分：{}。这样的结构可以保证先看全局，再看细节，最后落到可执行动作。", agenda.len(), agenda.join("、")),
        layout: "toc".into(),
        chapter: "开场".into(),
        estimated_seconds: 24,
    });

    for (idx, section) in sections.iter().take(5).enumerate() {
        let bullets = professional_bullets(section, idx);
        let layout = if idx == 0 { "section" } else { "content" };
        slides.push(SlidePlan {
            title: section.title.clone(),
            speaker_note: speaker_note(idx, &section.title, &bullets),
            bullets,
            layout: layout.into(),
            chapter: section.title.clone(),
            estimated_seconds: 42,
        });
    }

    slides.push(SlidePlan {
        title: "行动建议".into(),
        bullets: vec![
            "先固化可复用报告模板".into(),
            "把数据、脚本和产物闭环".into(),
            "逐步增强语音与视频体验".into(),
            "用历史记录沉淀最佳实践".into(),
        ],
        speaker_note: "基于前面的分析，建议先把高频报告场景模板化，保证 PPT、Word 和视频产物稳定可复用；再持续接入更多模型和素材能力，形成完整的本地内容生产闭环。".into(),
        layout: "recommendation".into(),
        chapter: "落地".into(),
        estimated_seconds: 42,
    });

    slides.push(SlidePlan {
        title: "总结与下一步".into(),
        bullets: vec![
            "主流程聚焦可靠交付".into(),
            "多媒体能力保持可选增强".into(),
            "持续提升模板和表达质量".into(),
        ],
        speaker_note: "最后总结，本次方案的重点不是堆砌功能，而是让报告生成主流程真正可用、可复现、可交付。下一步可以继续增强模板系统、音色选择和视频分镜表现。谢谢大家。".into(),
        layout: "summary".into(),
        chapter: "总结".into(),
        estimated_seconds: 32,
    });

    ReportPlan {
        title,
        subtitle: subtitle_for_style(style),
        summary: thesis,
        style: style.to_string(),
        slides,
        generation_note: note.to_string(),
    }
}

#[derive(Debug, Clone)]
struct Section {
    title: String,
    points: Vec<String>,
}

fn infer_title(input: &str) -> String {
    input
        .lines()
        .find_map(|line| {
            let trimmed = line.trim().trim_start_matches('#').trim();
            (!trimmed.is_empty()).then(|| clean_text(trimmed, 42))
        })
        .unwrap_or_else(|| "AI 报告".to_string())
}

fn extract_sections(input: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_title = String::new();
    let mut points: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            push_section(&mut sections, &mut current_title, &mut points);
            current_title = clean_text(trimmed.trim_start_matches('#').trim(), 26);
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("• ") {
            points.push(clean_text(&trimmed[2..], 44));
        } else if points.len() < 6 {
            points.push(clean_text(trimmed, 50));
        }
    }
    push_section(&mut sections, &mut current_title, &mut points);

    if sections.is_empty() {
        let sentences: Vec<String> = input
            .split(&['。', '！', '？', '.', '!', '?', '\n'][..])
            .map(|s| clean_text(s.trim(), 44))
            .filter(|s| !s.is_empty())
            .take(18)
            .collect();
        for (idx, chunk) in sentences.chunks(4).take(4).enumerate() {
            sections.push(Section {
                title: ["背景与目标", "关键发现", "方案设计", "落地路径"].get(idx).unwrap_or(&"核心内容").to_string(),
                points: chunk.to_vec(),
            });
        }
    }

    while sections.len() < 3 {
        let title = ["背景与目标", "关键洞察", "落地路径"][sections.len()].to_string();
        sections.push(Section {
            title,
            points: vec![
                "明确业务目标和边界".into(),
                "提炼关键问题和机会".into(),
                "形成可执行推进计划".into(),
            ],
        });
    }
    sections
}

fn push_section(sections: &mut Vec<Section>, title: &mut String, points: &mut Vec<String>) {
    if title.trim().is_empty() {
        return;
    }
    let mut normalized = dedup(points);
    if normalized.is_empty() {
        normalized = vec![
            "现状需要进一步结构化".into(),
            "关键判断需要形成共识".into(),
            "后续动作需要明确责任".into(),
        ];
    }
    sections.push(Section {
        title: title.clone(),
        points: normalized,
    });
    title.clear();
    points.clear();
}

fn agenda_titles(sections: &[Section]) -> Vec<String> {
    let mut items: Vec<String> = sections.iter().take(5).map(|s| s.title.clone()).collect();
    if !items.iter().any(|s| s.contains("建议") || s.contains("路径")) {
        items.push("行动建议".into());
    }
    items
}

fn executive_bullets(sections: &[Section], summary: &str) -> Vec<String> {
    let mut out = vec![
        clean_text(summary, 26),
        "关键问题已具备结构化线索".into(),
        "交付重点应聚焦稳定主流程".into(),
    ];
    if let Some(first) = sections.first().and_then(|s| s.points.first()) {
        out.push(format!("首要抓手：{}", clean_text(first, 18)));
    }
    normalize_count(out, "执行摘要")
}

fn professional_bullets(section: &Section, index: usize) -> Vec<String> {
    let prefixes = [
        ["背景判断", "核心矛盾", "影响范围", "优先目标", "衡量指标"],
        ["关键发现", "证据线索", "价值机会", "风险约束", "决策要点"],
        ["方案抓手", "流程闭环", "资源配置", "体验提升", "交付标准"],
        ["实施路径", "阶段目标", "责任分工", "里程碑", "复盘机制"],
        ["增长空间", "质量保障", "扩展能力", "长期沉淀", "下一步"],
    ];
    let prefix = prefixes.get(index).unwrap_or(&prefixes[1]);
    let mut bullets = Vec::new();
    for (idx, point) in section.points.iter().take(5).enumerate() {
        let label = prefix.get(idx).unwrap_or(&"关键要点");
        bullets.push(format!("{label}：{}", clean_text(point, 22)));
    }
    normalize_count(bullets, &section.title)
}

fn normalize_count(mut bullets: Vec<String>, context: &str) -> Vec<String> {
    bullets = dedup(&bullets);
    let fallback = [
        format!("目标聚焦：{}", clean_text(context, 16)),
        "判断清晰：形成统一认知".into(),
        "路径明确：拆解可执行动作".into(),
        "结果可衡量：沉淀复盘指标".into(),
    ];
    for item in fallback {
        if bullets.len() >= 4 {
            break;
        }
        bullets.push(item);
    }
    bullets.into_iter().take(5).collect()
}

fn speaker_note(index: usize, title: &str, bullets: &[String]) -> String {
    let lead = match index {
        0 => "首先看第一个核心部分。",
        1 => "在明确背景之后，我们继续看关键发现。",
        2 => "第三部分进入方案设计。",
        3 => "接下来关注落地路径。",
        _ => "最后补充一个重要视角。",
    };
    let joined = bullets
        .iter()
        .take(3)
        .map(|s| s.replace('：', "，"))
        .collect::<Vec<_>>()
        .join("；");
    format!("{lead}这一页围绕“{title}”展开，重点包括：{joined}。请注意，这些要点不是孤立信息，而是服务于后续行动选择和资源投入优先级。")
}

fn subtitle_for_style(style: &str) -> String {
    if style.contains("agent-pro") {
        "专业 Agent 增强报告".into()
    } else if style.contains("training") {
        "培训课件与讲解稿".into()
    } else if style.contains("roadshow") {
        "路演汇报与行动建议".into()
    } else {
        "专业汇报与本地生成".into()
    }
}

fn summarize(input: &str) -> String {
    let compact = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return "围绕输入主题形成结构化报告，覆盖背景、洞察、方案和下一步。".into();
    }
    clean_text(&compact, 72)
}

fn dedup(items: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let cleaned = clean_text(item, 44);
        if !cleaned.is_empty() && !out.iter().any(|old| old == &cleaned) {
            out.push(cleaned);
        }
    }
    out
}

fn clean_text(input: &str, limit: usize) -> String {
    let mut text = input
        .replace('\t', " ")
        .replace("**", "")
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > limit {
        text = text.chars().take(limit).collect::<String>();
    }
    text.trim_matches(&['-', '*', '•', '：', ':', '；', ';', '。', ' '][..]).to_string()
}
