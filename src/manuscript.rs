//! Writer-facing view of a project folder: the main documents (chapters), sheets
//! (characters), notes, word counts. Which folders these are comes from the project's
//! kind (`crate::kinds`). Everything is derived from plain Markdown files; nothing is
//! stored.
use crate::kinds::{Kind, Section};
use std::{
    cmp::Ordering,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Where export looks for images, besides the project folder. The Novel kind's documents
/// folder; export does not read the kind yet.
pub const CHAPTERS_DIR: &str = "chapters";
/// Reference material: PDFs, images, clippings. Read only in the app.
pub const RESEARCH_DIR: &str = "research";

/// Count prose words, ignoring Markdown punctuation and HTML comments.
pub fn word_count(text: &str) -> usize {
    let mut count = 0;
    let mut rest = text;
    loop {
        let (visible, next) = match rest.find("<!--") {
            Some(start) => {
                let after = &rest[start + 4..];
                let end = after.find("-->").map_or(after.len(), |e| e + 3);
                (&rest[..start], Some(&after[end.min(after.len())..]))
            }
            None => (rest, None),
        };
        count += visible
            .split_whitespace()
            .filter(|token| token.chars().any(char::is_alphanumeric))
            .count();
        match next {
            Some(next) => rest = next,
            None => return count,
        }
    }
}

/// Minutes to read aloud-ish at a typical silent reading pace.
pub fn reading_minutes(words: usize) -> usize {
    words.div_ceil(250)
}

/// Thousands separators for display: 12345 -> "12,345".
pub fn group(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// First Markdown heading, or a readable version of the file name.
pub fn title(text: &str, path: &Path) -> String {
    text.lines()
        .map(str::trim)
        .find_map(|line| {
            let stripped = line.trim_start_matches('#');
            (line.starts_with('#') && stripped.starts_with(' ') && !stripped.trim().is_empty())
                .then(|| stripped.trim().trim_end_matches('#').trim().to_string())
        })
        .unwrap_or_else(|| pretty_name(path))
}

fn pretty_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_');
    let words: Vec<String> = stem
        .split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        })
        .collect();
    if words.is_empty() {
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        words.join(" ")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub title: String,
    pub words: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Index {
    /// The kind's main documents (a novel's chapters), in reading order.
    pub documents: Vec<Entry>,
    /// The kind's sheets (a novel's characters).
    pub sheets: Vec<Entry>,
    /// Markdown files at the project root (world rules, outlines, AGENT.md, ...).
    pub notes: Vec<Entry>,
    /// Every file under `research/`, sorted by path (hidden files skipped).
    pub research: Vec<PathBuf>,
    pub has_documents_dir: bool,
    pub has_sheets_dir: bool,
    pub has_research_dir: bool,
}

impl Index {
    pub fn build(root: &Path, kind: &Kind) -> Self {
        let documents_dir = root.join(&kind.documents.folder);
        let sheets_dir = root.join(&kind.sheets.folder);
        let research_dir = root.join(RESEARCH_DIR);
        let mut research = Vec::new();
        collect_files(&research_dir, 0, &mut research);
        research.sort_by(|a, b| natural_path_cmp(a, b));
        Index {
            documents: entries(&documents_dir),
            sheets: entries(&sheets_dir),
            notes: entries(root),
            has_documents_dir: documents_dir.is_dir(),
            has_sheets_dir: sheets_dir.is_dir(),
            research,
            has_research_dir: research_dir.is_dir(),
        }
    }

    /// Words across all documents, using `live` text for the open file if given.
    pub fn manuscript_words(&self, open: Option<(&Path, usize)>) -> usize {
        self.documents
            .iter()
            .map(|e| match open {
                Some((path, words)) if path == e.path => words,
                _ => e.words,
            })
            .sum()
    }
}

/// Compare paths the way people read them: folder by folder, numbers by value
/// (`note-2` before `note-10`), letters without regard to case.
pub fn natural_path_cmp(a: &Path, b: &Path) -> Ordering {
    let mut left = a.components();
    let mut right = b.components();
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = natural_cmp(
                    &x.as_os_str().to_string_lossy(),
                    &y.as_os_str().to_string_lossy(),
                );
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// Natural order for one name. Ties (such as `01` and `1`, or `A` and `a`) fall back
/// to plain string order, so the result is always total and stable.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a, b);
    while !x.is_empty() && !y.is_empty() {
        let digits_x = x.chars().next().is_some_and(|c| c.is_ascii_digit());
        let digits_y = y.chars().next().is_some_and(|c| c.is_ascii_digit());
        let order = if digits_x && digits_y {
            let run_x = run(x, true);
            let run_y = run(y, true);
            let trimmed_x = run_x.trim_start_matches('0');
            let trimmed_y = run_y.trim_start_matches('0');
            let order = trimmed_x
                .len()
                .cmp(&trimmed_y.len())
                .then_with(|| trimmed_x.cmp(trimmed_y));
            x = &x[run_x.len()..];
            y = &y[run_y.len()..];
            order
        } else {
            let cx = x.chars().next().unwrap();
            let cy = y.chars().next().unwrap();
            x = &x[cx.len_utf8()..];
            y = &y[cy.len_utf8()..];
            cx.to_lowercase().cmp(cy.to_lowercase())
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    x.len().cmp(&y.len()).then_with(|| a.cmp(b))
}

/// The leading run of ASCII digits (or of other characters) in `s`.
fn run(s: &str, digits: bool) -> &str {
    let end = s
        .find(|c: char| c.is_ascii_digit() != digits)
        .unwrap_or(s.len());
    &s[..end]
}

/// Files below `dir`, a few levels deep, skipping hidden names (`.git`, `.DS_Store`).
fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    const MAX_DEPTH: usize = 4;
    const MAX_FILES: usize = 2000;
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        if out.len() >= MAX_FILES {
            return;
        }
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if depth < MAX_DEPTH {
                collect_files(&path, depth + 1, out);
            }
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

fn entries(dir: &Path) -> Vec<Entry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_markdown(p))
        .collect();
    paths.sort_by(|a, b| natural_path_cmp(a, b));
    paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap_or_default();
            Entry {
                title: title(&text, &path),
                words: word_count(&text),
                path,
            }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    /// Character index of the start of the heading line.
    pub char_index: usize,
}

/// ATX headings outside fenced code blocks.
pub fn outline(text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut chars = 0;
    let mut fence: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
        } else if trimmed.starts_with("```") {
            fence = Some("```");
        } else if trimmed.starts_with("~~~") {
            fence = Some("~~~");
        } else if line.starts_with('#') {
            let level = line.chars().take_while(|&c| c == '#').count();
            let rest = &trimmed[level..];
            if level <= 6 && rest.starts_with(' ') && !rest.trim().is_empty() {
                headings.push(Heading {
                    level,
                    text: rest.trim().trim_end_matches('#').trim().to_string(),
                    char_index: chars,
                });
            }
        }
        chars += line.chars().count();
    }
    headings
}

fn create_file(path: &Path, content: &str) -> io::Result<PathBuf> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(path.to_owned())
}

/// Create a numbered document (`chapters/NNN-chapter-N.md` for a novel) after the highest
/// existing number.
pub fn new_document(root: &Path, section: &Section) -> io::Result<PathBuf> {
    let dir = root.join(&section.folder);
    fs::create_dir_all(&dir)?;
    let mut highest = 0;
    let mut width = 3;
    for entry in fs::read_dir(&dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<usize>() {
            highest = highest.max(n);
            width = width.max(digits.len());
        }
    }
    let mut n = highest + 1;
    loop {
        let name = section.new_name.replace("{n}", &n.to_string());
        let title = section.new_title.replace("{n}", &n.to_string());
        let path = dir.join(format!("{n:0width$}-{name}.md"));
        match create_file(&path, &format!("# {title}\n\n")) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => n += 1,
            result => return result,
        }
    }
}

/// Create a sheet (`characters/new-character[-N].md` for a novel) from the kind's sheet
/// template.
pub fn new_sheet(root: &Path, kind: &Kind) -> io::Result<PathBuf> {
    let section = &kind.sheets;
    let dir = root.join(&section.folder);
    fs::create_dir_all(&dir)?;
    let template = section
        .template
        .as_deref()
        .and_then(|path| kind.file(path))
        .unwrap_or_default();
    let body = template
        .split_once('\n')
        .map_or("", |(_, rest)| rest)
        .trim_start_matches('\n');
    let body = body
        .split_once("\n\n")
        .filter(|(first, _)| first.starts_with('>') || first.trim().is_empty())
        .map_or(body, |(_, rest)| rest);
    let content = format!("# {}\n\n{body}", section.new_title);
    for n in 1.. {
        let name = if n == 1 {
            format!("{}.md", section.new_name)
        } else {
            format!("{}-{n}.md", section.new_name)
        };
        match create_file(&dir.join(name), &content) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            result => return result,
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_names_naturally() {
        let mut names = vec![
            "note-10.txt",
            "Note-2.txt",
            "note-1.txt",
            "10-end.md",
            "2-middle.md",
            "1-start.md",
            "001-opening.md",
            "notes.txt",
        ];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            vec![
                "001-opening.md",
                "1-start.md",
                "2-middle.md",
                "10-end.md",
                "note-1.txt",
                "Note-2.txt",
                "note-10.txt",
                "notes.txt",
            ]
        );
        assert_eq!(natural_cmp("a", "a"), Ordering::Equal);
        assert_ne!(natural_cmp("01", "1"), Ordering::Equal);
        // Folders sort as a whole: "maps/x" stays with "maps", before "maps2".
        let mut paths = vec![
            PathBuf::from("maps2/a.png"),
            PathBuf::from("maps/b.png"),
            PathBuf::from("maps.txt"),
        ];
        paths.sort_by(|a, b| natural_path_cmp(a, b));
        assert_eq!(
            paths,
            vec![
                PathBuf::from("maps/b.png"),
                PathBuf::from("maps.txt"),
                PathBuf::from("maps2/a.png"),
            ]
        );
    }

    #[test]
    fn lists_research_files_recursively_without_hidden_ones() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("research/maps")).unwrap();
        fs::write(root.join("research/notes.txt"), "x").unwrap();
        fs::write(root.join("research/maps/harbor.png"), "x").unwrap();
        fs::write(root.join("research/.DS_Store"), "x").unwrap();
        let index = Index::build(root, crate::kinds::default_kind());
        assert!(index.has_research_dir);
        let names: Vec<_> = index
            .research
            .iter()
            .map(|p| p.strip_prefix(root.join("research")).unwrap().to_path_buf())
            .collect();
        assert_eq!(
            names,
            vec![PathBuf::from("maps/harbor.png"), PathBuf::from("notes.txt")]
        );
        assert!(
            !Index::build(&root.join("research"), crate::kinds::default_kind()).has_research_dir
        );
    }

    #[test]
    fn counts_prose_words_only() {
        assert_eq!(word_count("# Opening\n\nShe *ran* — fast.\n- item"), 5);
        assert_eq!(word_count("<!-- template note -->\n# Title"), 1);
        assert_eq!(word_count("a <!-- x --> b <!-- unterminated"), 2);
        assert_eq!(word_count("Olá, coração! 你好"), 3);
        assert_eq!(word_count(""), 0);
    }

    #[test]
    fn titles_and_grouping() {
        let path = Path::new("chapters/002-the-crossing.md");
        assert_eq!(title("# The Crossing ##\ntext", path), "The Crossing");
        assert_eq!(title("no heading", path), "The Crossing");
        assert_eq!(
            title("#hashtag", Path::new("x/maria_silva.md")),
            "Maria Silva"
        );
        assert_eq!(group(0), "0");
        assert_eq!(group(1234567), "1,234,567");
        assert_eq!(reading_minutes(1), 1);
        assert_eq!(reading_minutes(500), 2);
    }

    #[test]
    fn outline_skips_code_fences() {
        let text = "# One\ntext\n```\n# not a heading\n```\n## Scene ✦\n";
        let headings = outline(text);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "Scene ✦");
        assert_eq!(headings[1].char_index, text.find("## Scene").unwrap());
    }

    #[test]
    fn index_and_new_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("chapters")).unwrap();
        fs::write(root.join("chapters/001-opening.md"), "# Opening\n\none two").unwrap();
        fs::write(root.join("chapters/notes.txt"), "ignored").unwrap();
        fs::write(root.join("world-rules.md"), "# World rules").unwrap();
        let chapter = new_document(root, &crate::kinds::default_kind().documents).unwrap();
        assert!(chapter.ends_with("chapters/002-chapter-2.md"));
        let character = new_sheet(root, crate::kinds::default_kind()).unwrap();
        let second = new_sheet(root, crate::kinds::default_kind()).unwrap();
        assert!(second.ends_with("characters/new-character-2.md"));
        assert!(fs::read_to_string(&character)
            .unwrap()
            .starts_with("# New character\n\n## Identity"));

        let index = Index::build(root, crate::kinds::default_kind());
        let titles: Vec<_> = index.documents.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["Opening", "Chapter 2"]);
        assert_eq!(index.sheets.len(), 2);
        assert_eq!(index.notes.len(), 1);
        assert_eq!(index.manuscript_words(None), 5);
        let open = root.join("chapters/001-opening.md");
        assert_eq!(index.manuscript_words(Some((&open, 100))), 102);
    }
}
