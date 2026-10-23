//! Export the manuscript, entirely in Rust: chapters are parsed into a small
//! document tree ([`book`]) and written as DOCX, EPUB, PDF, ODT, or HTML.
use crate::i18n::t;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub mod book;
mod docx;
mod epub;
mod html;
mod odt;
mod pdf;
mod xml;

pub use book::Source;
use book::{Book, Meta};

/// Optional files in the project root that customise the export.
pub const METADATA_FILE: &str = "metadata.yaml";
pub const REFERENCE_DOCX: &str = "reference.docx";
pub const REFERENCE_ODT: &str = "reference.odt";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Docx,
    Epub,
    Pdf,
    Odt,
    Html,
}

impl Format {
    pub const ALL: [Format; 5] = [
        Format::Docx,
        Format::Epub,
        Format::Pdf,
        Format::Odt,
        Format::Html,
    ];

    /// The message that names it; its `.hint` says what it is for.
    fn key(self) -> &'static str {
        match self {
            Format::Docx => "format-docx",
            Format::Epub => "format-epub",
            Format::Pdf => "format-pdf",
            Format::Odt => "format-odt",
            Format::Html => "format-html",
        }
    }

    pub fn label(self) -> String {
        t!(self.key())
    }

    pub fn hint(self) -> String {
        t!(&format!("{}.hint", self.key()))
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Docx => "docx",
            Format::Epub => "epub",
            Format::Pdf => "pdf",
            Format::Odt => "odt",
            Format::Html => "html",
        }
    }

    /// Formats with pages (EPUB and HTML reflow).
    pub fn has_pages(self) -> bool {
        matches!(self, Format::Docx | Format::Pdf | Format::Odt)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PageSize {
    /// 148 × 210 mm, a common novel size.
    #[default]
    A5,
    A4,
    Letter,
    /// 6 × 9 in, the usual trade paperback.
    Trade,
}

impl PageSize {
    pub const ALL: [PageSize; 4] = [
        PageSize::A5,
        PageSize::Trade,
        PageSize::A4,
        PageSize::Letter,
    ];

    pub fn label(self) -> String {
        t!(match self {
            PageSize::A5 => "page-a5",
            PageSize::A4 => "page-a4",
            PageSize::Letter => "page-letter",
            PageSize::Trade => "page-trade",
        })
    }

    /// Width and height in millimetres.
    pub fn millimetres(self) -> (f64, f64) {
        match self {
            PageSize::A5 => (148.0, 210.0),
            PageSize::A4 => (210.0, 297.0),
            PageSize::Letter => (215.9, 279.4),
            PageSize::Trade => (152.4, 228.6),
        }
    }

    /// Small pages use a smaller body font and margins.
    pub fn is_small(self) -> bool {
        matches!(self, PageSize::A5 | PageSize::Trade)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    pub format: Format,
    pub author: String,
    /// Language of the manuscript, for hyphenation and e-readers ("en", "pt-BR").
    /// Empty means "use metadata.yaml, or English".
    pub language: String,
    pub page_size: PageSize,
    pub table_of_contents: bool,
    /// Book titles by project folder.
    pub titles: BTreeMap<String, String>,
    /// Folder of the last export.
    pub last_directory: Option<PathBuf>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: Format::Docx,
            author: String::new(),
            language: String::new(),
            page_size: PageSize::A5,
            table_of_contents: false,
            titles: BTreeMap::new(),
            last_directory: None,
        }
    }
}

/// What the author chose in the export window.
#[derive(Clone, Debug)]
pub struct Options {
    pub format: Format,
    pub title: String,
    pub author: String,
    pub language: String,
    pub page_size: PageSize,
    pub table_of_contents: bool,
}

/// A finished export, not yet written to disk.
pub struct Output {
    pub bytes: Vec<u8>,
    /// Things the author should know (missing images, ignored metadata, ...).
    pub warnings: Vec<String>,
}

/// Build the book from the chapters (in reading order) and write it in the
/// chosen format. Reads `metadata.yaml` and reference documents from `project`.
pub fn export(options: &Options, sources: &[Source], project: &Path) -> Result<Output, String> {
    let mut warnings = Vec::new();
    // Values typed in the export window win over metadata.yaml.
    let mut meta = Meta {
        title: options.title.trim().to_string(),
        author: options.author.trim().to_string(),
        language: options.language.trim().to_string(),
        ..Meta::default()
    };
    let metadata = project.join(METADATA_FILE);
    if metadata.is_file() {
        match std::fs::read_to_string(&metadata) {
            Ok(yaml) => {
                if let Err(e) = book::read_metadata(&yaml, &mut meta) {
                    warnings.push(t!(
                        "export-metadata-ignored",
                        file = METADATA_FILE,
                        error = e.to_string()
                    ));
                }
            }
            Err(e) => warnings.push(t!(
                "cannot-read",
                path = METADATA_FILE,
                error = e.to_string()
            )),
        }
    }
    if meta.title.is_empty() {
        meta.title = title_from_folder(project);
    }
    let mut book = Book::build(meta, sources, project);
    if book.chapters.is_empty() {
        return Err(t!("export-nothing"));
    }
    warnings.append(&mut book.warnings);

    let reference = |name: &str| -> Option<Vec<u8>> {
        let path = project.join(name);
        path.is_file().then(|| std::fs::read(path).ok()).flatten()
    };
    let zip_error = |e: zip::result::ZipError| t!("export-write-failed", error = e.to_string());
    let bytes = match options.format {
        Format::Html => html::page(&book, options.table_of_contents).into_bytes(),
        Format::Epub => epub::write(&book, options.table_of_contents).map_err(zip_error)?,
        Format::Odt => {
            let bytes = odt::write(&book, options.table_of_contents, options.page_size)
                .map_err(zip_error)?;
            match reference(REFERENCE_ODT) {
                Some(reference) => match xml::read_entry(&reference, "styles.xml") {
                    Some(styles) => xml::replace_entries(&bytes, &[("styles.xml", styles)])
                        .map_err(zip_error)?,
                    None => {
                        warnings.push(t!("export-no-styles", file = REFERENCE_ODT));
                        bytes
                    }
                },
                None => bytes,
            }
        }
        Format::Docx => {
            let bytes = docx::write(&book, options.table_of_contents, options.page_size)?;
            match reference(REFERENCE_DOCX) {
                Some(reference) => match xml::read_entry(&reference, "word/styles.xml") {
                    Some(styles) => xml::replace_entries(&bytes, &[("word/styles.xml", styles)])
                        .map_err(zip_error)?,
                    None => {
                        warnings.push(t!("export-no-styles", file = REFERENCE_DOCX));
                        bytes
                    }
                },
                None => bytes,
            }
        }
        Format::Pdf => pdf::write(&book, options.table_of_contents, options.page_size)?,
    };
    Ok(Output { bytes, warnings })
}

/// "the-salt-road" → "The Salt Road".
pub fn title_from_folder(project: &Path) -> String {
    let name = project
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    name.split(['-', '_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A file name for the export: "The Salt Road" → "The Salt Road.docx".
pub fn default_file_name(title: &str, format: Format) -> String {
    let clean: String = title
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let clean = clean.trim();
    let stem = if clean.is_empty() {
        "manuscript"
    } else {
        clean
    };
    format!("{stem}.{}", format.extension())
}

/// Open a file or folder with the desktop's default application.
pub fn open_with_system(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = crate::process::command("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = crate::process::command("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = crate::process::command("xdg-open");
    command.arg(path).spawn().map(|_| ())
}

/// Where to put the export by default.
pub fn default_directory(project: &Path, last: Option<&PathBuf>) -> PathBuf {
    last.filter(|dir| dir.is_dir())
        .cloned()
        .unwrap_or_else(|| project.to_path_buf())
}

#[cfg(test)]
pub fn sample_book() -> Book {
    let sources = [
        Source {
            title: "One".into(),
            text: "# Tide & Harbor\n\nTide & *rope*.[^n]\n\n***\n\nAfter.\n\n[^n]: A note.".into(),
        },
        Source {
            title: "Two".into(),
            text: "Second chapter.".into(),
        },
    ];
    let meta = Meta {
        title: "Salt & Sea".into(),
        author: "Ana".into(),
        language: "pt-BR".into(),
        ..Meta::default()
    };
    Book::build(meta, &sources, Path::new("/nonexistent"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(
            title_from_folder(Path::new("/x/the-salt-road")),
            "The Salt Road"
        );
        assert_eq!(default_file_name("A: B/C?", Format::Epub), "A BC.epub");
        assert_eq!(default_file_name("  ", Format::Pdf), "manuscript.pdf");
    }

    #[test]
    fn export_reads_metadata_and_prefers_typed_values() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(
            project.path().join(METADATA_FILE),
            "title: From Yaml\nauthor: Yaml Author\nlang: pt-BR\n",
        )
        .unwrap();
        let options = Options {
            format: Format::Html,
            title: " Typed ".into(),
            author: String::new(),
            language: String::new(),
            page_size: PageSize::A5,
            table_of_contents: false,
        };
        let sources = [Source {
            title: "One".into(),
            text: "Text ![x](missing.png)".into(),
        }];
        let out = export(&options, &sources, project.path()).unwrap();
        let html = String::from_utf8(out.bytes).unwrap();
        assert!(html.contains("<title>Typed</title>"));
        assert!(html.contains("Yaml Author"));
        assert!(html.contains("lang=\"pt-BR\""));
        assert_eq!(out.warnings.len(), 1);

        assert!(export(&options, &[], project.path()).is_err());
    }

    #[test]
    fn every_format_exports() {
        let project = tempfile::tempdir().unwrap();
        let sources = [Source {
            title: "One".into(),
            text: "# Start\n\nPlain *prose*.[^a]\n\n---\n\nMore.\n\n[^a]: Note.".into(),
        }];
        for format in Format::ALL {
            let options = Options {
                format,
                title: "Book".into(),
                author: "Me".into(),
                language: "en".into(),
                page_size: PageSize::A5,
                table_of_contents: true,
            };
            let out = export(&options, &sources, project.path()).unwrap();
            assert!(out.bytes.len() > 100, "{format:?}");
            if format == Format::Pdf {
                assert!(out.bytes.starts_with(b"%PDF"));
            }
        }
    }
}
