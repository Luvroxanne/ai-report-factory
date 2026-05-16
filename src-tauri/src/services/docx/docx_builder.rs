use std::{fs::File, io::Write, path::Path};

use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{db::models::ReportPlan, utils::errors::AppResult};

pub fn build_docx(plan: &ReportPlan, path: &Path) -> AppResult<()> {
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    write_entry(&mut zip, opts, "[Content_Types].xml", CONTENT_TYPES)?;
    write_entry(&mut zip, opts, "_rels/.rels", ROOT_RELS)?;
    write_entry(&mut zip, opts, "docProps/app.xml", APP_PROPS)?;
    write_entry(&mut zip, opts, "docProps/core.xml", &core_props(&plan.title))?;
    write_entry(&mut zip, opts, "word/_rels/document.xml.rels", DOCUMENT_RELS)?;
    write_entry(&mut zip, opts, "word/document.xml", &document_xml(plan))?;
    write_entry(&mut zip, opts, "word/styles.xml", STYLES)?;
    write_entry(&mut zip, opts, "word/settings.xml", SETTINGS)?;
    write_entry(&mut zip, opts, "word/fontTable.xml", FONT_TABLE)?;
    zip.finish()?;
    Ok(())
}

pub fn build_markdown(plan: &ReportPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", plan.title));
    out.push_str(&format!("> {}\n\n", plan.subtitle));
    out.push_str("## 报告摘要\n\n");
    out.push_str(&format!("{}\n\n", plan.summary));
    out.push_str("## 讲述节奏\n\n");
    out.push_str("- 开场：明确主题、听众收益和汇报结构。\n");
    out.push_str("- 主体：每页围绕一个判断展开，先结论后依据。\n");
    out.push_str("- 收束：回到行动建议，明确下一步。\n\n");
    for (idx, slide) in plan.slides.iter().enumerate() {
        out.push_str(&format!("## 第 {} 页：{}\n\n", idx + 1, slide.title));
        out.push_str(&format!("- 页面类型：{}\n", slide.layout));
        out.push_str(&format!("- 所属章节：{}\n", slide.chapter));
        out.push_str(&format!("- 建议时长：{} 秒\n\n", slide.estimated_seconds));
        out.push_str("### 页面要点\n\n");
        for bullet in &slide.bullets {
            out.push_str(&format!("- {bullet}\n"));
        }
        out.push_str("\n### 口播稿\n\n");
        out.push_str(&format!("{}\n\n", polished_note(plan, idx)));
    }
    out
}

pub fn build_txt(plan: &ReportPlan) -> String {
    let mut out = format!("{}\n{}\n\n报告摘要：{}\n\n", plan.title, plan.subtitle, plan.summary);
    for (idx, slide) in plan.slides.iter().enumerate() {
        out.push_str(&format!("第 {} 页：{}\n", idx + 1, slide.title));
        out.push_str(&format!("章节：{}｜建议时长：{} 秒\n", slide.chapter, slide.estimated_seconds));
        out.push_str("要点：\n");
        for bullet in &slide.bullets {
            out.push_str(&format!("  - {bullet}\n"));
        }
        out.push_str("口播稿：\n");
        out.push_str(&polished_note(plan, idx));
        out.push_str("\n\n");
    }
    out
}

fn write_entry(zip: &mut ZipWriter<File>, opts: SimpleFileOptions, name: &str, content: &str) -> AppResult<()> {
    zip.start_file(name, opts)?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

fn document_xml(plan: &ReportPlan) -> String {
    let mut body = String::new();
    body.push_str(&paragraph(&plan.title, "Title"));
    body.push_str(&paragraph(&plan.subtitle, "Subtitle"));
    body.push_str(&paragraph("报告摘要", "Heading1"));
    body.push_str(&paragraph(&plan.summary, "Body"));
    body.push_str(&paragraph("演讲者提示", "Heading1"));
    body.push_str(&paragraph("这份解说稿按 PPT 页码组织。建议汇报时先讲结论，再讲依据，最后落到下一步动作；每页可以根据现场互动适当压缩或展开。", "Body"));

    for (idx, slide) in plan.slides.iter().enumerate() {
        body.push_str(&paragraph(&format!("第 {} 页：{}", idx + 1, slide.title), "Heading1"));
        body.push_str(&paragraph(&format!("章节：{}　页面类型：{}　建议时长：{} 秒", slide.chapter, slide.layout, slide.estimated_seconds), "Meta"));
        body.push_str(&paragraph("页面要点", "Heading2"));
        for bullet in &slide.bullets {
            body.push_str(&bullet_paragraph(bullet));
        }
        body.push_str(&paragraph("完整口播稿", "Heading2"));
        for para in split_note(&polished_note(plan, idx)) {
            body.push_str(&paragraph(&para, "Body"));
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1200" w:right="1200" w:bottom="1200" w:left="1200" w:header="720" w:footer="720" w:gutter="0"/><w:cols w:space="720"/><w:docGrid w:linePitch="360"/></w:sectPr></w:body>
</w:document>"#
    )
}

fn polished_note(plan: &ReportPlan, idx: usize) -> String {
    let slide = &plan.slides[idx];
    let base = slide.speaker_note.trim();
    if base.chars().count() >= 70 {
        return base.to_string();
    }
    let points = slide.bullets.iter().take(3).cloned().collect::<Vec<_>>().join("；");
    let transition = if idx + 1 < plan.slides.len() {
        format!("讲完这一页后，我们自然过渡到“{}”。", plan.slides[idx + 1].title)
    } else {
        "最后可以用这一页收束全场，并引导大家确认下一步行动。".into()
    };
    format!("这一页的主题是“{}”。建议先用一句话给出结论，再展开三个关键点：{}。{} {}", slide.title, points, base, transition)
}

fn split_note(note: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for part in note.split(&['。', '；', ';'][..]) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if current.chars().count() + trimmed.chars().count() > 120 && !current.is_empty() {
            out.push(format!("{}。", current.trim()));
            current.clear();
        }
        if !current.is_empty() {
            current.push('；');
        }
        current.push_str(trimmed);
    }
    if !current.trim().is_empty() {
        out.push(format!("{}。", current.trim()));
    }
    out
}

fn paragraph(text: &str, style: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="{style}"/></w:pPr><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        escape_xml(text)
    )
}

fn bullet_paragraph(text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="Bullet"/></w:pPr><w:r><w:t xml:space="preserve">• {}</w:t></w:r></w:p>"#,
        escape_xml(text)
    )
}

fn core_props(title: &str) -> String {
    let now = chrono::Utc::now().to_rfc3339();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{}</dc:title><dc:creator>AI Report Factory</dc:creator><cp:lastModifiedBy>AI Report Factory</cp:lastModifiedBy><dcterms:created xsi:type="dcterms:W3CDTF">{now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{now}</dcterms:modified></cp:coreProperties>"#,
        escape_xml(title)
    )
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/word/fontTable.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"/></Types>"#;
const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;
const DOCUMENT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable" Target="fontTable.xml"/></Relationships>"#;
const APP_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>AI Report Factory</Application><DocSecurity>0</DocSecurity><ScaleCrop>false</ScaleCrop><Company>AI Report Factory</Company></Properties>"#;
const SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:zoom w:percent="100"/><w:defaultTabStop w:val="420"/><w:characterSpacingControl w:val="doNotCompress"/></w:settings>"#;
const FONT_TABLE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:fonts xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:font w:name="Microsoft YaHei"><w:family w:val="swiss"/><w:pitch w:val="variable"/></w:font></w:fonts>"#;
const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style w:type="paragraph" w:default="1" w:styleId="Body"><w:name w:val="Body"/><w:qFormat/><w:pPr><w:spacing w:after="160" w:line="320" w:lineRule="auto"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:sz w:val="22"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:qFormat/><w:pPr><w:spacing w:after="240"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:b/><w:color w:val="111827"/><w:sz w:val="42"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:qFormat/><w:pPr><w:spacing w:after="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:color w:val="2563EB"/><w:sz w:val="26"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="160"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:b/><w:color w:val="0F172A"/><w:sz w:val="30"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="220" w:after="120"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:b/><w:color w:val="2563EB"/><w:sz w:val="24"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Meta"><w:name w:val="Meta"/><w:qFormat/><w:pPr><w:spacing w:after="120"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:color w:val="64748B"/><w:sz w:val="19"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Bullet"><w:name w:val="Bullet"/><w:qFormat/><w:pPr><w:ind w:left="420" w:hanging="220"/><w:spacing w:after="80"/></w:pPr><w:rPr><w:rFonts w:ascii="Microsoft YaHei" w:eastAsia="Microsoft YaHei"/><w:sz w:val="21"/></w:rPr></w:style>
</w:styles>"#;
