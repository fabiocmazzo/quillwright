//! EPUB 3 e-books, with an EPUB 2 table of contents (toc.ncx) for older readers.
use super::book::{plain, Book};
use super::html::{Target, Writer, CSS};
use super::xml::{escape, Package};

fn xhtml(book: &Book, title: &str, body: &str) -> String {
    let lang = escape(&book.meta.language_tag());
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" \
         lang=\"{lang}\" xml:lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\"/>\n\
         <title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n\
         </head>\n<body>\n{body}</body>\n</html>\n",
        escape(title)
    )
}

/// A stable identifier for the book, so re-exports update the same entry in
/// e-reader libraries.
fn identifier(book: &Book) -> String {
    let mut a: u64 = 0xcbf2_9ce4_8422_2325;
    let mut b: u64 = 0x8422_2325_cbf2_9ce4;
    for byte in format!("{}\u{0}{}", book.meta.title, book.meta.author).bytes() {
        a = (a ^ byte as u64).wrapping_mul(0x100_0000_01b3);
        b = (b ^ byte as u64)
            .wrapping_mul(0x100_0000_01b3)
            .rotate_left(5);
    }
    let hex = format!("{a:016x}{b:016x}");
    format!(
        "urn:uuid:{}-{}-4{}-a{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

fn chapter_file(index: usize) -> String {
    format!("chapter-{:03}.xhtml", index + 1)
}

pub fn write(book: &Book, table_of_contents: bool) -> zip::result::ZipResult<Vec<u8>> {
    let meta = &book.meta;
    let writer = Writer {
        book,
        target: Target::Epub,
    };
    let mut package = Package::new("application/epub+zip")?;
    package.add(
        "META-INF/container.xml",
        br#"<?xml version="1.0" encoding="utf-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="EPUB/package.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#,
    )?;
    package.add("EPUB/style.css", CSS.as_bytes())?;

    let mut title_page = String::new();
    writer.title_page(&mut title_page);
    package.add(
        "EPUB/title.xhtml",
        xhtml(book, &meta.title, &title_page).as_bytes(),
    )?;

    for (i, chapter) in book.chapters.iter().enumerate() {
        let mut body = String::from("<section epub:type=\"chapter\">\n");
        writer.chapter(chapter, &format!("chapter-{}", i + 1), &mut body);
        body.push_str("</section>\n");
        package.add(
            &format!("EPUB/{}", chapter_file(i)),
            xhtml(book, &chapter.plain_title(), &body).as_bytes(),
        )?;
    }
    for (i, image) in book.images.iter().enumerate() {
        package.add(
            &format!("EPUB/images/image-{}.{}", i + 1, image.extension()),
            &image.bytes,
        )?;
    }

    // Navigation document (EPUB 3). It is also the visible table of contents.
    let contents = meta.contents_label();
    let mut nav =
        format!("<nav epub:type=\"toc\" id=\"toc\" class=\"toc\">\n<h1>{contents}</h1>\n<ol>\n");
    for (i, chapter) in book.chapters.iter().enumerate() {
        nav.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            chapter_file(i),
            escape(&chapter.plain_title())
        ));
    }
    nav.push_str("</ol>\n</nav>\n<nav epub:type=\"landmarks\" hidden=\"hidden\">\n<ol>\n");
    nav.push_str("<li><a epub:type=\"titlepage\" href=\"title.xhtml\">Title page</a></li>\n");
    if let Some(_first) = book.chapters.first() {
        nav.push_str(&format!(
            "<li><a epub:type=\"bodymatter\" href=\"{}\">Start</a></li>\n",
            chapter_file(0)
        ));
    }
    nav.push_str("</ol>\n</nav>\n");
    package.add("EPUB/nav.xhtml", xhtml(book, contents, &nav).as_bytes())?;

    let id = identifier(book);
    let mut ncx = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n\
         <head><meta name=\"dtb:uid\" content=\"{}\"/></head>\n\
         <docTitle><text>{}</text></docTitle>\n<navMap>\n",
        escape(&id),
        escape(&meta.title)
    );
    for (i, chapter) in book.chapters.iter().enumerate() {
        ncx.push_str(&format!(
            "<navPoint id=\"nav-{n}\" playOrder=\"{n}\"><navLabel><text>{}</text></navLabel>\
             <content src=\"{}\"/></navPoint>\n",
            escape(&plain(&chapter.title)),
            chapter_file(i),
            n = i + 1
        ));
    }
    ncx.push_str("</navMap>\n</ncx>\n");
    package.add("EPUB/toc.ncx", ncx.as_bytes())?;

    // Package document.
    let modified = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let mut opf = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\" xml:lang=\"{lang}\">\n\
         <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
         <dc:identifier id=\"uid\">{}</dc:identifier>\n<dc:title>{}</dc:title>\n\
         <dc:language>{lang}</dc:language>\n<meta property=\"dcterms:modified\">{modified}</meta>\n",
        escape(&id),
        escape(&meta.title),
        lang = escape(&meta.language_tag()),
    );
    for (element, value) in [
        ("dc:creator", &meta.author),
        ("dc:date", &meta.date),
        ("dc:rights", &meta.rights),
        ("dc:description", &meta.description),
    ] {
        if !value.trim().is_empty() {
            opf.push_str(&format!("<{element}>{}</{element}>\n", escape(value)));
        }
    }
    opf.push_str(
        "<meta name=\"generator\" content=\"Quillwright\"/>\n</metadata>\n<manifest>\n\
         <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n\
         <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n\
         <item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n\
         <item id=\"title\" href=\"title.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
    );
    for i in 0..book.chapters.len() {
        opf.push_str(&format!(
            "<item id=\"chapter-{}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n",
            i + 1,
            chapter_file(i)
        ));
    }
    for (i, image) in book.images.iter().enumerate() {
        opf.push_str(&format!(
            "<item id=\"image-{n}\" href=\"images/image-{n}.{}\" media-type=\"{}\"/>\n",
            image.extension(),
            image.mime(),
            n = i + 1
        ));
    }
    opf.push_str("</manifest>\n<spine toc=\"ncx\">\n<itemref idref=\"title\"/>\n");
    if table_of_contents {
        opf.push_str("<itemref idref=\"nav\"/>\n");
    }
    for i in 0..book.chapters.len() {
        opf.push_str(&format!("<itemref idref=\"chapter-{}\"/>\n", i + 1));
    }
    opf.push_str("</spine>\n</package>\n");
    package.add("EPUB/package.opf", opf.as_bytes())?;
    package.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::xml::read_entry;

    #[test]
    fn epub_contains_chapters_navigation_and_metadata() {
        let book = crate::export::sample_book();
        let bytes = write(&book, true).unwrap();
        let text = |name: &str| String::from_utf8(read_entry(&bytes, name).unwrap()).unwrap();
        let opf = text("EPUB/package.opf");
        assert!(opf.contains("<dc:title>Salt &amp; Sea</dc:title>"));
        assert!(opf.contains("<dc:language>pt-BR</dc:language>"));
        assert!(opf.contains("<dc:creator>Ana</dc:creator>"));
        assert!(opf.contains("<itemref idref=\"nav\"/>"));
        assert!(opf.contains("properties=\"nav\""));
        let nav = text("EPUB/nav.xhtml");
        assert!(nav.contains("<h1>Sumário</h1>"));
        assert!(nav.contains("<a href=\"chapter-002.xhtml\">Two</a>"));
        let chapter = text("EPUB/chapter-001.xhtml");
        assert!(chapter.starts_with("<?xml"));
        assert!(chapter.contains("epub:type=\"noteref\""));
        assert!(chapter.contains("<aside epub:type=\"footnote\" id=\"note-1\">"));
        assert!(text("EPUB/toc.ncx").contains("playOrder=\"2\""));
        // Same book, same identifier.
        assert_eq!(identifier(&book), identifier(&book));
        assert!(identifier(&book).starts_with("urn:uuid:"));

        let without = write(&book, false).unwrap();
        let opf = String::from_utf8(read_entry(&without, "EPUB/package.opf").unwrap()).unwrap();
        assert!(!opf.contains("<itemref idref=\"nav\"/>"));
    }
}
