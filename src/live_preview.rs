//! Markdown projection. Display offsets are explicitly mapped to source characters.
use egui::{text::LayoutJob, Color32, FontFamily, FontId, Stroke, TextFormat, Ui};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::ops::Range;

#[derive(Clone, Copy, Default, PartialEq)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    link: bool,
    quote: bool,
    heading: u8,
    text: bool,
    /// Scene break (`***`, `---`): shown as a centered glyph.
    scene_break: bool,
    replacement: Option<char>,
}

fn styles(source: &str) -> Vec<Style> {
    let mut result = vec![Style::default(); source.len()];
    // Match the extensions used by the former egui_commonmark preview.
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES;
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if matches!(tag, Tag::Item) {
                    let raw = &source[range.clone()];
                    let marker_len = raw.find(char::is_whitespace).unwrap_or(0);
                    let prefix = &raw[..marker_len];
                    if matches!(prefix, "-" | "*" | "+")
                        || prefix.ends_with('.')
                        || prefix.ends_with(')')
                    {
                        let line_start = source[..range.start].rfind('\n').map_or(0, |i| i + 1);
                        for style in
                            &mut result[line_start..(range.start + marker_len + 1).min(range.end)]
                        {
                            style.text = true;
                        }
                        if marker_len == 1 {
                            result[range.start].replacement = Some('•');
                        }
                    }
                }
                for style in &mut result[range] {
                    match tag {
                        Tag::Strong => style.bold = true,
                        Tag::Emphasis => style.italic = true,
                        Tag::Strikethrough => style.strike = true,
                        Tag::Heading { level, .. } => {
                            style.heading = level as u8;
                            style.bold = true;
                        }
                        Tag::CodeBlock(_) => style.code = true,
                        Tag::BlockQuote => style.quote = true,
                        Tag::Link { .. } | Tag::Image { .. } => style.link = true,
                        Tag::Table(_) => style.code = true,
                        _ => {}
                    }
                }
            }
            Event::Rule => {
                // Keep the first marker character visible and swap it for a glyph.
                let first = &mut result[range.start];
                first.text = true;
                first.scene_break = true;
                first.replacement = Some('⁂');
            }
            Event::Text(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_) => {
                for style in &mut result[range] {
                    style.text = true;
                }
            }
            Event::TaskListMarker(checked) => {
                result[range.start].text = true;
                result[range.start].replacement = Some(if checked { '☑' } else { '☐' });
                if source.as_bytes().get(range.end) == Some(&b' ') {
                    result[range.end].text = true;
                }
            }
            Event::Code(_) => {
                let raw = &source[range.clone()];
                let ticks = raw.bytes().take_while(|&b| b == b'`').count();
                for style in &mut result[range.clone()] {
                    style.code = true;
                }
                for style in &mut result[range.start + ticks..range.end - ticks] {
                    style.text = true;
                }
            }
            _ => {}
        }
    }
    result
}

fn active_lines(source: &str, selection: Option<(usize, usize)>) -> Option<Range<usize>> {
    let (a, b) = selection?;
    let byte = |index| {
        source
            .char_indices()
            .nth(index)
            .map_or(source.len(), |(i, _)| i)
    };
    let start = byte(a.min(b));
    let end = byte(a.max(b));
    let start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let end = source[end..].find('\n').map_or(source.len(), |i| end + i);
    Some(start..end)
}

/// Byte range of the blank-line-separated paragraph containing `char_index`.
fn paragraph(source: &str, char_index: usize) -> Range<usize> {
    let at = source
        .char_indices()
        .nth(char_index)
        .map_or(source.len(), |(i, _)| i);
    let start = source[..at].rfind("\n\n").map_or(0, |i| i + 2);
    let end = source[at..].find("\n\n").map_or(source.len(), |i| at + i);
    start..end.max(start)
}

pub struct Projection {
    pub job: LayoutJob,
    // One source character index per displayed character, plus the end boundary.
    pub source_offsets: Vec<usize>,
}

impl Projection {
    pub fn to_source(&self, display: usize) -> usize {
        self.source_offsets[display.min(self.source_offsets.len() - 1)]
    }

    pub fn to_display(&self, source: usize) -> usize {
        self.source_offsets
            .partition_point(|&offset| offset < source)
            .min(self.source_offsets.len() - 1)
    }
}

pub fn project(ui: &Ui, source: &str, width: f32, selection: Option<(usize, usize)>) -> Projection {
    let styles = styles(source);
    let active = active_lines(source, selection);
    let metrics = crate::fonts::typography(ui.ctx());
    let text_color = ui.visuals().text_color();
    let dim = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(egui::Id::new("editor_dim")))
        .unwrap_or(false);
    let focus = selection.filter(|_| dim).map(|(primary, _)| paragraph(source, primary));
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut source_offsets = Vec::new();
    for (char_index, (index, ch)) in source.char_indices().enumerate() {
        let style = styles[index];
        let syntax = !style.text;
        let editing = active.as_ref().is_some_and(|range| range.contains(&index));
        if syntax && !editing && ch != '\n' {
            continue;
        }
        source_offsets.push(char_index);
        let color = if editing {
            text_color
        } else if style.link {
            ui.visuals().hyperlink_color
        } else if style.quote || style.scene_break {
            ui.visuals().weak_text_color()
        } else {
            text_color
        };
        let color = match &focus {
            Some(range) if !range.contains(&index) && ch != '\n' => color.gamma_multiply(0.3),
            _ => color,
        };
        let family = if style.code {
            FontFamily::Monospace
        } else {
            crate::fonts::family(style.bold && !editing, style.italic && !editing)
        };
        let scale = match if editing { 0 } else { style.heading } {
            1 => 1.8,
            2 => 1.5,
            3 => 1.3,
            4 => 1.15,
            _ if style.scene_break && !editing => 1.5,
            _ => 1.0,
        };
        let format = TextFormat {
            font_id: FontId::new(metrics.size * scale, family),
            color,
            line_height: Some(metrics.size * scale * metrics.line_height),
            background: if style.code {
                ui.visuals().code_bg_color
            } else {
                Color32::TRANSPARENT
            },
            underline: if style.link && !syntax && !editing {
                Stroke::new(1.0_f32, color)
            } else {
                Stroke::NONE
            },
            strikethrough: if style.strike && !syntax && !editing {
                Stroke::new(1.0_f32, color)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        };
        // Coalesce adjacent characters to avoid one layout section per character.
        let start = job.text.len();
        job.text.push(if editing {
            ch
        } else {
            style.replacement.unwrap_or(ch)
        });
        let centered = style.scene_break && !editing;
        if centered {
            job.sections.push(egui::text::LayoutSection {
                leading_space: (width / 2.0 - metrics.size * 0.75).max(0.0),
                byte_range: start..job.text.len(),
                format,
            });
        } else if let Some(last) = job.sections.last_mut().filter(|last| last.format == format) {
            last.byte_range.end = job.text.len();
        } else {
            job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: start..job.text.len(),
                format,
            });
        }
    }
    if job.sections.is_empty() {
        job.append(
            "",
            0.0,
            TextFormat {
                font_id: FontId::new(metrics.size, crate::fonts::family(false, false)),
                line_height: Some(metrics.size * metrics.line_height),
                color: text_color,
                ..Default::default()
            },
        );
    }
    source_offsets.push(source.chars().count());
    Projection {
        job,
        source_offsets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimming_fades_only_other_paragraphs() {
        let ctx = egui::Context::default();
        crate::fonts::install_test_fonts(&ctx);
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("editor_dim"), true));
        let source = "First paragraph.\n\nSecond one.";
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let cursor = source.find("Second").unwrap() + 2;
                let job = project(ui, source, 300.0, Some((cursor, cursor))).job;
                let color_at = |text: &str| {
                    let at = job.text.find(text).unwrap();
                    job.sections
                        .iter()
                        .find(|s| s.byte_range.contains(&at))
                        .unwrap()
                        .format
                        .color
                };
                let active = color_at("Second");
                let faded = color_at("First");
                assert!(faded.a() < active.a(), "{faded:?} vs {active:?}");
                // Without editor focus (no selection) nothing is dimmed.
                let job = project(ui, source, 300.0, None).job;
                assert!(job.sections.iter().all(|s| s.format.color == active));
            });
        });
    }

    #[test]
    fn scene_breaks_and_paragraph_ranges() {
        let source = "Before.\n\n***\n\nAfter.";
        let styles = styles(source);
        let star = source.find('*').unwrap();
        assert!(styles[star].scene_break && styles[star].text);
        assert_eq!(styles[star].replacement, Some('⁂'));
        assert!(!styles[star + 1].text);
        let after = source.find("After").unwrap();
        assert_eq!(&source[paragraph(source, after)], "After.");
        assert_eq!(&source[paragraph(source, 2)], "Before.");
        assert_eq!(paragraph("", 0), 0..0);
    }

    #[test]
    fn nested_formatting_and_unicode_use_source_offsets() {
        let source = "# Olá **bold *nested***\n\n~~gone~~ [link](url) `code`";
        let s = styles(source);
        assert_eq!(s[source.find("Olá").unwrap()].heading, 1);
        let nested = s[source.find("nested").unwrap()];
        assert!(nested.bold && nested.italic && nested.text);
        assert!(!s[source.find("**").unwrap()].text);
        assert!(s[source.find("gone").unwrap()].strike);
        assert!(s[source.find("link").unwrap()].link);
        assert!(!s[source.find("url").unwrap()].text);
        assert!(s[source.find("code").unwrap()].code);
    }

    #[test]
    fn code_does_not_parse_emphasis_and_incomplete_markdown_stays_text() {
        let source = "```rust\n**literal**\n```\n\n*unfinished";
        let s = styles(source);
        let code = s[source.find("literal").unwrap()];
        assert!(code.code && code.text && !code.bold);
        assert!(!s[0].text);
        assert!(s[source.find("*unfinished").unwrap()].text);
    }

    #[test]
    fn active_lines_handle_unicode_reversed_selection_and_eof() {
        let source = "éé\n**bold**\nlast";
        assert_eq!(active_lines(source, Some((1, 1))), Some(0..4));
        assert_eq!(active_lines(source, Some((5, 1))), Some(0..13));
        assert_eq!(active_lines(source, Some((99, 99))), Some(14..18));
        assert_eq!(active_lines(source, None), None);
    }

    #[test]
    fn projection_hides_syntax_and_maps_unicode_source() {
        let ctx = egui::Context::default();
        crate::fonts::install_test_fonts(&ctx);
        let source = "# Chapter One\n\nThe **café** was *moving*. [link](url)";
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let inactive = project(ui, source, 300.0, None);
                assert_eq!(
                    inactive.job.text,
                    "Chapter One\n\nThe café was moving. link"
                );
                for (display, ch) in inactive.job.text.chars().enumerate() {
                    let src = inactive.to_source(display);
                    assert_eq!(source.chars().nth(src), Some(ch));
                    assert_eq!(inactive.to_display(src), display);
                }
                let active = project(ui, source, 300.0, Some((20, 20)));
                assert_eq!(
                    active.job.text,
                    "Chapter One\n\nThe **café** was *moving*. [link](url)"
                );
                let all = project(ui, source, 300.0, Some((0, source.chars().count())));
                assert_eq!(all.job.text, source);
                assert_eq!(project(ui, "", 300.0, None).to_source(0), 0);
            });
        });
    }
}
