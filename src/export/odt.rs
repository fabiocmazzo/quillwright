//! OpenDocument text (LibreOffice), written by hand. Named styles match the
//! ones LibreOffice and pandoc use, so a `reference.odt` can restyle the book.
use super::book::{sole_image, Block, Book, Inline};
use super::xml::{escape, Package};
use super::PageSize;
use std::fmt::Write;

const NAMESPACES: &str = "xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" \
xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" \
xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" \
xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" \
xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\" \
xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" \
office:version=\"1.3\"";

/// Text with runs of spaces and tabs kept, as ODF collapses them otherwise.
fn text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut spaces = 0;
    let flush = |out: &mut String, spaces: &mut usize| {
        if *spaces > 0 {
            out.push(' ');
            if *spaces > 1 {
                let _ = write!(out, "<text:s text:c=\"{}\"/>", *spaces - 1);
            }
            *spaces = 0;
        }
    };
    for ch in value.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => {
                flush(&mut out, &mut spaces);
                out.push_str("<text:tab/>");
            }
            '\n' => {
                flush(&mut out, &mut spaces);
                out.push_str("<text:line-break/>");
            }
            c => {
                flush(&mut out, &mut spaces);
                out.push_str(&escape(&c.to_string()));
            }
        }
    }
    flush(&mut out, &mut spaces);
    out
}

fn image_path(book: &Book, index: usize) -> String {
    format!(
        "Pictures/image-{}.{}",
        index + 1,
        book.images[index].extension()
    )
}

struct Writer<'a> {
    book: &'a Book,
    /// Widest an image may be, in centimetres.
    text_width: f64,
    out: String,
    frames: usize,
    tables: usize,
}

impl Writer<'_> {
    fn span(&mut self, style: &str, content: &[Inline]) {
        let _ = write!(self.out, "<text:span text:style-name=\"{style}\">");
        self.inlines(content);
        self.out.push_str("</text:span>");
    }

    fn frame(&mut self, index: usize) {
        let image = &self.book.images[index];
        let mut width = image.width.max(1) as f64 / 96.0 * 2.54;
        let mut height = image.height.max(1) as f64 / 96.0 * 2.54;
        if width > self.text_width {
            height *= self.text_width / width;
            width = self.text_width;
        }
        self.frames += 1;
        let _ = write!(
            self.out,
            "<draw:frame draw:style-name=\"fr_Image\" draw:name=\"Image{}\" text:anchor-type=\"as-char\" \
             svg:width=\"{width:.2}cm\" svg:height=\"{height:.2}cm\"><draw:image xlink:href=\"{}\" \
             xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame>",
            self.frames,
            image_path(self.book, index)
        );
    }

    fn inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            match inline {
                Inline::Text(t) => self.out.push_str(&text(t)),
                Inline::Emph(c) => self.span("T_Em", c),
                Inline::Strong(c) => self.span("T_Strong", c),
                Inline::Strike(c) => self.span("T_Strike", c),
                Inline::Code(t) => {
                    let _ = write!(
                        self.out,
                        "<text:span text:style-name=\"T_Code\">{}</text:span>",
                        text(t)
                    );
                }
                Inline::Link(url, c) => {
                    let _ = write!(
                        self.out,
                        "<text:a xlink:type=\"simple\" xlink:href=\"{}\" text:style-name=\"Internet_20_link\">",
                        escape(url)
                    );
                    self.inlines(c);
                    self.out.push_str("</text:a>");
                }
                Inline::Image { image: Some(i), .. } => self.frame(*i),
                Inline::Image { image: None, alt } => self.out.push_str(&text(alt)),
                Inline::Note(n) => {
                    let _ = write!(
                        self.out,
                        "<text:note text:id=\"ftn{n}\" text:note-class=\"footnote\">\
                         <text:note-citation>{}</text:note-citation><text:note-body>",
                        n + 1
                    );
                    let note = self.book.notes.get(*n).cloned().unwrap_or_default();
                    for block in &note {
                        self.out.push_str("<text:p text:style-name=\"Footnote\">");
                        match block {
                            Block::Para(i) | Block::Heading(_, i) => self.inlines(i),
                            Block::Code(t) => self.out.push_str(&text(t)),
                            _ => {}
                        }
                        self.out.push_str("</text:p>");
                    }
                    self.out.push_str("</text:note-body></text:note>");
                }
                Inline::LineBreak => self.out.push_str("<text:line-break/>"),
            }
        }
    }

    fn paragraph(&mut self, style: &str, inlines: &[Inline]) {
        let _ = write!(self.out, "<text:p text:style-name=\"{style}\">");
        self.inlines(inlines);
        self.out.push_str("</text:p>\n");
    }

    fn blocks(&mut self, blocks: &[Block], mut first: bool, quote: usize) {
        for block in blocks {
            match block {
                Block::Para(inlines) => {
                    if let Some((Some(i), _)) = sole_image(inlines) {
                        self.out.push_str("<text:p text:style-name=\"Figure\">");
                        self.frame(i);
                        self.out.push_str("</text:p>\n");
                    } else {
                        let style = match (quote, first) {
                            (0, true) => "First_20_paragraph".to_string(),
                            (0, false) => "Text_20_body".to_string(),
                            (depth, _) => format!("P_Quote{}", depth.min(4)),
                        };
                        self.paragraph(&style, inlines);
                    }
                    first = false;
                }
                Block::Heading(level, inlines) => {
                    let level = (*level).clamp(2, 3);
                    let _ = write!(
                        self.out,
                        "<text:h text:style-name=\"Heading_20_{level}\" text:outline-level=\"{level}\">"
                    );
                    self.inlines(inlines);
                    self.out.push_str("</text:h>\n");
                    first = true;
                }
                Block::Quote(inner) => {
                    self.blocks(inner, true, quote + 1);
                    first = true;
                }
                Block::List { start, items } => {
                    let style = if start.is_some() {
                        "L_Number"
                    } else {
                        "L_Bullet"
                    };
                    let _ = write!(self.out, "<text:list text:style-name=\"{style}\">");
                    for (n, item) in items.iter().enumerate() {
                        match start {
                            Some(s) if n == 0 => {
                                let _ =
                                    write!(self.out, "<text:list-item text:start-value=\"{s}\">");
                            }
                            _ => self.out.push_str("<text:list-item>"),
                        }
                        for block in item {
                            match block {
                                Block::Para(inlines) => self.paragraph("List_20_Contents", inlines),
                                other => self.blocks(std::slice::from_ref(other), true, quote),
                            }
                        }
                        self.out.push_str("</text:list-item>");
                    }
                    self.out.push_str("</text:list>\n");
                    first = true;
                }
                Block::Code(code) => {
                    for line in code.trim_end().lines() {
                        let _ = writeln!(
                            self.out,
                            "<text:p text:style-name=\"Preformatted_20_Text\">{}</text:p>",
                            text(line)
                        );
                    }
                    first = true;
                }
                Block::SceneBreak => {
                    self.out.push_str("<text:p text:style-name=\"Scene_20_Break\">*<text:s text:c=\"3\"/>*<text:s text:c=\"3\"/>*</text:p>\n");
                    first = true;
                }
                Block::Table { header, rows } => {
                    let columns = header
                        .len()
                        .max(rows.iter().map(Vec::len).max().unwrap_or(0))
                        .max(1);
                    self.tables += 1;
                    let _ = write!(
                        self.out,
                        "<table:table table:name=\"Table{}\"><table:table-column table:number-columns-repeated=\"{columns}\"/>",
                        self.tables
                    );
                    let row = |writer: &mut Self, cells: &[Vec<Inline>], style: &str| {
                        writer.out.push_str("<table:table-row>");
                        for c in 0..columns {
                            writer
                                .out
                                .push_str("<table:table-cell office:value-type=\"string\">");
                            let cell = cells.get(c).cloned().unwrap_or_default();
                            writer.paragraph(style, &cell);
                            writer.out.push_str("</table:table-cell>");
                        }
                        writer.out.push_str("</table:table-row>");
                    };
                    if !header.is_empty() {
                        self.out.push_str("<table:table-header-rows>");
                        row(self, header, "Table_20_Heading");
                        self.out.push_str("</table:table-header-rows>");
                    }
                    for cells in rows {
                        row(self, cells, "Table_20_Contents");
                    }
                    self.out.push_str("</table:table>\n");
                    first = true;
                }
            }
        }
    }
}

fn content(book: &Book, table_of_contents: bool, text_width: f64) -> String {
    let meta = &book.meta;
    let mut quotes = String::new();
    for depth in 1..=4 {
        let _ = write!(
            quotes,
            "<style:style style:name=\"P_Quote{depth}\" style:family=\"paragraph\" style:parent-style-name=\"Quotations\">\
             <style:paragraph-properties fo:margin-left=\"{depth}cm\"/></style:style>"
        );
    }
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-content {NAMESPACES}>\n\
         <office:automatic-styles>\
         <style:style style:name=\"T_Em\" style:family=\"text\"><style:text-properties fo:font-style=\"italic\" style:font-style-asian=\"italic\" style:font-style-complex=\"italic\"/></style:style>\
         <style:style style:name=\"T_Strong\" style:family=\"text\"><style:text-properties fo:font-weight=\"bold\" style:font-weight-asian=\"bold\" style:font-weight-complex=\"bold\"/></style:style>\
         <style:style style:name=\"T_Strike\" style:family=\"text\"><style:text-properties style:text-line-through-style=\"solid\" style:text-line-through-type=\"single\"/></style:style>\
         <style:style style:name=\"T_Code\" style:family=\"text\"><style:text-properties style:font-name=\"Liberation Mono\"/></style:style>\
         <style:style style:name=\"P_TitlePage\" style:family=\"paragraph\" style:parent-style-name=\"Title\" style:master-page-name=\"First_20_Page\"/>\
         <style:style style:name=\"P_Chapter\" style:family=\"paragraph\" style:parent-style-name=\"Heading_20_1\"><style:paragraph-properties fo:break-before=\"page\"/></style:style>\
         <style:style style:name=\"P_FirstChapter\" style:family=\"paragraph\" style:parent-style-name=\"Heading_20_1\" style:master-page-name=\"Standard\"><style:paragraph-properties style:page-number=\"1\"/></style:style>\
         {quotes}\
         <style:style style:name=\"fr_Image\" style:family=\"graphic\"><style:graphic-properties style:vertical-pos=\"top\" style:vertical-rel=\"baseline\"/></style:style>\
         <text:list-style style:name=\"L_Bullet\"><text:list-level-style-bullet text:level=\"1\" text:bullet-char=\"•\"><style:list-level-properties text:list-level-position-and-space-mode=\"label-alignment\"><style:list-level-label-alignment text:label-followed-by=\"listtab\" text:list-tab-stop-position=\"1cm\" fo:text-indent=\"-0.5cm\" fo:margin-left=\"1cm\"/></style:list-level-properties></text:list-level-style-bullet></text:list-style>\
         <text:list-style style:name=\"L_Number\"><text:list-level-style-number text:level=\"1\" style:num-suffix=\".\" style:num-format=\"1\"><style:list-level-properties text:list-level-position-and-space-mode=\"label-alignment\"><style:list-level-label-alignment text:label-followed-by=\"listtab\" text:list-tab-stop-position=\"1cm\" fo:text-indent=\"-0.6cm\" fo:margin-left=\"1cm\"/></style:list-level-properties></text:list-level-style-number></text:list-style>\
         </office:automatic-styles>\n<office:body>\n<office:text>\n"
    );

    let _ = writeln!(
        out,
        "<text:p text:style-name=\"P_TitlePage\">{}</text:p>",
        text(&meta.title)
    );
    for (style, value) in [
        ("Subtitle", &meta.subtitle),
        ("Author", &meta.author),
        ("Date", &meta.date),
        ("Date", &meta.rights),
    ] {
        if !value.trim().is_empty() {
            let _ = writeln!(
                out,
                "<text:p text:style-name=\"{style}\">{}</text:p>",
                text(value)
            );
        }
    }

    if table_of_contents {
        let label = escape(meta.contents_label());
        let _ = write!(
            out,
            "<text:table-of-content text:protected=\"true\" text:name=\"Table of Contents1\">\
             <text:table-of-content-source text:outline-level=\"1\">\
             <text:index-title-template text:style-name=\"Contents_20_Heading\">{label}</text:index-title-template>\
             <text:table-of-content-entry-template text:outline-level=\"1\" text:style-name=\"Contents_20_1\">\
             <text:index-entry-link-start/><text:index-entry-text/><text:index-entry-tab-stop style:type=\"right\" style:leader-char=\".\"/>\
             <text:index-entry-page-number/><text:index-entry-link-end/></text:table-of-content-entry-template>\
             </text:table-of-content-source><text:index-body>\
             <text:index-title text:name=\"Table of Contents1_Head\"><text:p text:style-name=\"Contents_20_Heading\">{label}</text:p></text:index-title>"
        );
        for chapter in &book.chapters {
            let _ = write!(
                out,
                "<text:p text:style-name=\"Contents_20_1\">{}</text:p>",
                text(&chapter.plain_title())
            );
        }
        out.push_str("</text:index-body></text:table-of-content>\n");
    }

    let mut writer = Writer {
        book,
        text_width,
        out,
        frames: 0,
        tables: 0,
    };
    for (i, chapter) in book.chapters.iter().enumerate() {
        // The first chapter restarts page numbering at 1 on the normal pages.
        let style = if i == 0 {
            "P_FirstChapter"
        } else {
            "P_Chapter"
        };
        let _ = write!(
            writer.out,
            "<text:h text:style-name=\"{style}\" text:outline-level=\"1\">"
        );
        writer.inlines(&chapter.title);
        writer.out.push_str("</text:h>\n");
        writer.blocks(&chapter.blocks, true, 0);
    }
    writer
        .out
        .push_str("</office:text>\n</office:body>\n</office:document-content>\n");
    writer.out
}

fn styles(book: &Book, page: PageSize) -> String {
    let (lang, region) = book.meta.language_parts();
    let country = region
        .map(|r| format!(" fo:country=\"{}\"", escape(&r)))
        .unwrap_or_default();
    let (width, height) = page.millimetres();
    let margin = if page.is_small() { 18.0 } else { 25.0 };
    let size = if page.is_small() { "11pt" } else { "12pt" };
    let paragraph = |name: &str, display: &str, parent: &str, extra: &str| {
        format!(
            "<style:style style:name=\"{name}\" style:display-name=\"{display}\" style:family=\"paragraph\" \
             style:parent-style-name=\"{parent}\">{extra}</style:style>"
        )
    };
    let centered = "<style:paragraph-properties fo:text-align=\"center\"/>";
    let named = [
        paragraph("Text_20_body", "Text body", "Standard",
            "<style:paragraph-properties fo:text-indent=\"1cm\" fo:margin-top=\"0cm\" fo:margin-bottom=\"0cm\"/>"),
        paragraph("First_20_paragraph", "First paragraph", "Text_20_body",
            "<style:paragraph-properties fo:text-indent=\"0cm\"/>"),
        paragraph("List_20_Contents", "List Contents", "Standard", ""),
        paragraph("Heading", "Heading", "Standard",
            "<style:paragraph-properties fo:keep-with-next=\"always\"/>"),
        "<style:style style:name=\"Heading_20_1\" style:display-name=\"Heading 1\" style:family=\"paragraph\" \
             style:parent-style-name=\"Heading\" style:next-style-name=\"First_20_paragraph\" style:default-outline-level=\"1\">\
             <style:paragraph-properties fo:text-align=\"center\" fo:margin-top=\"4cm\" fo:margin-bottom=\"1.2cm\"/>\
             <style:text-properties fo:font-size=\"170%\"/></style:style>".to_string(),
        "<style:style style:name=\"Heading_20_2\" style:display-name=\"Heading 2\" style:family=\"paragraph\" \
             style:parent-style-name=\"Heading\" style:next-style-name=\"First_20_paragraph\" style:default-outline-level=\"2\">\
             <style:paragraph-properties fo:margin-top=\"0.6cm\" fo:margin-bottom=\"0.2cm\"/>\
             <style:text-properties fo:font-size=\"125%\" fo:font-weight=\"bold\"/></style:style>".to_string(),
        "<style:style style:name=\"Heading_20_3\" style:display-name=\"Heading 3\" style:family=\"paragraph\" \
             style:parent-style-name=\"Heading\" style:next-style-name=\"First_20_paragraph\" style:default-outline-level=\"3\">\
             <style:paragraph-properties fo:margin-top=\"0.4cm\" fo:margin-bottom=\"0.2cm\"/>\
             <style:text-properties fo:font-weight=\"bold\" fo:font-style=\"italic\"/></style:style>".to_string(),
        paragraph("Title", "Title", "Standard",
            "<style:paragraph-properties fo:text-align=\"center\" fo:margin-top=\"6cm\" fo:margin-bottom=\"0.4cm\"/><style:text-properties fo:font-size=\"220%\"/>"),
        paragraph("Subtitle", "Subtitle", "Standard",
            "<style:paragraph-properties fo:text-align=\"center\"/><style:text-properties fo:font-size=\"130%\" fo:font-style=\"italic\"/>"),
        paragraph("Author", "Author", "Standard",
            "<style:paragraph-properties fo:text-align=\"center\" fo:margin-top=\"2.5cm\"/><style:text-properties fo:font-size=\"120%\"/>"),
        paragraph("Date", "Date", "Standard",
            "<style:paragraph-properties fo:text-align=\"center\" fo:margin-top=\"0.8cm\"/>"),
        paragraph("Quotations", "Quotations", "Standard",
            "<style:paragraph-properties fo:margin-left=\"1cm\" fo:margin-right=\"1cm\" fo:margin-top=\"0.2cm\" fo:margin-bottom=\"0.2cm\"/>"),
        paragraph("Preformatted_20_Text", "Preformatted Text", "Standard",
            "<style:paragraph-properties fo:line-height=\"100%\"/><style:text-properties style:font-name=\"Liberation Mono\" fo:font-size=\"90%\"/>"),
        paragraph("Scene_20_Break", "Scene Break", "Standard",
            "<style:paragraph-properties fo:text-align=\"center\" fo:margin-top=\"0.4cm\" fo:margin-bottom=\"0.4cm\"/>"),
        paragraph("Figure", "Figure", "Standard",
            "<style:paragraph-properties fo:text-align=\"center\" fo:margin-top=\"0.4cm\" fo:margin-bottom=\"0.4cm\"/>"),
        paragraph("Table_20_Contents", "Table Contents", "Standard", "<style:paragraph-properties fo:line-height=\"115%\"/>"),
        paragraph("Table_20_Heading", "Table Heading", "Table_20_Contents", "<style:text-properties fo:font-weight=\"bold\"/>"),
        paragraph("Footnote", "Footnote", "Standard",
            "<style:paragraph-properties fo:line-height=\"115%\"/><style:text-properties fo:font-size=\"85%\"/>"),
        paragraph("Footer", "Footer", "Standard", centered),
        paragraph("Contents_20_Heading", "Contents Heading", "Heading",
            "<style:paragraph-properties fo:text-align=\"center\" fo:break-before=\"page\" fo:margin-top=\"2cm\" fo:margin-bottom=\"1cm\"/><style:text-properties fo:font-size=\"150%\"/>"),
        format!("<style:style style:name=\"Contents_20_1\" style:display-name=\"Contents 1\" style:family=\"paragraph\" style:parent-style-name=\"Standard\">\
             <style:paragraph-properties><style:tab-stops><style:tab-stop style:position=\"{:.1}cm\" style:type=\"right\" style:leader-style=\"dotted\" style:leader-text=\".\"/></style:tab-stops></style:paragraph-properties></style:style>",
             (width - 2.0 * margin) / 10.0),
        "<style:style style:name=\"Internet_20_link\" style:display-name=\"Internet link\" style:family=\"text\">\
         <style:text-properties fo:color=\"#000080\" style:text-underline-style=\"solid\" style:text-underline-width=\"auto\" style:text-underline-color=\"font-color\"/></style:style>".to_string(),
    ]
    .concat();
    let outline: String = (1..=3)
        .map(|l| format!("<text:outline-level-style text:level=\"{l}\" style:num-format=\"\"/>"))
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-styles {NAMESPACES}>\n\
         <office:font-face-decls>\
         <style:font-face style:name=\"Liberation Serif\" svg:font-family=\"'Liberation Serif'\" style:font-family-generic=\"roman\" style:font-pitch=\"variable\"/>\
         <style:font-face style:name=\"Liberation Mono\" svg:font-family=\"'Liberation Mono'\" style:font-family-generic=\"modern\" style:font-pitch=\"fixed\"/>\
         </office:font-face-decls>\n<office:styles>\
         <style:default-style style:family=\"paragraph\"><style:paragraph-properties fo:hyphenation-ladder-count=\"no-limit\" style:writing-mode=\"page\"/>\
         <style:text-properties style:font-name=\"Liberation Serif\" fo:font-size=\"{size}\" fo:language=\"{}\"{country} fo:hyphenate=\"true\" \
         fo:hyphenation-remain-char-count=\"2\" fo:hyphenation-push-char-count=\"2\"/></style:default-style>\
         <style:style style:name=\"Standard\" style:family=\"paragraph\" style:class=\"text\"><style:paragraph-properties fo:line-height=\"150%\"/></style:style>\
         {named}\
         <text:outline-style style:name=\"Outline\">{outline}</text:outline-style>\
         <text:notes-configuration text:note-class=\"footnote\" style:num-format=\"1\" text:start-value=\"0\" text:footnotes-position=\"page\" text:start-numbering-at=\"document\"/>\
         </office:styles>\n<office:automatic-styles>\
         <style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"{width}mm\" fo:page-height=\"{height}mm\" \
         style:print-orientation=\"portrait\" fo:margin-top=\"{margin}mm\" fo:margin-bottom=\"{}mm\" fo:margin-left=\"{margin}mm\" fo:margin-right=\"{margin}mm\"/>\
         <style:footer-style><style:header-footer-properties fo:min-height=\"0cm\" fo:margin-top=\"0.4cm\"/></style:footer-style></style:page-layout>\
         </office:automatic-styles>\n<office:master-styles>\
         <style:master-page style:name=\"Standard\" style:page-layout-name=\"pm1\"><style:footer><text:p text:style-name=\"Footer\"><text:page-number text:select-page=\"current\"/></text:p></style:footer></style:master-page>\
         <style:master-page style:name=\"First_20_Page\" style:display-name=\"First Page\" style:page-layout-name=\"pm1\" style:next-style-name=\"Standard\"/>\
         </office:master-styles>\n</office:document-styles>\n",
        escape(&lang),
        margin * 0.7,
    )
}

fn meta_xml(book: &Book) -> String {
    let meta = &book.meta;
    let mut fields = format!(
        "<meta:generator>Quillwright</meta:generator><dc:title>{}</dc:title><dc:language>{}</dc:language>",
        escape(&meta.title),
        escape(&meta.language_tag())
    );
    if !meta.author.trim().is_empty() {
        let author = escape(&meta.author);
        let _ = write!(fields, "<meta:initial-creator>{author}</meta:initial-creator><dc:creator>{author}</dc:creator>");
    }
    if !meta.description.trim().is_empty() {
        let _ = write!(
            fields,
            "<dc:description>{}</dc:description>",
            escape(&meta.description)
        );
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-meta {NAMESPACES}><office:meta>{fields}</office:meta></office:document-meta>\n"
    )
}

pub fn write(
    book: &Book,
    table_of_contents: bool,
    page: PageSize,
) -> zip::result::ZipResult<Vec<u8>> {
    let (width, _) = page.millimetres();
    let margin = if page.is_small() { 18.0 } else { 25.0 };
    let text_width = (width - 2.0 * margin) / 10.0;

    let mut manifest = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\">\n\
         <manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.3\" manifest:media-type=\"application/vnd.oasis.opendocument.text\"/>\n\
         <manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\n\
         <manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/>\n\
         <manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/>\n",
    );
    for (i, image) in book.images.iter().enumerate() {
        let _ = writeln!(
            manifest,
            "<manifest:file-entry manifest:full-path=\"{}\" manifest:media-type=\"{}\"/>",
            image_path(book, i),
            image.mime()
        );
    }
    manifest.push_str("</manifest:manifest>\n");

    let mut package = Package::new("application/vnd.oasis.opendocument.text")?;
    package.add(
        "content.xml",
        content(book, table_of_contents, text_width).as_bytes(),
    )?;
    package.add("styles.xml", styles(book, page).as_bytes())?;
    package.add("meta.xml", meta_xml(book).as_bytes())?;
    for (i, image) in book.images.iter().enumerate() {
        package.add(&image_path(book, i), &image.bytes)?;
    }
    package.add("META-INF/manifest.xml", manifest.as_bytes())?;
    package.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::xml::read_entry;

    #[test]
    fn spaces_are_kept() {
        assert_eq!(
            text("a  b\tc <d>"),
            "a <text:s text:c=\"1\"/>b<text:tab/>c &lt;d&gt;"
        );
        assert_eq!(text("x y"), "x y");
    }

    #[test]
    fn odt_has_content_styles_and_notes() {
        let bytes = write(&crate::export::sample_book(), true, PageSize::A5).unwrap();
        let entry = |name: &str| String::from_utf8(read_entry(&bytes, name).unwrap()).unwrap();
        let content = entry("content.xml");
        assert!(content.contains("<text:h text:style-name=\"P_FirstChapter\" text:outline-level=\"1\">Tide &amp; Harbor</text:h>"));
        assert!(content.contains("<text:span text:style-name=\"T_Em\">rope</text:span>"));
        assert!(content
            .contains("<text:note-body><text:p text:style-name=\"Footnote\">A note.</text:p>"));
        assert!(content.contains(">Sumário</text:p>"));
        assert!(content.contains("<text:p text:style-name=\"Contents_20_1\">Two</text:p>"));
        let styles = entry("styles.xml");
        assert!(styles.contains("fo:language=\"pt\" fo:country=\"BR\""));
        assert!(styles.contains("fo:page-width=\"148mm\""));
        assert!(entry("meta.xml").contains("<dc:creator>Ana</dc:creator>"));
        assert!(entry("META-INF/manifest.xml").contains("content.xml"));
    }
}
