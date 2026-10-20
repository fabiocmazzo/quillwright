//! PDF through typst, used as a library: the book is written as typst markup
//! and laid out with the bundled Literata fonts. Nothing is read from disk or
//! the network while compiling.
use super::book::{notes_in, sole_image, Block, Book, Inline};
use super::PageSize;
use std::collections::HashMap;
use std::fmt::Write;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

/// A typst string literal, which shows its text verbatim: no markup inside
/// the author's prose is ever interpreted.
fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' | '\r' | '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn image_name(book: &Book, index: usize) -> String {
    format!("image-{}.{}", index + 1, book.images[index].extension())
}

struct Markup<'a> {
    book: &'a Book,
    /// Width of the text block in millimetres, to keep images inside it.
    text_width: f64,
    out: String,
}

impl Markup<'_> {
    fn inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            match inline {
                Inline::Text(t) => {
                    let _ = write!(self.out, "#{}", string(t));
                }
                Inline::Emph(c) => self.wrapped("emph", c),
                Inline::Strong(c) => self.wrapped("strong", c),
                Inline::Strike(c) => self.wrapped("strike", c),
                Inline::Code(t) => {
                    let _ = write!(self.out, "#raw({})", string(t));
                }
                Inline::Link(url, c) => {
                    let _ = write!(self.out, "#link({})[", string(url));
                    self.inlines(c);
                    self.out.push(']');
                }
                Inline::Image { image: Some(i), .. } => {
                    let _ = write!(
                        self.out,
                        "#box(image({}, height: 1em))",
                        string(&image_name(self.book, *i))
                    );
                }
                Inline::Image { image: None, alt } => {
                    let _ = write!(self.out, "#{}", string(alt));
                }
                Inline::Note(n) => {
                    self.out.push_str("#footnote[");
                    let note = self.book.notes.get(*n).cloned().unwrap_or_default();
                    self.blocks(&note);
                    self.out.push(']');
                }
                Inline::LineBreak => self.out.push_str("#linebreak()"),
            }
        }
    }

    fn wrapped(&mut self, function: &str, content: &[Inline]) {
        let _ = write!(self.out, "#{function}[");
        self.inlines(content);
        self.out.push(']');
    }

    /// Content in square brackets, for function arguments.
    fn content(&mut self, blocks: &[Block]) {
        self.out.push('[');
        self.blocks(blocks);
        self.out.push(']');
    }

    fn blocks(&mut self, blocks: &[Block]) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                self.out.push_str("\n\n");
            }
            self.block(block);
        }
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Para(inlines) => match sole_image(inlines) {
                Some((Some(i), _)) => {
                    let image = &self.book.images[i];
                    // 150 dpi, never wider than the text.
                    let natural = image.width as f64 / 150.0 * 25.4;
                    let _ = write!(
                        self.out,
                        "#block(above: 1.2em, below: 1.2em, width: 100%, align(center, image({}, width: {:.1}mm)))",
                        string(&image_name(self.book, i)),
                        natural.min(self.text_width)
                    );
                }
                _ => self.inlines(inlines),
            },
            Block::Heading(level, inlines) => {
                let _ = write!(self.out, "#heading(level: {level}, outlined: false)[");
                self.inlines(inlines);
                self.out.push(']');
            }
            Block::Quote(blocks) => {
                self.out.push_str("#quote(block: true)");
                self.content(blocks);
            }
            Block::List { start, items } => {
                match start {
                    Some(n) => {
                        let _ = write!(self.out, "#enum(start: {n}");
                    }
                    None => self.out.push_str("#list("),
                }
                for (i, item) in items.iter().enumerate() {
                    if i > 0 || start.is_some() {
                        self.out.push_str(", ");
                    }
                    self.content(item);
                }
                self.out.push(')');
            }
            Block::Code(text) => {
                let _ = write!(
                    self.out,
                    "#raw(block: true, {})",
                    code_string(text.trim_end())
                );
            }
            Block::SceneBreak => self
                .out
                .push_str("#block(above: 1.2em, below: 1.2em, width: 100%, align(center)[#\"*\"#h(1.5em)#\"*\"#h(1.5em)#\"*\"])"),
            Block::Table { header, rows } => {
                let columns = header.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
                let _ = write!(
                    self.out,
                    "#block(above: 1.2em, below: 1.2em, width: 100%, align(center, table(columns: {}",
                    columns.max(1)
                );
                if !header.is_empty() {
                    self.out.push_str(", table.header(");
                    for (i, cell) in header.iter().enumerate() {
                        if i > 0 {
                            self.out.push_str(", ");
                        }
                        self.out.push_str("strong[");
                        self.inlines(cell);
                        self.out.push(']');
                    }
                    self.out.push(')');
                }
                for row in rows {
                    for c in 0..columns {
                        self.out.push_str(", [");
                        if let Some(cell) = row.get(c) {
                            self.inlines(cell);
                        }
                        self.out.push(']');
                    }
                }
                self.out.push_str(")))");
            }
        }
    }
}

/// A string literal that keeps line breaks (for code blocks).
fn code_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("    "),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The whole document as typst markup.
fn markup(book: &Book, table_of_contents: bool, page: PageSize) -> String {
    let meta = &book.meta;
    let (width, height) = page.millimetres();
    let (margin_x, margin_y, size) = if page.is_small() {
        (18.0, 20.0, "10.5pt")
    } else {
        (25.0, 25.0, "12pt")
    };
    let (lang, region) = meta.language_parts();
    let lang = if (2..=3).contains(&lang.len()) && lang.chars().all(|c| c.is_ascii_alphabetic()) {
        lang
    } else {
        "en".to_string()
    };
    let region = region
        .filter(|r| r.chars().all(|c| c.is_ascii_alphabetic()))
        .map(|r| format!(", region: {}", string(&r)))
        .unwrap_or_default();

    let mut out = String::new();
    let _ = write!(out, "#set document(title: {}", string(&meta.title));
    if !meta.author.trim().is_empty() {
        let _ = write!(out, ", author: {}", string(meta.author.trim()));
    }
    out.push_str(")\n");
    let _ = writeln!(
        out,
        "#set page(width: {width}mm, height: {height}mm, margin: (x: {margin_x}mm, y: {margin_y}mm))"
    );
    let _ = writeln!(
        out,
        "#set text(font: \"Literata\", size: {size}, lang: {}{region}, hyphenate: true)",
        string(&lang)
    );
    out.push_str(
        "#set par(justify: true, leading: 0.7em, spacing: 0.7em, first-line-indent: (amount: 1.5em, all: false))\n\
         #show raw: set text(font: \"Literata\", size: 0.9em)\n\
         #show heading: set text(weight: \"regular\")\n\
         #show heading.where(level: 1): it => {\n\
           pagebreak(weak: true)\n\
           v(18%)\n\
           align(center, text(size: 1.6em, it.body))\n\
           v(2.5em)\n\
         }\n\
         #show heading: set block(above: 1.6em, below: 1em)\n\
         #show quote.where(block: true): set pad(x: 1.5em)\n\
         #show outline.entry.where(level: 1): set block(above: 0.9em)\n",
    );

    // Title page, without a page number.
    out.push_str("#page(numbering: none)[#align(center + horizon)[\n");
    let _ = writeln!(out, "#text(size: 2.2em)[#{}]", string(&meta.title));
    if !meta.subtitle.trim().is_empty() {
        let _ = writeln!(
            out,
            "\n#v(0.6em)\n#text(size: 1.2em, style: \"italic\")[#{}]",
            string(&meta.subtitle)
        );
    }
    if !meta.author.trim().is_empty() {
        let _ = writeln!(
            out,
            "\n#v(3em)\n#text(size: 1.2em)[#{}]",
            string(&meta.author)
        );
    }
    for extra in [&meta.date, &meta.rights] {
        if !extra.trim().is_empty() {
            let _ = writeln!(out, "\n#v(1.5em)\n#{}", string(extra));
        }
    }
    out.push_str("]]\n");

    if table_of_contents {
        let _ = writeln!(
            out,
            "#page(numbering: none)[#outline(title: {}, depth: 1)]",
            string(meta.contents_label())
        );
    }
    out.push_str("#set page(numbering: \"1\")\n#counter(page).update(1)\n\n");

    let mut writer = Markup {
        book,
        text_width: width - 2.0 * margin_x,
        out,
    };
    for chapter in &book.chapters {
        writer.out.push_str("#heading(level: 1)[");
        writer.inlines(&chapter.title);
        writer.out.push_str("]\n\n");
        writer.blocks(&chapter.blocks);
        writer.out.push_str("\n\n");
    }
    // Footnotes are written in place, so the list is only checked for sanity.
    debug_assert!(book
        .chapters
        .iter()
        .flat_map(|c| notes_in(&c.blocks))
        .all(|n| n < book.notes.len()));
    writer.out
}

struct BookWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
    files: HashMap<FileId, Bytes>,
}

impl World for BookWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

pub fn write(book: &Book, table_of_contents: bool, page: PageSize) -> Result<Vec<u8>, String> {
    let fonts: Vec<Font> = crate::fonts::LITERATA
        .iter()
        .flat_map(|data| Font::iter(Bytes::new(data.to_vec())))
        .collect();
    let mut files = HashMap::new();
    for (i, image) in book.images.iter().enumerate() {
        let path = VirtualPath::new(image_name(book, i)).map_err(|e| format!("{e:?}"))?;
        files.insert(
            RootedPath::new(VirtualRoot::Project, path).intern(),
            Bytes::new(image.bytes.clone()),
        );
    }
    let world = BookWorld {
        library: LazyHash::new(Library::builder().build()),
        book: LazyHash::new(FontBook::from_fonts(&fonts)),
        fonts,
        source: Source::detached(markup(book, table_of_contents, page)),
        files,
    };
    let document: PagedDocument = typst::compile(&world).output.map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        format!("Could not lay out the PDF: {}", messages.join("; "))
    })?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        format!("Could not write the PDF: {}", messages.join("; "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_is_never_interpreted_as_markup() {
        assert_eq!(string("a \"b\" \\ #c\nd"), "\"a \\\"b\\\" \\\\ #c d\"");
        let markup = markup(&crate::export::sample_book(), true, PageSize::A5);
        assert!(markup.contains("lang: \"pt\", region: \"BR\""));
        assert!(markup.contains("#heading(level: 1)[#\"Tide & Harbor\"]"));
        assert!(markup.contains("#emph[#\"rope\"]#\".\"#footnote[#\"A note.\"]"));
        assert!(markup.contains("#outline(title: \"Sumário\", depth: 1)"));
    }

    #[test]
    fn pdf_compiles() {
        let pdf = write(&crate::export::sample_book(), true, PageSize::Trade).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
