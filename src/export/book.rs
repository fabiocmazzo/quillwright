//! The manuscript as a small document tree, shared by every output format.
//! Built from the chapter files with `pulldown-cmark`, the same parser the
//! editor uses.
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Code(String),
    Link(String, Vec<Inline>),
    /// Index into [`Book::images`], or `None` when the file could not be read.
    Image {
        image: Option<usize>,
        alt: String,
    },
    /// Index into [`Book::notes`].
    Note(usize),
    LineBreak,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Para(Vec<Inline>),
    /// Level 2–6 headings inside a chapter (level 1 starts a chapter).
    Heading(u8, Vec<Inline>),
    Quote(Vec<Block>),
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Code(String),
    SceneBreak,
    Table {
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub title: Vec<Inline>,
    pub blocks: Vec<Block>,
}

impl Chapter {
    pub fn plain_title(&self) -> String {
        plain(&self.title)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub bytes: Vec<u8>,
    /// "png", "jpeg", or "gif".
    pub kind: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Image {
    pub fn mime(&self) -> &'static str {
        match self.kind {
            "png" => "image/png",
            "gif" => "image/gif",
            _ => "image/jpeg",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self.kind {
            "jpeg" => "jpg",
            other => other,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    pub title: String,
    pub subtitle: String,
    pub author: String,
    /// BCP 47 language tag, for example "en" or "pt-BR".
    pub language: String,
    pub date: String,
    pub rights: String,
    pub description: String,
}

impl Meta {
    /// ("pt", Some("BR")) from "pt-BR".
    pub fn language_parts(&self) -> (String, Option<String>) {
        let tag = self.language.trim();
        let mut parts = tag.split(['-', '_']);
        let lang = parts
            .next()
            .filter(|l| !l.is_empty())
            .unwrap_or("en")
            .to_lowercase();
        let region = parts
            .next()
            .filter(|r| r.len() == 2)
            .map(|r| r.to_uppercase());
        (lang, region)
    }

    /// The language tag to write into documents ("en" when unset).
    pub fn language_tag(&self) -> String {
        match self.language_parts() {
            (lang, Some(region)) => format!("{lang}-{region}"),
            (lang, None) => lang,
        }
    }

    /// The localized title for a table of contents.
    pub fn contents_label(&self) -> &'static str {
        match self.language_parts().0.as_str() {
            "pt" => "Sumário",
            "es" => "Índice",
            "fr" => "Table des matières",
            "de" => "Inhalt",
            "it" => "Indice",
            "nl" => "Inhoud",
            "zh" => "目录",
            "ja" => "目次",
            _ => "Contents",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub meta: Meta,
    pub chapters: Vec<Chapter>,
    pub notes: Vec<Vec<Block>>,
    pub images: Vec<Image>,
    /// Problems worth telling the author about (missing images, ...).
    pub warnings: Vec<String>,
}

/// One chapter file, in reading order.
pub struct Source {
    /// Used when the file does not start with a level-1 heading.
    pub title: String,
    pub text: String,
}

impl Book {
    /// Parse the chapter files. Images are looked up next to the chapters and
    /// in the project folder.
    pub fn build(meta: Meta, sources: &[Source], project: &Path) -> Book {
        let mut book = Book {
            meta,
            ..Book::default()
        };
        let mut images = ImageCache::new(project);
        for source in sources {
            let text = source.text.trim_start_matches('\u{feff}');
            let mut parsed = parse(text, &mut book.notes, &mut images);
            // A file may hold several chapters (several level-1 headings). Text
            // before the first heading belongs to a chapter named after the file.
            let untitled = parsed.first().is_some_and(|c| c.title.is_empty());
            if untitled {
                if parsed[0].blocks.is_empty() && parsed.len() > 1 {
                    parsed.remove(0);
                } else {
                    parsed[0].title = vec![Inline::Text(source.title.clone())];
                }
            }
            book.chapters.extend(parsed);
        }
        book.images = images.images;
        book.warnings = images.warnings;
        book
    }
}

/// The plain text of some inlines (titles, alt text, table of contents).
pub fn plain(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) | Inline::Code(t) => out.push_str(t),
            Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) | Inline::Link(_, c) => {
                out.push_str(&plain(c))
            }
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::LineBreak => out.push(' '),
            Inline::Note(_) => {}
        }
    }
    out
}

/// A paragraph that is just one image is laid out as a centered figure.
pub fn sole_image(inlines: &[Inline]) -> Option<(Option<usize>, &str)> {
    let mut found = None;
    for inline in inlines {
        match inline {
            Inline::Image { image, alt } if found.is_none() => found = Some((*image, alt.as_str())),
            Inline::Text(t) if t.trim().is_empty() => {}
            Inline::LineBreak => {}
            _ => return None,
        }
    }
    found
}

/// Notes referenced in these blocks, in reading order (each once).
pub fn notes_in(blocks: &[Block]) -> Vec<usize> {
    fn inlines(list: &[Inline], out: &mut Vec<usize>) {
        for inline in list {
            match inline {
                Inline::Note(n) if !out.contains(n) => out.push(*n),
                Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) | Inline::Link(_, c) => {
                    inlines(c, out)
                }
                _ => {}
            }
        }
    }
    fn walk(blocks: &[Block], out: &mut Vec<usize>) {
        for block in blocks {
            match block {
                Block::Para(i) | Block::Heading(_, i) => inlines(i, out),
                Block::Quote(b) => walk(b, out),
                Block::List { items, .. } => items.iter().for_each(|item| walk(item, out)),
                Block::Table { header, rows } => {
                    header.iter().for_each(|cell| inlines(cell, out));
                    rows.iter().flatten().for_each(|cell| inlines(cell, out));
                }
                Block::Code(_) | Block::SceneBreak => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(blocks, &mut out);
    out
}

// ----- Images ------------------------------------------------------------------

struct ImageCache {
    roots: Vec<PathBuf>,
    by_path: HashMap<String, Option<usize>>,
    images: Vec<Image>,
    warnings: Vec<String>,
}

impl ImageCache {
    fn new(project: &Path) -> Self {
        ImageCache {
            roots: vec![
                project.join(crate::manuscript::CHAPTERS_DIR),
                project.to_path_buf(),
            ],
            by_path: HashMap::new(),
            images: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn get(&mut self, src: &str) -> Option<usize> {
        if let Some(found) = self.by_path.get(src) {
            return *found;
        }
        let found = self.load(src);
        if found.is_none() {
            self.warnings.push(crate::i18n::t!(
                "export-image-skipped",
                file = src.to_string()
            ));
        }
        self.by_path.insert(src.to_string(), found);
        found
    }

    fn load(&mut self, src: &str) -> Option<usize> {
        if src.contains("://") {
            return None; // Export never downloads anything.
        }
        let decoded = src.replace("%20", " ");
        let path = Path::new(&decoded);
        let bytes = if path.is_absolute() {
            std::fs::read(path).ok()?
        } else {
            self.roots
                .iter()
                .find_map(|root| std::fs::read(root.join(path)).ok())?
        };
        let kind = match imagesize::image_type(&bytes).ok()? {
            imagesize::ImageType::Png => "png",
            imagesize::ImageType::Jpeg => "jpeg",
            imagesize::ImageType::Gif => "gif",
            _ => return None,
        };
        let size = imagesize::blob_size(&bytes).ok()?;
        self.images.push(Image {
            bytes,
            kind,
            width: size.width as u32,
            height: size.height as u32,
        });
        Some(self.images.len() - 1)
    }
}

// ----- Markdown parsing --------------------------------------------------------

fn options() -> Options {
    // Like the editor, plus typographic quotes and dashes for print.
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
}

struct Cursor<'a, 'n> {
    events: Vec<Event<'a>>,
    at: usize,
    notes: &'n mut Vec<Vec<Block>>,
    /// Footnote labels of this file → note index.
    labels: HashMap<String, usize>,
    images: &'n mut ImageCache,
}

impl<'a> Cursor<'a, '_> {
    fn peek(&self) -> Option<&Event<'a>> {
        self.events.get(self.at)
    }

    fn next(&mut self) -> Option<Event<'a>> {
        let event = self.events.get(self.at).cloned();
        self.at += 1;
        event
    }

    fn note(&mut self, label: &str) -> usize {
        if let Some(&n) = self.labels.get(label) {
            return n;
        }
        self.notes.push(Vec::new());
        let n = self.notes.len() - 1;
        self.labels.insert(label.to_string(), n);
        n
    }

    /// Blocks until the end of the enclosing container (or the input).
    fn blocks(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();
        while let Some(event) = self.peek() {
            if is_inline(event) {
                let inlines = self.inlines_run();
                if !inlines.is_empty() {
                    blocks.push(Block::Para(inlines));
                }
                continue;
            }
            let Some(event) = self.next() else { break };
            match event {
                Event::End(_) => break,
                Event::Rule => blocks.push(Block::SceneBreak),
                Event::Start(tag) => match tag {
                    Tag::Paragraph => {
                        let inlines = self.inlines();
                        if !inlines.is_empty() {
                            blocks.push(Block::Para(inlines));
                        }
                    }
                    Tag::Heading { level, .. } => {
                        let inlines = self.inlines();
                        blocks.push(Block::Heading(heading_level(level), inlines));
                    }
                    Tag::BlockQuote => blocks.push(Block::Quote(self.blocks())),
                    Tag::CodeBlock(_) => {
                        let mut code = String::new();
                        while let Some(event) = self.next() {
                            match event {
                                Event::Text(t) => code.push_str(&t),
                                Event::End(_) => break,
                                _ => {}
                            }
                        }
                        blocks.push(Block::Code(code.trim_end_matches('\n').to_string()));
                    }
                    Tag::List(start) => {
                        let mut items = Vec::new();
                        while let Some(event) = self.next() {
                            match event {
                                Event::Start(Tag::Item) => items.push(self.blocks()),
                                Event::End(_) => break,
                                _ => {}
                            }
                        }
                        blocks.push(Block::List { start, items });
                    }
                    Tag::FootnoteDefinition(label) => {
                        let n = self.note(&label);
                        let body = self.blocks();
                        self.notes[n] = body;
                    }
                    Tag::Table(_) => blocks.push(self.table()),
                    // Raw HTML and metadata are not part of the book.
                    _ => self.skip(),
                },
                _ => {}
            }
        }
        blocks
    }

    fn table(&mut self) -> Block {
        let mut header = Vec::new();
        let mut rows = Vec::new();
        while let Some(event) = self.next() {
            match event {
                Event::Start(Tag::TableHead) => header = self.cells(),
                Event::Start(Tag::TableRow) => rows.push(self.cells()),
                Event::End(_) => break,
                _ => {}
            }
        }
        Block::Table { header, rows }
    }

    fn cells(&mut self) -> Vec<Vec<Inline>> {
        let mut cells = Vec::new();
        while let Some(event) = self.next() {
            match event {
                Event::Start(Tag::TableCell) => cells.push(self.inlines()),
                Event::End(_) => break,
                _ => {}
            }
        }
        cells
    }

    /// Skip a container we do not render, including everything inside it.
    fn skip(&mut self) {
        let mut depth = 1;
        while let Some(event) = self.next() {
            match event {
                Event::Start(_) => depth += 1,
                Event::End(_) => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    /// Inline content until the end of the enclosing element.
    fn inlines(&mut self) -> Vec<Inline> {
        let mut out = Vec::new();
        while let Some(event) = self.next() {
            if matches!(event, Event::End(_)) {
                break;
            }
            self.inline(event, &mut out);
        }
        merge_text(out)
    }

    /// Loose inline content at block level (tight list items): stop before the
    /// next block or the end of the container.
    fn inlines_run(&mut self) -> Vec<Inline> {
        let mut out = Vec::new();
        while let Some(event) = self.peek() {
            if !is_inline(event) {
                break;
            }
            let event = self.next().unwrap();
            self.inline(event, &mut out);
        }
        merge_text(out)
    }

    fn inline(&mut self, event: Event<'a>, out: &mut Vec<Inline>) {
        match event {
            Event::Text(t) => out.push(Inline::Text(t.to_string())),
            Event::Code(t) => out.push(Inline::Code(t.to_string())),
            Event::SoftBreak => out.push(Inline::Text(" ".into())),
            Event::HardBreak => out.push(Inline::LineBreak),
            Event::FootnoteReference(label) => {
                let n = self.note(&label);
                out.push(Inline::Note(n));
            }
            Event::TaskListMarker(done) => {
                out.push(Inline::Text(if done { "☑ " } else { "☐ " }.into()))
            }
            Event::Start(tag) => match tag {
                Tag::Emphasis => out.push(Inline::Emph(self.inlines())),
                Tag::Strong => out.push(Inline::Strong(self.inlines())),
                Tag::Strikethrough => out.push(Inline::Strike(self.inlines())),
                Tag::Link { dest_url, .. } => {
                    let content = self.inlines();
                    out.push(Inline::Link(dest_url.to_string(), content));
                }
                Tag::Image { dest_url, .. } => {
                    let alt = plain(&self.inlines());
                    let image = self.images.get(&dest_url);
                    out.push(Inline::Image { image, alt });
                }
                _ => self.skip(),
            },
            // Inline HTML is dropped; everything else is plain text already.
            _ => {}
        }
    }
}

fn is_inline(event: &Event) -> bool {
    match event {
        Event::Text(_)
        | Event::Code(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::FootnoteReference(_)
        | Event::TaskListMarker(_)
        | Event::InlineHtml(_) => true,
        Event::Start(tag) => matches!(
            tag,
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
        ),
        _ => false,
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Join adjacent text runs (the parser splits text at every special character).
fn merge_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match (out.last_mut(), inline) {
            (Some(Inline::Text(last)), Inline::Text(t)) => last.push_str(&t),
            (_, inline) => out.push(inline),
        }
    }
    out
}

/// Parse one file into chapters, split at level-1 headings.
fn parse(text: &str, notes: &mut Vec<Vec<Block>>, images: &mut ImageCache) -> Vec<Chapter> {
    let mut cursor = Cursor {
        events: Parser::new_ext(text, options()).collect(),
        at: 0,
        notes,
        labels: HashMap::new(),
        images,
    };
    let blocks = cursor.blocks();
    let mut chapters = vec![Chapter {
        title: Vec::new(),
        blocks: Vec::new(),
    }];
    for block in blocks {
        match block {
            Block::Heading(1, title) => chapters.push(Chapter {
                title,
                blocks: Vec::new(),
            }),
            block => chapters.last_mut().unwrap().blocks.push(block),
        }
    }
    if chapters[0].blocks.is_empty() && chapters.len() > 1 {
        chapters.remove(0);
    }
    chapters
}

/// Read the optional `metadata.yaml` into `meta`, without overriding values
/// the author typed in the export window.
pub fn read_metadata(yaml: &str, meta: &mut Meta) -> Result<(), String> {
    use yaml_rust2::{Yaml, YamlLoader};
    let docs = YamlLoader::load_from_str(yaml).map_err(|e| e.to_string())?;
    let Some(doc) = docs.first() else {
        return Ok(());
    };
    let text = |key: &str| -> Option<String> {
        match &doc[key] {
            Yaml::String(s) => Some(s.trim().to_string()),
            Yaml::Integer(n) => Some(n.to_string()),
            Yaml::Real(r) => Some(r.clone()),
            Yaml::Array(items) => Some(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Yaml::String(s) => Some(s.trim().to_string()),
                        // pandoc style: author: [{name: ...}]
                        Yaml::Hash(_) => item["name"].as_str().map(str::to_string),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            _ => None,
        }
        .filter(|s| !s.is_empty())
    };
    let fill = |slot: &mut String, value: Option<String>| {
        if slot.trim().is_empty() {
            if let Some(value) = value {
                *slot = value;
            }
        }
    };
    fill(&mut meta.title, text("title"));
    fill(&mut meta.subtitle, text("subtitle"));
    fill(&mut meta.author, text("author"));
    fill(
        &mut meta.language,
        text("lang").or_else(|| text("language")),
    );
    fill(&mut meta.date, text("date"));
    fill(&mut meta.rights, text("rights"));
    fill(
        &mut meta.description,
        text("description").or_else(|| text("abstract")),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(files: &[(&str, &str)]) -> Book {
        let sources: Vec<Source> = files
            .iter()
            .map(|(title, text)| Source {
                title: title.to_string(),
                text: text.to_string(),
            })
            .collect();
        Book::build(Meta::default(), &sources, Path::new("/nonexistent"))
    }

    fn text(s: &str) -> Inline {
        Inline::Text(s.into())
    }

    #[test]
    fn chapters_come_from_headings_or_file_titles() {
        let b = book(&[
            ("The Harbor", "# The Harbor\n\nTide.\n"),
            ("Untitled", "Just prose.\n\n***\n\nMore."),
            ("Two in one", "# One\n\nA.\n\n# Two\n\nB."),
        ]);
        let titles: Vec<_> = b.chapters.iter().map(|c| c.plain_title()).collect();
        assert_eq!(titles, ["The Harbor", "Untitled", "One", "Two"]);
        assert_eq!(
            b.chapters[1].blocks,
            [
                Block::Para(vec![text("Just prose.")]),
                Block::SceneBreak,
                Block::Para(vec![text("More.")])
            ]
        );
    }

    #[test]
    fn inline_formatting_and_smart_punctuation() {
        let b = book(&[(
            "x",
            "# T\n\n\"Hi,\" she said -- *really* **so**\nnext ~~no~~ `code` [l](http://x)",
        )]);
        let Block::Para(inlines) = &b.chapters[0].blocks[0] else {
            panic!()
        };
        assert_eq!(
            inlines,
            &[
                text("“Hi,” she said – "),
                Inline::Emph(vec![text("really")]),
                text(" "),
                Inline::Strong(vec![text("so")]),
                text(" next "),
                Inline::Strike(vec![text("no")]),
                text(" "),
                Inline::Code("code".into()),
                text(" "),
                Inline::Link("http://x".into(), vec![text("l")]),
            ]
        );
    }

    #[test]
    fn notes_lists_quotes_tables_and_code() {
        let b = book(&[(
            "x",
            "# T\n\nA note.[^a] Again.[^a]\n\n[^a]: The *note*.\n\n\
             - one\n- two\n\n3. three\n\n> quoted\n\n| h |\n|---|\n| c |\n\n```\nlet x;\n```\n",
        )]);
        assert_eq!(b.notes.len(), 1);
        assert_eq!(
            b.notes[0],
            [Block::Para(vec![
                text("The "),
                Inline::Emph(vec![text("note")]),
                text(".")
            ])]
        );
        let blocks = &b.chapters[0].blocks;
        assert_eq!(notes_in(blocks), [0]);
        assert!(matches!(&blocks[1], Block::List { start: None, items } if items.len() == 2));
        assert!(matches!(&blocks[2], Block::List { start: Some(3), .. }));
        assert!(matches!(&blocks[3], Block::Quote(_)));
        assert!(
            matches!(&blocks[4], Block::Table { header, rows } if header.len() == 1 && rows.len() == 1)
        );
        assert_eq!(blocks[5], Block::Code("let x;".into()));
    }

    #[test]
    fn html_and_missing_images() {
        let b = book(&[("x", "# T\n\n<div>raw</div>\n\n![A map](map.png)\n")]);
        assert_eq!(b.chapters[0].blocks.len(), 1);
        let Block::Para(inlines) = &b.chapters[0].blocks[0] else {
            panic!()
        };
        assert_eq!(sole_image(inlines), Some((None, "A map")));
        assert_eq!(b.warnings, ["Image not included: map.png"]);
    }

    #[test]
    fn images_are_read_from_the_project() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("chapters")).unwrap();
        // A 1×1 PNG.
        let png = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, 0x49, 0x48, 0x44, 0x52,
            0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1f, 0x15, 0xc4, 0x89,
        ];
        std::fs::write(temp.path().join("chapters/dot.png"), png).unwrap();
        let sources = [Source {
            title: "x".into(),
            text: "# T\n\n![dot](dot.png) and ![dot](dot.png)".into(),
        }];
        let b = Book::build(Meta::default(), &sources, temp.path());
        assert_eq!(b.images.len(), 1);
        assert_eq!((b.images[0].width, b.images[0].height), (1, 1));
        assert_eq!(b.images[0].mime(), "image/png");
        assert!(b.warnings.is_empty());
    }

    #[test]
    fn metadata_fills_only_empty_fields() {
        let mut meta = Meta {
            title: "Typed title".into(),
            ..Meta::default()
        };
        read_metadata(
            "title: Ignored\nsubtitle: A novel\nauthor:\n  - Ana\n  - name: Bia\nlang: pt-BR\ndate: 2026\n",
            &mut meta,
        )
        .unwrap();
        assert_eq!(meta.title, "Typed title");
        assert_eq!(meta.subtitle, "A novel");
        assert_eq!(meta.author, "Ana, Bia");
        assert_eq!(meta.date, "2026");
        assert_eq!(meta.language_parts(), ("pt".into(), Some("BR".into())));
        assert_eq!(meta.contents_label(), "Sumário");
        assert!(read_metadata("title: [unclosed", &mut meta).is_err());
    }
}
