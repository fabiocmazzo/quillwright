//! HTML: a single self-contained web page, and the XHTML used inside EPUB files.
use super::book::{notes_in, plain, sole_image, Block, Book, Chapter, Inline};
use super::xml::escape;
use base64::Engine;

/// Novel-style typography shared by the web page and the e-book.
pub const CSS: &str = r#"body {
  font-family: "Literata", Georgia, "Times New Roman", serif;
  line-height: 1.5;
  margin: 0 auto;
  max-width: 36em;
  padding: 2em 1.25em;
}
p { margin: 0; text-indent: 1.5em; text-align: justify; hyphens: auto; -webkit-hyphens: auto; }
h1 + p, h2 + p, h3 + p, h4 + p, .scene-break + p, .figure + p, blockquote + p,
ul + p, ol + p, pre + p, table + p, blockquote p, li p, .footnotes p, .noindent { text-indent: 0; }
h1.chapter { text-align: center; font-size: 1.6em; margin: 3em 0 2em; break-before: page; page-break-before: always; }
h2, h3, h4, h5, h6 { margin: 1.5em 0 0.75em; }
.title-page { text-align: center; margin: 5em 0; break-after: page; page-break-after: always; }
.title-page .title { font-size: 2.2em; margin: 0 0 0.3em; }
.title-page .subtitle { font-size: 1.2em; font-style: italic; text-indent: 0; text-align: center; }
.title-page .author { font-size: 1.2em; margin-top: 2em; text-indent: 0; text-align: center; }
.title-page .date, .title-page .rights { margin-top: 1em; text-indent: 0; text-align: center; }
.scene-break { text-align: center; text-indent: 0; margin: 1em 0; }
.figure { text-align: center; text-indent: 0; margin: 1em 0; }
img { max-width: 100%; height: auto; }
blockquote { margin: 1em 2em; }
pre { white-space: pre-wrap; font-size: 0.9em; }
table { border-collapse: collapse; margin: 1em auto; }
th, td { border: 1px solid #999; padding: 0.25em 0.5em; }
nav.toc ol { list-style: none; padding: 0; }
nav.toc li { margin: 0.3em 0; }
.footnotes { font-size: 0.9em; margin-top: 2em; border-top: 1px solid #999; }
.footnotes p { margin: 0.5em 0; }
sup a { text-decoration: none; }
"#;

/// How images and note markers are written, which differs between the
/// standalone page and the e-book.
#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Page,
    Epub,
}

pub struct Writer<'a> {
    pub book: &'a Book,
    pub target: Target,
}

impl Writer<'_> {
    fn image_src(&self, index: usize) -> String {
        let image = &self.book.images[index];
        match self.target {
            Target::Epub => format!("images/image-{}.{}", index + 1, image.extension()),
            Target::Page => format!(
                "data:{};base64,{}",
                image.mime(),
                base64::engine::general_purpose::STANDARD.encode(&image.bytes)
            ),
        }
    }

    pub fn inlines(&self, inlines: &[Inline], out: &mut String) {
        for inline in inlines {
            match inline {
                Inline::Text(t) => out.push_str(&escape(t)),
                Inline::Emph(c) => self.wrap("em", c, out),
                Inline::Strong(c) => self.wrap("strong", c, out),
                Inline::Strike(c) => self.wrap("del", c, out),
                Inline::Code(t) => {
                    out.push_str("<code>");
                    out.push_str(&escape(t));
                    out.push_str("</code>");
                }
                Inline::Link(url, c) => {
                    out.push_str(&format!("<a href=\"{}\">", escape(url)));
                    self.inlines(c, out);
                    out.push_str("</a>");
                }
                Inline::Image { image, alt } => match image {
                    Some(index) => out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\"/>",
                        self.image_src(*index),
                        escape(alt)
                    )),
                    None => out.push_str(&escape(alt)),
                },
                Inline::Note(n) => {
                    let n = n + 1;
                    let kind = if self.target == Target::Epub {
                        " epub:type=\"noteref\""
                    } else {
                        ""
                    };
                    out.push_str(&format!(
                        "<sup><a{kind} href=\"#note-{n}\" id=\"ref-{n}\">{n}</a></sup>"
                    ));
                }
                Inline::LineBreak => out.push_str("<br/>"),
            }
        }
    }

    fn wrap(&self, tag: &str, content: &[Inline], out: &mut String) {
        out.push_str(&format!("<{tag}>"));
        self.inlines(content, out);
        out.push_str(&format!("</{tag}>"));
    }

    pub fn blocks(&self, blocks: &[Block], out: &mut String) {
        for block in blocks {
            match block {
                Block::Para(inlines) => {
                    if let Some((Some(index), alt)) = sole_image(inlines) {
                        out.push_str(&format!(
                            "<div class=\"figure\"><img src=\"{}\" alt=\"{}\"/></div>\n",
                            self.image_src(index),
                            escape(alt)
                        ));
                    } else {
                        out.push_str("<p>");
                        self.inlines(inlines, out);
                        out.push_str("</p>\n");
                    }
                }
                Block::Heading(level, inlines) => {
                    let level = (*level).clamp(2, 6);
                    out.push_str(&format!("<h{level}>"));
                    self.inlines(inlines, out);
                    out.push_str(&format!("</h{level}>\n"));
                }
                Block::Quote(blocks) => {
                    out.push_str("<blockquote>\n");
                    self.blocks(blocks, out);
                    out.push_str("</blockquote>\n");
                }
                Block::List { start, items } => {
                    let open = match start {
                        Some(1) => "<ol>".to_string(),
                        Some(n) => format!("<ol start=\"{n}\">"),
                        None => "<ul>".to_string(),
                    };
                    out.push_str(&open);
                    out.push('\n');
                    for item in items {
                        out.push_str("<li>");
                        // Single-paragraph items stay compact.
                        match item.as_slice() {
                            [Block::Para(inlines)] => self.inlines(inlines, out),
                            _ => self.blocks(item, out),
                        }
                        out.push_str("</li>\n");
                    }
                    out.push_str(if start.is_some() {
                        "</ol>\n"
                    } else {
                        "</ul>\n"
                    });
                }
                Block::Code(code) => {
                    out.push_str("<pre><code>");
                    out.push_str(&escape(code));
                    out.push_str("</code></pre>\n");
                }
                Block::SceneBreak => out.push_str(
                    "<p class=\"scene-break\">*&#160;&#160;&#160;*&#160;&#160;&#160;*</p>\n",
                ),
                Block::Table { header, rows } => {
                    out.push_str("<table>\n");
                    if !header.is_empty() {
                        out.push_str("<thead><tr>");
                        for cell in header {
                            out.push_str("<th>");
                            self.inlines(cell, out);
                            out.push_str("</th>");
                        }
                        out.push_str("</tr></thead>\n");
                    }
                    out.push_str("<tbody>\n");
                    for row in rows {
                        out.push_str("<tr>");
                        for cell in row {
                            out.push_str("<td>");
                            self.inlines(cell, out);
                            out.push_str("</td>");
                        }
                        out.push_str("</tr>\n");
                    }
                    out.push_str("</tbody></table>\n");
                }
            }
        }
    }

    /// A chapter with its footnotes at the end.
    pub fn chapter(&self, chapter: &Chapter, id: &str, out: &mut String) {
        out.push_str(&format!("<h1 class=\"chapter\" id=\"{id}\">"));
        self.inlines(&chapter.title, out);
        out.push_str("</h1>\n");
        self.blocks(&chapter.blocks, out);
        let notes = notes_in(&chapter.blocks);
        if notes.is_empty() {
            return;
        }
        out.push_str("<section class=\"footnotes\">\n");
        for n in notes {
            let number = n + 1;
            let (open, close) = if self.target == Target::Epub {
                (
                    format!("<aside epub:type=\"footnote\" id=\"note-{number}\">"),
                    "</aside>",
                )
            } else {
                (format!("<div id=\"note-{number}\">"), "</div>")
            };
            out.push_str(&open);
            out.push_str(&format!("<p><a href=\"#ref-{number}\">{number}.</a> "));
            let body = &self.book.notes[n];
            // The first paragraph shares a line with the number.
            match body.split_first() {
                Some((Block::Para(first), rest)) => {
                    self.inlines(first, out);
                    out.push_str("</p>\n");
                    self.blocks(rest, out);
                }
                _ => {
                    out.push_str("</p>\n");
                    self.blocks(body, out);
                }
            }
            out.push_str(close);
            out.push('\n');
        }
        out.push_str("</section>\n");
    }

    pub fn title_page(&self, out: &mut String) {
        let meta = &self.book.meta;
        out.push_str("<section class=\"title-page\">\n");
        out.push_str(&format!(
            "<h1 class=\"title\">{}</h1>\n",
            escape(&meta.title)
        ));
        for (class, value) in [
            ("subtitle", &meta.subtitle),
            ("author", &meta.author),
            ("date", &meta.date),
            ("rights", &meta.rights),
        ] {
            if !value.trim().is_empty() {
                out.push_str(&format!("<p class=\"{class}\">{}</p>\n", escape(value)));
            }
        }
        out.push_str("</section>\n");
    }
}

/// The whole manuscript as one web page.
pub fn page(book: &Book, table_of_contents: bool) -> String {
    let writer = Writer {
        book,
        target: Target::Page,
    };
    let meta = &book.meta;
    let mut out = String::new();
    out.push_str(&format!(
        "<!DOCTYPE html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\"/>\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n\
         <meta name=\"generator\" content=\"Quillwright\"/>\n<title>{}</title>\n",
        escape(&meta.language_tag()),
        escape(&meta.title)
    ));
    if !meta.author.is_empty() {
        out.push_str(&format!(
            "<meta name=\"author\" content=\"{}\"/>\n",
            escape(&meta.author)
        ));
    }
    if !meta.description.is_empty() {
        out.push_str(&format!(
            "<meta name=\"description\" content=\"{}\"/>\n",
            escape(&meta.description)
        ));
    }
    out.push_str("<style>\n");
    out.push_str(CSS);
    out.push_str("</style>\n</head>\n<body>\n");
    writer.title_page(&mut out);
    if table_of_contents {
        out.push_str(&format!(
            "<nav class=\"toc\">\n<h2>{}</h2>\n<ol>\n",
            meta.contents_label()
        ));
        for (i, chapter) in book.chapters.iter().enumerate() {
            out.push_str(&format!(
                "<li><a href=\"#chapter-{}\">{}</a></li>\n",
                i + 1,
                escape(&plain(&chapter.title))
            ));
        }
        out.push_str("</ol>\n</nav>\n");
    }
    for (i, chapter) in book.chapters.iter().enumerate() {
        out.push_str("<section>\n");
        writer.chapter(chapter, &format!("chapter-{}", i + 1), &mut out);
        out.push_str("</section>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_is_escaped_and_complete() {
        let html = page(&crate::export::sample_book(), true);
        assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"pt-BR\">"));
        assert!(html.contains("<title>Salt &amp; Sea</title>"));
        assert!(html.contains("<h2>Sumário</h2>"));
        assert!(html.contains("<a href=\"#chapter-2\">Two</a>"));
        assert!(html.contains("<h1 class=\"chapter\" id=\"chapter-1\">Tide &amp; Harbor</h1>"));
        assert!(html.contains(
            "<p>Tide &amp; <em>rope</em>.<sup><a href=\"#note-1\" id=\"ref-1\">1</a></sup></p>"
        ));
        assert!(html.contains("class=\"scene-break\""));
        assert!(html.contains("<div id=\"note-1\"><p><a href=\"#ref-1\">1.</a> A note.</p>"));
        assert!(!html.contains("epub:type"));
    }
}
