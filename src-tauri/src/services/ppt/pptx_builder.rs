use std::{fs::File, io::Write, path::Path};

use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{
    db::models::{ReportPlan, SlidePlan},
    utils::errors::AppResult,
};

const SLIDE_W: u32 = 12_192_000;
const SLIDE_H: u32 = 6_858_000;

pub fn build_pptx(plan: &ReportPlan, path: &Path) -> AppResult<()> {
    let slides = slides_for(plan);
    let slide_count = slides.len();
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    write_entry(&mut zip, opts, "[Content_Types].xml", &content_types(slide_count))?;
    write_entry(&mut zip, opts, "_rels/.rels", ROOT_RELS)?;
    write_entry(&mut zip, opts, "docProps/app.xml", &app_props(slide_count))?;
    write_entry(&mut zip, opts, "docProps/core.xml", &core_props(&plan.title))?;
    write_entry(&mut zip, opts, "ppt/presentation.xml", &presentation_xml(slide_count))?;
    write_entry(&mut zip, opts, "ppt/_rels/presentation.xml.rels", &presentation_rels(slide_count))?;
    write_entry(&mut zip, opts, "ppt/presProps.xml", PRES_PROPS)?;
    write_entry(&mut zip, opts, "ppt/viewProps.xml", VIEW_PROPS)?;
    write_entry(&mut zip, opts, "ppt/tableStyles.xml", TABLE_STYLES)?;
    write_entry(&mut zip, opts, "ppt/theme/theme1.xml", THEME)?;
    write_entry(&mut zip, opts, "ppt/slideMasters/slideMaster1.xml", SLIDE_MASTER)?;
    write_entry(&mut zip, opts, "ppt/slideMasters/_rels/slideMaster1.xml.rels", MASTER_RELS)?;
    write_entry(&mut zip, opts, "ppt/slideLayouts/slideLayout1.xml", SLIDE_LAYOUT)?;
    write_entry(&mut zip, opts, "ppt/slideLayouts/_rels/slideLayout1.xml.rels", LAYOUT_RELS)?;

    for (idx, slide) in slides.iter().enumerate() {
        let xml = slide_xml(plan, slide, idx, slide_count);
        write_entry(&mut zip, opts, &format!("ppt/slides/slide{}.xml", idx + 1), &xml)?;
        write_entry(&mut zip, opts, &format!("ppt/slides/_rels/slide{}.xml.rels", idx + 1), SLIDE_RELS)?;
    }
    zip.finish()?;
    Ok(())
}

fn slides_for(plan: &ReportPlan) -> Vec<SlidePlan> {
    if !plan.slides.is_empty() {
        return plan.slides.clone();
    }
    vec![SlidePlan {
        title: plan.title.clone(),
        bullets: vec![plan.summary.clone()],
        speaker_note: plan.summary.clone(),
        layout: "cover".into(),
        chapter: "报告".into(),
        estimated_seconds: 30,
    }]
}

fn write_entry(zip: &mut ZipWriter<File>, opts: SimpleFileOptions, name: &str, content: &str) -> AppResult<()> {
    zip.start_file(name, opts)?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

fn content_types(slide_count: usize) -> String {
    let mut overrides = String::new();
    for i in 1..=slide_count {
        overrides.push_str(&format!(r#"<Override PartName="/ppt/slides/slide{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/presProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presProps+xml"/>
<Override PartName="/ppt/viewProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml"/>
<Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
{overrides}
</Types>"#
    )
}

fn presentation_xml(slide_count: usize) -> String {
    let ids = (1..=slide_count)
        .map(|i| format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 255 + i, i + 1))
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" saveSubsetFonts="1">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst>{ids}</p:sldIdLst>
<p:sldSz cx="{SLIDE_W}" cy="{SLIDE_H}" type="wide"/>
<p:notesSz cx="6858000" cy="9144000"/>
<p:defaultTextStyle><a:defPPr><a:defRPr lang="zh-CN"/></a:defPPr></p:defaultTextStyle>
</p:presentation>"#
    )
}

fn presentation_rels(slide_count: usize) -> String {
    let mut rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#.to_string();
    for i in 1..=slide_count {
        rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i}.xml"/>"#, i + 1));
    }
    rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps" Target="presProps.xml"/>"#, slide_count + 2));
    rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/viewProps" Target="viewProps.xml"/>"#, slide_count + 3));
    rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles" Target="tableStyles.xml"/>"#, slide_count + 4));
    format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#)
}

fn slide_xml(plan: &ReportPlan, slide: &SlidePlan, index: usize, total: usize) -> String {
    let theme = ThemePreset::from_style(&plan.style);
    let dark = index == 0 || slide.layout == "cover" || slide.layout == "section";
    let bg = if dark { theme.dark_bg } else { theme.light_bg };
    let fg = if dark { theme.text_on_dark } else { theme.text_on_light };
    let muted = if dark { theme.muted_on_dark } else { theme.muted_on_light };
    let accent = theme.accent(index);
    let mut shapes = String::new();
    shapes.push_str(&rect_shape(2, 0, 0, SLIDE_W, SLIDE_H, bg, "bg"));
    shapes.push_str(&decorations(index, accent, dark));

    match slide.layout.as_str() {
        "cover" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 780_000, 1_080_000, 9_900_000, 1_260_000, 4_600, fg, true, "l"));
            shapes.push_str(&text_box(11, "subtitle", &plan.subtitle, 820_000, 2_330_000, 8_600_000, 520_000, 2_000, muted, false, "l"));
            for (i, bullet) in slide.bullets.iter().take(3).enumerate() {
                let x = 820_000 + i as u32 * 3_330_000;
                shapes.push_str(&card(30 + i as u32, x, 4_280_000, 2_820_000, 880_000, theme.card_on_dark, theme.line_on_dark));
                shapes.push_str(&text_box(40 + i as u32, "cover-card", bullet, x + 220_000, 4_470_000, 2_360_000, 420_000, 1_650, theme.muted_on_dark, true, "l"));
            }
        }
        "toc" => {
            shapes.push_str(&text_box(10, "title", "汇报目录", 720_000, 580_000, 5_500_000, 700_000, 3_300, fg, true, "l"));
            for (i, bullet) in slide.bullets.iter().take(6).enumerate() {
                let y = 1_620_000 + i as u32 * 700_000;
                shapes.push_str(&circle_badge(30 + i as u32, 930_000, y + 50_000, accent, i + 1));
                shapes.push_str(&card(50 + i as u32, 1_540_000, y, 8_950_000, 560_000, theme.card_on_light, theme.line_on_light));
                shapes.push_str(&text_box(70 + i as u32, "agenda", bullet, 1_850_000, y + 95_000, 8_200_000, 340_000, 1_650, theme.text_on_light, true, "l"));
            }
        }
        "metric_cards" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 720_000, 520_000, 8_900_000, 720_000, 3_150, fg, true, "l"));
            shapes.push_str(&text_box(11, "chapter", "关键指标 / 影响判断 / 优先级", 760_000, 1_180_000, 5_800_000, 330_000, 1_250, muted, false, "l"));
            for (i, bullet) in slide.bullets.iter().take(4).enumerate() {
                let x = 860_000 + (i as u32 % 2) * 5_250_000;
                let y = 1_850_000 + (i as u32 / 2) * 1_750_000;
                shapes.push_str(&card(40 + i as u32, x, y, 4_650_000, 1_260_000, if dark { theme.card_on_dark } else { theme.card_on_light }, if dark { theme.line_on_dark } else { theme.line_on_light }));
                shapes.push_str(&text_box(60 + i as u32, "metric-num", &format!("{:02}", i + 1), x + 260_000, y + 170_000, 850_000, 420_000, 2_500, accent, true, "l"));
                shapes.push_str(&text_box(80 + i as u32, "metric-text", bullet, x + 1_120_000, y + 180_000, 3_180_000, 650_000, 1_520, fg, true, "l"));
            }
        }
        "timeline" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 720_000, 560_000, 8_900_000, 720_000, 3_100, fg, true, "l"));
            shapes.push_str(&rect_shape(20, 1_080_000, 3_260_000, 9_900_000, 70_000, accent, "timeline-line"));
            for (i, bullet) in slide.bullets.iter().take(5).enumerate() {
                let x = 980_000 + i as u32 * 2_050_000;
                shapes.push_str(&circle_badge(30 + i as u32, x, 3_090_000, accent, i + 1));
                shapes.push_str(&card(50 + i as u32, x - 230_000, 3_720_000, 1_690_000, 980_000, if dark { theme.card_on_dark } else { theme.card_on_light }, if dark { theme.line_on_dark } else { theme.line_on_light }));
                shapes.push_str(&text_box(80 + i as u32, "timeline-text", bullet, x - 60_000, 3_920_000, 1_350_000, 530_000, 1_250, fg, true, "l"));
            }
        }
        "comparison" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 720_000, 560_000, 8_900_000, 720_000, 3_100, fg, true, "l"));
            shapes.push_str(&card(30, 880_000, 1_740_000, 4_850_000, 3_700_000, if dark { theme.card_on_dark } else { theme.card_on_light }, accent));
            shapes.push_str(&card(31, 6_050_000, 1_740_000, 4_850_000, 3_700_000, if dark { theme.card_on_dark } else { theme.card_on_light }, accent));
            shapes.push_str(&text_box(40, "left-head", "现状 / 约束", 1_180_000, 2_000_000, 4_250_000, 420_000, 1_850, accent, true, "l"));
            shapes.push_str(&text_box(41, "right-head", "机会 / 方案", 6_350_000, 2_000_000, 4_250_000, 420_000, 1_850, accent, true, "l"));
            let left = slide.bullets.iter().take(3).cloned().collect::<Vec<_>>().join("\n• ");
            let right = slide.bullets.iter().skip(2).take(3).cloned().collect::<Vec<_>>().join("\n• ");
            shapes.push_str(&text_box(50, "left-body", &format!("• {left}"), 1_180_000, 2_640_000, 4_120_000, 2_250_000, 1_420, fg, false, "l"));
            shapes.push_str(&text_box(51, "right-body", &format!("• {right}"), 6_350_000, 2_640_000, 4_120_000, 2_250_000, 1_420, fg, false, "l"));
        }
        "process" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 720_000, 560_000, 8_900_000, 720_000, 3_100, fg, true, "l"));
            for (i, bullet) in slide.bullets.iter().take(5).enumerate() {
                let y = 1_620_000 + i as u32 * 760_000;
                shapes.push_str(&rect_shape(40 + i as u32, 950_000, y, 520_000, 520_000, accent, "process-dot"));
                shapes.push_str(&text_box(60 + i as u32, "process-num", &format!("{}", i + 1), 1_070_000, y + 95_000, 280_000, 260_000, 1_350, "FFFFFF", true, "c"));
                shapes.push_str(&card(80 + i as u32, 1_720_000, y - 20_000, 8_880_000, 560_000, if dark { theme.card_on_dark } else { theme.card_on_light }, if dark { theme.line_on_dark } else { theme.line_on_light }));
                shapes.push_str(&text_box(110 + i as u32, "process-text", bullet, 2_040_000, y + 90_000, 8_150_000, 330_000, 1_470, fg, true, "l"));
            }
        }
        "insight_cards" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 720_000, 520_000, 8_900_000, 720_000, 3_100, fg, true, "l"));
            for (i, bullet) in slide.bullets.iter().take(5).enumerate() {
                let x = 820_000 + i as u32 * 2_100_000;
                let h = if i % 2 == 0 { 2_900_000 } else { 2_420_000 };
                shapes.push_str(&card(40 + i as u32, x, 1_820_000, 1_720_000, h, if dark { theme.card_on_dark } else { theme.card_on_light }, if dark { theme.line_on_dark } else { theme.line_on_light }));
                shapes.push_str(&rect_shape(70 + i as u32, x, 1_820_000, 1_720_000, 130_000, accent, "card-top"));
                shapes.push_str(&text_box(90 + i as u32, "insight", bullet, x + 180_000, 2_170_000, 1_360_000, 1_620_000, 1_330, fg, true, "l"));
            }
        }
        "summary" | "recommendation" | "executive_summary" => {
            shapes.push_str(&text_box(10, "title", &slide.title, 720_000, 560_000, 8_900_000, 720_000, 3_250, fg, true, "l"));
            shapes.push_str(&text_box(11, "chapter", &slide.chapter, 760_000, 1_260_000, 4_200_000, 360_000, 1_350, muted, false, "l"));
            for (i, bullet) in slide.bullets.iter().take(5).enumerate() {
                let x = if i % 2 == 0 { 840_000 } else { 6_180_000 };
                let y = 1_900_000 + (i as u32 / 2) * 1_120_000;
                shapes.push_str(&card(30 + i as u32, x, y, 4_720_000, 850_000, if dark { theme.card_on_dark } else { theme.card_on_light }, if dark { theme.line_on_dark } else { theme.line_on_light }));
                shapes.push_str(&rect_shape(60 + i as u32, x, y, 110_000, 850_000, accent, "accent"));
                shapes.push_str(&text_box(80 + i as u32, "point", bullet, x + 320_000, y + 185_000, 4_080_000, 420_000, 1_600, fg, true, "l"));
            }
        }
        _ => {
            shapes.push_str(&text_box(10, "section", &slide.chapter, 760_000, 500_000, 3_900_000, 360_000, 1_250, accent, true, "l"));
            shapes.push_str(&text_box(11, "title", &slide.title, 720_000, 850_000, 9_600_000, 820_000, 3_100, fg, true, "l"));
            for (i, bullet) in slide.bullets.iter().take(5).enumerate() {
                let y = 1_930_000 + i as u32 * 760_000;
                shapes.push_str(&card(40 + i as u32, 880_000, y, 9_780_000, 560_000, if dark { theme.card_on_dark } else { theme.card_on_light }, if dark { theme.line_on_dark } else { theme.line_on_light }));
                shapes.push_str(&text_box(70 + i as u32, "bullet", &format!("0{}  {}", i + 1, bullet), 1_150_000, y + 105_000, 9_080_000, 340_000, 1_520, fg, false, "l"));
            }
        }
    }
    shapes.push_str(&text_box(900, "footer", &format!("{} · AI Report Factory · {}/{}", theme.name, index + 1, total), 7_850_000, 6_220_000, 3_700_000, 260_000, 950, if dark { theme.muted_on_dark } else { theme.muted_on_light }, false, "r"));

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld name="{}"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{SLIDE_W}" cy="{SLIDE_H}"/><a:chOff x="0" y="0"/><a:chExt cx="{SLIDE_W}" cy="{SLIDE_H}"/></a:xfrm></p:grpSpPr>{shapes}</p:spTree></p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#,
        escape_attr(&slide.title)
    )
}

fn decorations(index: usize, accent: &str, dark: bool) -> String {
    let mut out = String::new();
    out.push_str(&rect_shape(3, 0, 0, 130_000, SLIDE_H, accent, "brand-line"));
    out.push_str(&rect_shape(4, 10_920_000, 420_000, 820_000, 820_000, if dark { "1E293B" } else { "E0F2FE" }, "deco"));
    out.push_str(&rect_shape(5, 10_520_000, 5_620_000, 1_280_000, 130_000, accent, "deco-line"));
    if index % 2 == 0 {
        out.push_str(&rect_shape(6, 9_850_000, 980_000, 1_720_000, 80_000, "38BDF8", "deco-line-2"));
    }
    out
}

fn card(id: u32, x: u32, y: u32, cx: u32, cy: u32, fill: &str, line: &str) -> String {
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="card"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{fill}"/></a:solidFill><a:ln w="9525"><a:solidFill><a:srgbClr val="{line}"/></a:solidFill></a:ln></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp>"#
    )
}

fn rect_shape(id: u32, x: u32, y: u32, cx: u32, cy: u32, fill: &str, name: &str) -> String {
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{name}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{fill}"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp>"#,
        name = escape_attr(name)
    )
}

fn circle_badge(id: u32, x: u32, y: u32, fill: &str, number: usize) -> String {
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="number"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="390000" cy="390000"/></a:xfrm><a:prstGeom prst="ellipse"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{fill}"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr anchor="mid"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="zh-CN" sz="1400" b="1"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill><a:latin typeface="Microsoft YaHei"/><a:ea typeface="Microsoft YaHei"/></a:rPr><a:t>{number}</a:t></a:r></a:p></p:txBody></p:sp>"#
    )
}

fn text_box(id: u32, name: &str, text: &str, x: u32, y: u32, cx: u32, cy: u32, size: u32, color: &str, bold: bool, align: &str) -> String {
    let bold_attr = if bold { r#" b="1""# } else { "" };
    let escaped = escape_xml(text);
    let align = match align {
        "c" => "ctr",
        "r" => "r",
        _ => "l",
    };
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr>
<p:txBody><a:bodyPr wrap="square" anchor="mid"><a:noAutofit/></a:bodyPr><a:lstStyle/><a:p><a:pPr algn="{align}"/><a:r><a:rPr lang="zh-CN" sz="{size}"{bold_attr}><a:solidFill><a:srgbClr val="{color}"/></a:solidFill><a:latin typeface="Microsoft YaHei"/><a:ea typeface="Microsoft YaHei"/></a:rPr><a:t>{escaped}</a:t></a:r></a:p></p:txBody></p:sp>"#,
        escape_attr(name)
    )
}

fn app_props(slide_count: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>AI Report Factory</Application><PresentationFormat>宽屏</PresentationFormat><Slides>{slide_count}</Slides><Company>AI Report Factory</Company></Properties>"#
    )
}

fn core_props(title: &str) -> String {
    let now = chrono::Utc::now().to_rfc3339();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{}</dc:title><dc:creator>AI Report Factory</dc:creator><cp:lastModifiedBy>AI Report Factory</cp:lastModifiedBy><dcterms:created xsi:type="dcterms:W3CDTF">{now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{now}</dcterms:modified></cp:coreProperties>"#,
        escape_xml(title)
    )
}

#[derive(Debug, Clone, Copy)]
struct ThemePreset {
    name: &'static str,
    dark_bg: &'static str,
    light_bg: &'static str,
    text_on_dark: &'static str,
    text_on_light: &'static str,
    muted_on_dark: &'static str,
    muted_on_light: &'static str,
    card_on_dark: &'static str,
    card_on_light: &'static str,
    line_on_dark: &'static str,
    line_on_light: &'static str,
    accents: [&'static str; 6],
}

impl ThemePreset {
    fn from_style(style: &str) -> Self {
        if style.contains("ivory-business") {
            Self {
                name: "商务白金",
                dark_bg: "1F2937",
                light_bg: "FFFBEB",
                text_on_dark: "FFFFFF",
                text_on_light: "1F2937",
                muted_on_dark: "FDE68A",
                muted_on_light: "6B4E16",
                card_on_dark: "374151",
                card_on_light: "FFFFFF",
                line_on_dark: "92400E",
                line_on_light: "FCD34D",
                accents: ["D97706", "92400E", "B45309", "4B5563", "CA8A04", "111827"],
            }
        } else if style.contains("emerald-training") {
            Self {
                name: "翡翠培训",
                dark_bg: "052E2B",
                light_bg: "ECFDF5",
                text_on_dark: "F0FDFA",
                text_on_light: "064E3B",
                muted_on_dark: "99F6E4",
                muted_on_light: "047857",
                card_on_dark: "134E4A",
                card_on_light: "FFFFFF",
                line_on_dark: "0F766E",
                line_on_light: "A7F3D0",
                accents: ["10B981", "14B8A6", "059669", "0D9488", "22C55E", "065F46"],
            }
        } else if style.contains("sunset-roadshow") {
            Self {
                name: "橙紫路演",
                dark_bg: "2E1065",
                light_bg: "FFF7ED",
                text_on_dark: "FFFFFF",
                text_on_light: "431407",
                muted_on_dark: "FED7AA",
                muted_on_light: "9A3412",
                card_on_dark: "4C1D95",
                card_on_light: "FFFFFF",
                line_on_dark: "F97316",
                line_on_light: "FDBA74",
                accents: ["F97316", "A855F7", "EC4899", "F59E0B", "7C3AED", "BE123C"],
            }
        } else if style.contains("violet-creative") {
            Self {
                name: "紫色创意",
                dark_bg: "1E1B4B",
                light_bg: "F5F3FF",
                text_on_dark: "FFFFFF",
                text_on_light: "312E81",
                muted_on_dark: "DDD6FE",
                muted_on_light: "6D28D9",
                card_on_dark: "312E81",
                card_on_light: "FFFFFF",
                line_on_dark: "8B5CF6",
                line_on_light: "C4B5FD",
                accents: ["8B5CF6", "6366F1", "A855F7", "EC4899", "06B6D4", "7C3AED"],
            }
        } else {
            Self {
                name: "极光科技蓝",
                dark_bg: "0F172A",
                light_bg: "F8FAFC",
                text_on_dark: "FFFFFF",
                text_on_light: "111827",
                muted_on_dark: "BAE6FD",
                muted_on_light: "475569",
                card_on_dark: "1E293B",
                card_on_light: "FFFFFF",
                line_on_dark: "334155",
                line_on_light: "CBD5E1",
                accents: ["38BDF8", "2563EB", "10B981", "F59E0B", "8B5CF6", "EF4444"],
            }
        }
    }

    fn accent(&self, index: usize) -> &'static str {
        self.accents[index % self.accents.len()]
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_attr(input: &str) -> String {
    escape_xml(input)
}

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;
const SLIDE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#;
const MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/></Relationships>"#;
const LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;
const PRES_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentationPr xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;
const VIEW_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:viewPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:normalViewPr><p:restoredLeft sz="15620"/><p:restoredTop sz="94660"/></p:normalViewPr><p:slideViewPr><p:cSldViewPr><p:cViewPr varScale="1"><p:scale><a:sx n="100" d="100"/><a:sy n="100" d="100"/></p:scale><p:origin x="0" y="0"/></p:cViewPr><p:guideLst/></p:cSldViewPr></p:slideViewPr></p:viewPr>"#;
const TABLE_STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"/>"#;
const SLIDE_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1"><p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#;
const SLIDE_MASTER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>"#;
const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="AI Report Factory">
<a:themeElements>
<a:clrScheme name="AI Report Factory"><a:dk1><a:srgbClr val="111827"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="0F172A"/></a:dk2><a:lt2><a:srgbClr val="F8FAFC"/></a:lt2><a:accent1><a:srgbClr val="2563EB"/></a:accent1><a:accent2><a:srgbClr val="38BDF8"/></a:accent2><a:accent3><a:srgbClr val="10B981"/></a:accent3><a:accent4><a:srgbClr val="F59E0B"/></a:accent4><a:accent5><a:srgbClr val="8B5CF6"/></a:accent5><a:accent6><a:srgbClr val="EF4444"/></a:accent6><a:hlink><a:srgbClr val="2563EB"/></a:hlink><a:folHlink><a:srgbClr val="8B5CF6"/></a:folHlink></a:clrScheme>
<a:fontScheme name="AI Report Factory"><a:majorFont><a:latin typeface="Microsoft YaHei"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/></a:majorFont><a:minorFont><a:latin typeface="Microsoft YaHei"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/></a:minorFont></a:fontScheme>
<a:fmtScheme name="AI Report Factory"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:lumMod val="110000"/><a:satMod val="105000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:lumMod val="90000"/><a:satMod val="105000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill><a:solidFill><a:schemeClr val="phClr"><a:lumMod val="95000"/></a:schemeClr></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="12700" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="19050" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"><a:tint val="95000"/></a:schemeClr></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:lumMod val="90000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill></a:bgFillStyleLst></a:fmtScheme>
</a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"#;
