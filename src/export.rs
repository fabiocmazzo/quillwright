//! Export the manuscript with pandoc: chapters joined in reading order, then
//! converted to DOCX, EPUB, PDF, ODT, or HTML.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Page breaks before chapters and centered scene breaks, per output format.
pub const FILTER: &str = include_str!("../assets/export/manuscript.lua");

/// Optional files in the project root that customise the export.
pub const METADATA_FILE: &str = "metadata.yaml";
pub const REFERENCE_DOCX: &str = "reference.docx";
pub const REFERENCE_ODT: &str = "reference.odt";

/// PDF engines pandoc can use, in order of preference.
const PDF_ENGINES: &[&str] = &[
    "typst",
    "tectonic",
    "xelatex",
    "lualatex",
    "pdflatex",
    "weasyprint",
    "wkhtmltopdf",
];

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

    pub fn label(self) -> &'static str {
        match self {
            Format::Docx => "Word (.docx)",
            Format::Epub => "EPUB e-book (.epub)",
            Format::Pdf => "PDF (.pdf)",
            Format::Odt => "OpenDocument (.odt)",
            Format::Html => "Web page (.html)",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Format::Docx => "For editors, agents, and Google Docs",
            Format::Epub => "For e-readers and beta readers",
            Format::Pdf => "For printing and sharing a fixed layout",
            Format::Odt => "For LibreOffice",
            Format::Html => "A single self-contained page",
        }
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
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    /// The pandoc executable.
    pub pandoc: String,
    /// Force a PDF engine instead of picking the first one installed.
    pub pdf_engine: Option<String>,
    pub format: Format,
    pub author: String,
    pub table_of_contents: bool,
    /// Book titles by project folder.
    pub titles: BTreeMap<String, String>,
    /// Folder of the last export.
    pub last_directory: Option<PathBuf>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            pandoc: "pandoc".into(),
            pdf_engine: None,
            format: Format::Docx,
            author: String::new(),
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
    pub table_of_contents: bool,
    pub pdf_engine: Option<String>,
}

/// One chapter, in reading order.
pub struct Chapter {
    pub title: String,
    pub text: String,
}

/// Join chapters into one Markdown document. A chapter that does not start
/// with a level-1 heading gets one from its title, so every chapter begins on
/// its own page and appears in the table of contents.
pub fn assemble(chapters: &[Chapter]) -> String {
    let mut book = String::new();
    for chapter in chapters {
        let text = chapter.text.trim_start_matches('\u{feff}').trim();
        if !starts_with_chapter_heading(text) {
            book.push_str(&format!("# {}\n\n", chapter.title));
        }
        book.push_str(text);
        book.push_str("\n\n");
    }
    book
}

fn starts_with_chapter_heading(text: &str) -> bool {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.starts_with("# ") || line.trim() == "#")
}

/// Pandoc arguments. The manuscript is read from stdin.
pub fn arguments(options: &Options, output: &Path, filter: &Path, project: &Path) -> Vec<String> {
    let mut args: Vec<String> = vec![
        // CommonMark like the editor, with smart quotes, footnotes, and tables.
        // YAML blocks are off: a "---" scene break must never start one.
        "--from=commonmark_x-yaml_metadata_block".into(),
        format!("--lua-filter={}", filter.display()),
        format!("--output={}", output.display()),
        format!(
            "--resource-path={}{}{}",
            project.join("chapters").display(),
            if cfg!(windows) { ";" } else { ":" },
            project.display()
        ),
    ];
    let metadata = project.join(METADATA_FILE);
    if metadata.is_file() {
        args.push(format!("--metadata-file={}", metadata.display()));
    }
    // Values typed in the export window win over metadata.yaml.
    if !options.title.trim().is_empty() {
        args.push(format!("--metadata=title:{}", options.title.trim()));
    }
    if !options.author.trim().is_empty() {
        args.push(format!("--metadata=author:{}", options.author.trim()));
    }
    if options.table_of_contents {
        args.push("--toc".into());
        args.push("--toc-depth=1".into());
    }
    match options.format {
        Format::Docx => {
            let reference = project.join(REFERENCE_DOCX);
            if reference.is_file() {
                args.push(format!("--reference-doc={}", reference.display()));
            }
        }
        Format::Odt => {
            let reference = project.join(REFERENCE_ODT);
            if reference.is_file() {
                args.push(format!("--reference-doc={}", reference.display()));
            }
        }
        Format::Html => {
            args.push("--standalone".into());
            args.push("--embed-resources".into());
        }
        Format::Pdf => {
            if let Some(engine) = &options.pdf_engine {
                args.push(format!("--pdf-engine={engine}"));
            }
        }
        Format::Epub => {}
    }
    args
}

/// The installed pandoc version, or None if it cannot be run.
pub fn pandoc_version(pandoc: &str) -> Option<String> {
    let output = crate::process::command(pandoc)
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    output
        .status
        .success()
        .then(|| text.lines().next().unwrap_or("pandoc").trim().to_string())
}

/// PDF engines installed on this machine, best first.
pub fn installed_pdf_engines() -> Vec<String> {
    PDF_ENGINES
        .iter()
        .filter(|name| on_path(name))
        .map(|name| name.to_string())
        .collect()
}

fn on_path(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file() || (cfg!(windows) && candidate.with_extension("exe").is_file())
    })
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
mod tests {
    use super::*;

    fn chapter(title: &str, text: &str) -> Chapter {
        Chapter {
            title: title.into(),
            text: text.into(),
        }
    }

    #[test]
    fn assemble_keeps_order_and_adds_missing_headings() {
        let book = assemble(&[
            chapter("The Harbor", "# The Harbor\n\nTide.\n"),
            chapter("Untitled", "Just prose.\n\n***\n\nMore."),
            chapter("Ice", "\n\n# Ice\nCold."),
        ]);
        assert_eq!(
            book,
            "# The Harbor\n\nTide.\n\n# Untitled\n\nJust prose.\n\n***\n\nMore.\n\n# Ice\nCold.\n\n"
        );
    }

    #[test]
    fn a_level_two_heading_is_not_a_chapter_heading() {
        let book = assemble(&[chapter("One", "## Scene\n\nText")]);
        assert!(book.starts_with("# One\n\n## Scene"));
    }

    #[test]
    fn arguments_include_options_and_project_files() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(METADATA_FILE), "lang: pt-BR\n").unwrap();
        std::fs::write(project.path().join(REFERENCE_DOCX), "").unwrap();
        let options = Options {
            format: Format::Docx,
            title: " The Salt Road ".into(),
            author: "".into(),
            table_of_contents: true,
            pdf_engine: Some("typst".into()),
        };
        let args = arguments(
            &options,
            Path::new("/out/book.docx"),
            Path::new("/f.lua"),
            project.path(),
        );
        assert!(args.contains(&"--output=/out/book.docx".to_string()));
        assert!(args.contains(&"--metadata=title:The Salt Road".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("--metadata=author")));
        assert!(args.contains(&"--toc".to_string()));
        assert!(args.iter().any(|a| a.starts_with("--metadata-file=")));
        assert!(args.iter().any(|a| a.starts_with("--reference-doc=")));
        // The PDF engine only matters for PDF.
        assert!(!args.iter().any(|a| a.starts_with("--pdf-engine")));

        let pdf = Options {
            format: Format::Pdf,
            ..options
        };
        let args = arguments(
            &pdf,
            Path::new("/out/book.pdf"),
            Path::new("/f.lua"),
            project.path(),
        );
        assert!(args.contains(&"--pdf-engine=typst".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("--reference-doc=")));
    }

    #[test]
    fn names() {
        assert_eq!(
            title_from_folder(Path::new("/x/the-salt-road")),
            "The Salt Road"
        );
        assert_eq!(default_file_name("A: B/C?", Format::Epub), "A BC.epub");
        assert_eq!(default_file_name("  ", Format::Pdf), "manuscript.pdf");
    }
}
