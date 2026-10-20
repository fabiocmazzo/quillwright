//! Word documents with docx-rs. Style IDs match the ones pandoc uses, so a
//! `reference.docx` made for pandoc keeps working.
use super::book::{sole_image, Block, Book, Inline};
use super::PageSize;
use docx_rs::{
    AlignmentType, BreakType, Docx, Footer, Footnote, Hyperlink, HyperlinkType, LineSpacing,
    PageMargin, PageNum, Paragraph, Pic, Run, RunFonts, RunProperty, SpecialIndentType, Style,
    StyleType, Table, TableCell, TableOfContents, TableRow, VertAlignType,
};

const TWIPS_PER_MM: f64 = 1440.0 / 25.4;
const EMU_PER_TWIP: u32 = 635;
const INDENT: i32 = 567; // 1 cm

#[derive(Clone, Copy, Default)]
struct Format {
    italic: bool,
    bold: bool,
    strike: bool,
    code: bool,
}

// Short-lived while building one paragraph, so the size difference does not matter.
#[allow(clippy::large_enum_variant)]
enum Child {
    Run(Run),
    Link(Hyperlink),
}

fn add_children(mut paragraph: Paragraph, children: Vec<Child>) -> Paragraph {
    for child in children {
        paragraph = match child {
            Child::Run(run) => paragraph.add_run(run),
            Child::Link(link) => paragraph.add_hyperlink(link),
        };
    }
    paragraph
}

fn styled_run(text: &str, format: Format) -> Run {
    let mut run = Run::new().add_text(text);
    if format.italic {
        run = run.italic();
    }
    if format.bold {
        run = run.bold();
    }
    if format.strike {
        run = run.strike();
    }
    if format.code {
        run = run.style("VerbatimChar");
    }
    run
}

struct Writer<'a> {
    book: &'a Book,
    docx: Docx,
    /// Widest an image may be, in EMU.
    text_width_emu: u32,
}

impl Writer<'_> {
    fn runs(&self, inlines: &[Inline], format: Format, out: &mut Vec<Child>) {
        for inline in inlines {
            match inline {
                Inline::Text(t) => out.push(Child::Run(styled_run(t, format))),
                Inline::Emph(c) => self.runs(
                    c,
                    Format {
                        italic: !format.italic,
                        ..format
                    },
                    out,
                ),
                Inline::Strong(c) => self.runs(
                    c,
                    Format {
                        bold: true,
                        ..format
                    },
                    out,
                ),
                Inline::Strike(c) => self.runs(
                    c,
                    Format {
                        strike: true,
                        ..format
                    },
                    out,
                ),
                Inline::Code(t) => out.push(Child::Run(styled_run(
                    t,
                    Format {
                        code: true,
                        ..format
                    },
                ))),
                Inline::Link(url, c) => {
                    let mut inner = Vec::new();
                    self.runs(c, format, &mut inner);
                    let mut link = Hyperlink::new(url.as_str(), HyperlinkType::External);
                    for child in inner {
                        if let Child::Run(run) = child {
                            link = link.add_run(run.style("Hyperlink"));
                        }
                    }
                    out.push(Child::Link(link));
                }
                Inline::Image { image: Some(i), .. } => {
                    out.push(Child::Run(Run::new().add_image(self.picture(*i))))
                }
                Inline::Image { image: None, alt } => out.push(Child::Run(styled_run(alt, format))),
                Inline::Note(n) => {
                    let mut footnote = Footnote::new();
                    for paragraph in self.note_paragraphs(*n) {
                        footnote.add_content(paragraph);
                    }
                    out.push(Child::Run(Run::new().add_footnote_reference(footnote)));
                }
                Inline::LineBreak => {
                    out.push(Child::Run(Run::new().add_break(BreakType::TextWrapping)))
                }
            }
        }
    }

    fn paragraph(&self, style: &str, inlines: &[Inline]) -> Paragraph {
        let mut children = Vec::new();
        self.runs(inlines, Format::default(), &mut children);
        add_children(Paragraph::new().style(style), children)
    }

    fn note_paragraphs(&self, note: usize) -> Vec<Paragraph> {
        let blocks = self.book.notes.get(note).cloned().unwrap_or_default();
        let mut paragraphs = Vec::new();
        for block in &blocks {
            let inlines = match block {
                Block::Para(i) | Block::Heading(_, i) => i.clone(),
                Block::Code(t) => vec![Inline::Code(t.clone())],
                other => vec![Inline::Text(block_text(other))],
            };
            paragraphs.push(self.paragraph("FootnoteText", &inlines));
        }
        paragraphs
    }

    fn picture(&self, index: usize) -> Pic {
        let image = &self.book.images[index];
        // 96 px per inch; an inch is 914 400 EMU.
        let mut width = image.width.max(1) * 9525;
        let mut height = image.height.max(1) * 9525;
        if width > self.text_width_emu {
            height = (height as u64 * self.text_width_emu as u64 / width as u64) as u32;
            width = self.text_width_emu;
        }
        Pic::new(&image.bytes).size(width, height)
    }

    fn push(&mut self, paragraph: Paragraph) {
        self.docx = std::mem::take(&mut self.docx).add_paragraph(paragraph);
    }

    /// Write blocks. `first` is true at the start of a chapter or after a
    /// scene break, where the paragraph is not indented.
    fn blocks(&mut self, blocks: &[Block], mut first: bool, indent: i32) {
        for block in blocks {
            match block {
                Block::Para(inlines) => {
                    if let Some((Some(i), _)) = sole_image(inlines) {
                        let picture = self.picture(i);
                        self.push(
                            Paragraph::new()
                                .style("CaptionedFigure")
                                .add_run(Run::new().add_image(picture)),
                        );
                    } else {
                        let style = if indent > 0 {
                            "BlockText"
                        } else if first {
                            "FirstParagraph"
                        } else {
                            "BodyText"
                        };
                        let mut paragraph = self.paragraph(style, inlines);
                        if indent > 0 {
                            paragraph = paragraph.indent(Some(indent), None, None, None);
                        }
                        self.push(paragraph);
                    }
                    first = false;
                }
                Block::Heading(level, inlines) => {
                    let style = format!("Heading{}", (*level).clamp(2, 3));
                    let paragraph = self.paragraph(&style, inlines);
                    self.push(paragraph);
                    first = true;
                }
                Block::Quote(inner) => {
                    self.blocks(inner, true, indent + INDENT);
                    first = true;
                }
                Block::List { start, items } => {
                    for (n, item) in items.iter().enumerate() {
                        let marker = match start {
                            Some(s) => format!("{}.\t", s + n as u64),
                            None => "•\t".to_string(),
                        };
                        let level = indent + INDENT;
                        for (b, block) in item.iter().enumerate() {
                            match block {
                                Block::Para(inlines) => {
                                    let mut children = vec![];
                                    if b == 0 {
                                        children.push(Child::Run(Run::new().add_text(&marker)));
                                    }
                                    self.runs(inlines, Format::default(), &mut children);
                                    let special = (b == 0)
                                        .then_some(SpecialIndentType::Hanging(INDENT / 2 + 100));
                                    self.push(
                                        add_children(Paragraph::new().style("Compact"), children)
                                            .indent(Some(level), special, None, None),
                                    );
                                }
                                other => self.blocks(std::slice::from_ref(other), true, level),
                            }
                        }
                    }
                    first = true;
                }
                Block::Code(text) => {
                    let mut run = Run::new();
                    for (i, line) in text.trim_end().lines().enumerate() {
                        if i > 0 {
                            run = run.add_break(BreakType::TextWrapping);
                        }
                        run = run.add_text(line);
                    }
                    self.push(Paragraph::new().style("SourceCode").add_run(run).indent(
                        Some(indent),
                        None,
                        None,
                        None,
                    ));
                    first = true;
                }
                Block::SceneBreak => {
                    self.push(
                        Paragraph::new()
                            .style("SceneBreak")
                            .add_run(Run::new().add_text("*   *   *")),
                    );
                    first = true;
                }
                Block::Table { header, rows } => {
                    let columns = header
                        .len()
                        .max(rows.iter().map(Vec::len).max().unwrap_or(0));
                    let mut table_rows = Vec::new();
                    let mut all = Vec::new();
                    if !header.is_empty() {
                        all.push((true, header));
                    }
                    all.extend(rows.iter().map(|r| (false, r)));
                    for (is_header, row) in all {
                        let cells = (0..columns)
                            .map(|c| {
                                let cell = row.get(c).cloned().unwrap_or_default();
                                let cell = if is_header {
                                    vec![Inline::Strong(cell)]
                                } else {
                                    cell
                                };
                                TableCell::new().add_paragraph(self.paragraph("Compact", &cell))
                            })
                            .collect();
                        table_rows.push(TableRow::new(cells));
                    }
                    self.docx = std::mem::take(&mut self.docx).add_table(Table::new(table_rows));
                    first = true;
                }
            }
        }
    }
}

/// Plain text of a block that cannot appear inside a footnote as such.
fn block_text(block: &Block) -> String {
    match block {
        Block::Para(i) | Block::Heading(_, i) => super::book::plain(i),
        Block::Quote(b) => b.iter().map(block_text).collect::<Vec<_>>().join(" "),
        Block::List { items, .. } => items
            .iter()
            .flatten()
            .map(block_text)
            .collect::<Vec<_>>()
            .join("; "),
        Block::Code(t) => t.clone(),
        Block::SceneBreak => "*".into(),
        Block::Table { .. } => String::new(),
    }
}

/// Paragraph properties docx-rs only offers on paragraphs, not on styles.
trait StyleExt {
    fn keep_next_style(self) -> Self;
    fn page_break_before_style(self) -> Self;
}

impl StyleExt for Style {
    fn keep_next_style(mut self) -> Self {
        self.paragraph_property = self.paragraph_property.keep_next(true);
        self
    }

    fn page_break_before_style(mut self) -> Self {
        self.paragraph_property = self
            .paragraph_property
            .page_break_before(true)
            .keep_next(true);
        self
    }
}

fn spacing(before: u32, after: u32) -> LineSpacing {
    LineSpacing::new().before(before).after(after)
}

fn paragraph_style(id: &str, name: &str) -> Style {
    Style::new(id, StyleType::Paragraph)
        .name(name)
        .q_format(true)
}

/// Styles in the spirit of a manuscript: a serif face, 1.5 line spacing,
/// indented paragraphs without space between them.
fn styles(docx: Docx, small: bool) -> Docx {
    let body = if small { 22 } else { 24 }; // half-points
    let mut superscript = Style::new("FootnoteReference", StyleType::Character)
        .name("footnote reference")
        .based_on("DefaultParagraphFont");
    superscript.run_property = RunProperty::new().vert_align(VertAlignType::SuperScript);
    let list = [
        paragraph_style("Normal", "Normal").line_spacing(spacing(0, 0).line(360)),
        paragraph_style("BodyText", "Body Text")
            .based_on("Normal")
            .indent(None, Some(SpecialIndentType::FirstLine(INDENT)), None, None),
        paragraph_style("FirstParagraph", "First Paragraph")
            .based_on("BodyText")
            .next("BodyText")
            .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None),
        paragraph_style("Compact", "Compact").based_on("Normal"),
        paragraph_style("BlockText", "Block Text")
            .based_on("Normal")
            .line_spacing(spacing(120, 120)),
        paragraph_style("Title", "Title")
            .based_on("Normal")
            .next("Subtitle")
            .align(AlignmentType::Center)
            .size(body * 2 + 8)
            .line_spacing(spacing(3600, 240)),
        paragraph_style("Subtitle", "Subtitle")
            .based_on("Normal")
            .align(AlignmentType::Center)
            .italic()
            .size(body + 6),
        paragraph_style("Author", "Author")
            .based_on("Normal")
            .align(AlignmentType::Center)
            .size(body + 4)
            .line_spacing(spacing(1440, 0)),
        paragraph_style("Date", "Date")
            .based_on("Normal")
            .align(AlignmentType::Center)
            .line_spacing(spacing(480, 0)),
        paragraph_style("Heading1", "heading 1")
            .based_on("Normal")
            .next("FirstParagraph")
            .align(AlignmentType::Center)
            .size(body + 12)
            .outline_lvl(0)
            .line_spacing(spacing(2880, 720))
            .page_break_before_style(),
        paragraph_style("Heading2", "heading 2")
            .based_on("Normal")
            .next("FirstParagraph")
            .size(body + 4)
            .bold()
            .outline_lvl(1)
            .line_spacing(spacing(360, 120))
            .keep_next_style(),
        paragraph_style("Heading3", "heading 3")
            .based_on("Normal")
            .next("FirstParagraph")
            .bold()
            .italic()
            .outline_lvl(2)
            .line_spacing(spacing(240, 120))
            .keep_next_style(),
        paragraph_style("TOCHeading", "TOC Heading")
            .based_on("Normal")
            .align(AlignmentType::Center)
            .size(body + 12)
            .line_spacing(spacing(1440, 720))
            .page_break_before_style(),
        paragraph_style("SourceCode", "Source Code")
            .based_on("Normal")
            .fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New"))
            .size(body - 4)
            .line_spacing(spacing(120, 120).line(240)),
        paragraph_style("SceneBreak", "Scene Break")
            .based_on("Normal")
            .align(AlignmentType::Center)
            .line_spacing(spacing(240, 240)),
        paragraph_style("CaptionedFigure", "Captioned Figure")
            .based_on("Normal")
            .align(AlignmentType::Center)
            .line_spacing(spacing(240, 240)),
        paragraph_style("FootnoteText", "footnote text")
            .based_on("Normal")
            .size(body - 4)
            .line_spacing(spacing(0, 60).line(240)),
        Style::new("DefaultParagraphFont", StyleType::Character).name("Default Paragraph Font"),
        superscript,
        Style::new("Hyperlink", StyleType::Character)
            .name("Hyperlink")
            .based_on("DefaultParagraphFont")
            .color("0563C1")
            .underline("single"),
        Style::new("VerbatimChar", StyleType::Character)
            .name("Verbatim Char")
            .based_on("DefaultParagraphFont")
            .fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New")),
    ];
    list.into_iter().fold(docx, Docx::add_style)
}

pub fn write(book: &Book, table_of_contents: bool, page: PageSize) -> Result<Vec<u8>, String> {
    let meta = &book.meta;
    let (width, height) = page.millimetres();
    let margin = if page.is_small() { 18.0 } else { 25.0 };
    let twips = |mm: f64| (mm * TWIPS_PER_MM).round() as i32;
    let text_width = twips(width - 2.0 * margin);

    let docx = Docx::new()
        .page_size(twips(width) as u32, twips(height) as u32)
        .page_margin(PageMargin {
            top: twips(margin),
            left: twips(margin),
            bottom: twips(margin),
            right: twips(margin),
            header: twips(margin / 2.0),
            footer: twips(margin / 2.0),
            gutter: 0,
        })
        .default_fonts(
            RunFonts::new()
                .ascii("Times New Roman")
                .hi_ansi("Times New Roman")
                .cs("Times New Roman")
                .east_asia("Times New Roman"),
        )
        .default_size(if page.is_small() { 22 } else { 24 })
        .title_pg()
        .first_footer(Footer::new())
        .footer(
            Footer::new().add_paragraph(
                Paragraph::new()
                    .align(AlignmentType::Center)
                    .add_page_num(PageNum::new()),
            ),
        );
    let docx = styles(docx, page.is_small());
    let mut writer = Writer {
        book,
        docx,
        text_width_emu: text_width.max(1) as u32 * EMU_PER_TWIP,
    };

    // Title page.
    writer.push(
        Paragraph::new()
            .style("Title")
            .add_run(Run::new().add_text(&meta.title)),
    );
    for (style, value) in [
        ("Subtitle", &meta.subtitle),
        ("Author", &meta.author),
        ("Date", &meta.date),
        ("Date", &meta.rights),
    ] {
        if !value.trim().is_empty() {
            writer.push(
                Paragraph::new()
                    .style(style)
                    .add_run(Run::new().add_text(value)),
            );
        }
    }

    if table_of_contents {
        // Word fills the table in when the document is opened (it asks first).
        let toc = TableOfContents::new()
            .heading_styles_range(1, 1)
            .hyperlink()
            .auto()
            .dirty()
            .add_before_paragraph(
                Paragraph::new()
                    .style("TOCHeading")
                    .add_run(Run::new().add_text(meta.contents_label())),
            );
        writer.docx = std::mem::take(&mut writer.docx).add_table_of_contents(toc);
    }

    for chapter in &book.chapters {
        let heading = writer.paragraph("Heading1", &chapter.title);
        writer.push(heading);
        writer.blocks(&chapter.blocks, true, 0);
    }

    let mut bytes = std::io::Cursor::new(Vec::new());
    writer
        .docx
        .build()
        .pack(&mut bytes)
        .map_err(|e| format!("Could not write the file: {e}"))?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::xml::read_entry;

    #[test]
    fn docx_has_styles_chapters_and_notes() {
        let bytes = write(&crate::export::sample_book(), true, PageSize::A5).unwrap();
        let entry = |name: &str| String::from_utf8(read_entry(&bytes, name).unwrap()).unwrap();
        let document = entry("word/document.xml");
        assert!(document.contains("w:val=\"Heading1\""));
        assert!(document.contains("Tide &amp; Harbor"));
        assert!(document.contains("w:val=\"FirstParagraph\""));
        assert!(document.contains("*   *   *"));
        assert!(document.contains("Sumário"));
        assert!(document.contains("TOC \\o"));
        assert!(entry("word/footnotes.xml").contains("A note."));
        let styles = entry("word/styles.xml");
        assert!(styles.contains("w:styleId=\"BodyText\""));
        assert!(styles.contains("w:styleId=\"FootnoteReference\""));
    }
}
