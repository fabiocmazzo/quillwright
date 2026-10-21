//! Project-wide search: chapters, characters, notes, and research (text, Markdown, and the
//! text of PDFs). One worker thread runs searches; a newer query stops the older one
//! between files. Matching ignores case and accents (see `reference::text::fold`).
use crate::reference::{pdf, text};
use std::{
    ops::Range,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread,
};

/// Hits kept per file (all are counted).
const HITS_PER_FILE: usize = 100;
/// Hits kept in total.
const MAX_HITS: usize = 2000;
/// Characters of context around a match.
const BEFORE: usize = 40;
const AFTER: usize = 70;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Chapter,
    Character,
    Note,
    Research,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::Chapter => "Chapter",
            SourceKind::Character => "Character",
            SourceKind::Note => "Note",
            SourceKind::Research => "Research",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Source {
    pub path: PathBuf,
    pub title: String,
    pub kind: SourceKind,
    pub pdf: bool,
    /// The editor's text for the open file, which may not be saved yet.
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    /// 1-based line in a text file.
    Line(usize),
    /// 0-based page of a PDF.
    Page(usize),
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub location: Location,
    /// Character range in the file's text (or in the page's text, for a PDF).
    pub range: Range<usize>,
    pub before: String,
    pub matched: String,
    pub after: String,
}

#[derive(Clone, Debug)]
pub struct FileHits {
    pub source: Source,
    pub hits: Vec<Hit>,
    /// All matches in the file, including those not kept.
    pub total: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Results {
    pub generation: u64,
    pub query: String,
    pub files: Vec<FileHits>,
    pub done: bool,
    /// The PDF being read for the first time, if any.
    pub reading: Option<String>,
    /// Files that could not be read.
    pub failed: usize,
}

impl Results {
    pub fn total(&self) -> usize {
        self.files.iter().map(|f| f.total).sum()
    }
}

struct Request {
    generation: u64,
    query: String,
    sources: Vec<Source>,
}

pub struct Searcher {
    requests: mpsc::Sender<Request>,
    results: mpsc::Receiver<Results>,
    generation: Arc<AtomicU64>,
}

impl Searcher {
    pub fn new() -> Searcher {
        Searcher::with_cache_dir(text::default_cache_dir())
    }

    /// A searcher that keeps PDF text in `cache_dir`.
    pub fn with_cache_dir(cache_dir: Option<PathBuf>) -> Searcher {
        let (requests, request_rx) = mpsc::channel::<Request>();
        let (result_tx, results) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let latest = generation.clone();
        let _ = thread::Builder::new()
            .name("project-search".into())
            .spawn(move || {
                while let Ok(mut request) = request_rx.recv() {
                    while let Ok(newer) = request_rx.try_recv() {
                        request = newer;
                    }
                    run(request, &latest, cache_dir.as_deref(), &result_tx);
                }
            });
        Searcher {
            requests,
            results,
            generation,
        }
    }

    /// Start a search; earlier ones stop. Returns its generation.
    pub fn search(&self, query: &str, sources: Vec<Source>) -> u64 {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.requests.send(Request {
            generation,
            query: query.to_owned(),
            sources,
        });
        generation
    }

    /// The newest results that arrived, if any.
    pub fn poll(&self) -> Option<Results> {
        self.results.try_iter().last()
    }
}

fn run(
    request: Request,
    latest: &AtomicU64,
    cache_dir: Option<&std::path::Path>,
    out: &mpsc::Sender<Results>,
) {
    let query = text::fold_query(&request.query);
    let mut results = Results {
        generation: request.generation,
        query: request.query.clone(),
        ..Results::default()
    };
    if query.is_empty() {
        results.done = true;
        let _ = out.send(results);
        return;
    }
    let stale = || latest.load(Ordering::SeqCst) != request.generation;
    let mut kept = 0;
    // Text files first: they are quick, and the PDFs may need reading.
    let (pdfs, texts): (Vec<Source>, Vec<Source>) =
        request.sources.into_iter().partition(|s| s.pdf);
    for source in texts {
        if stale() {
            return;
        }
        let content = match &source.text {
            Some(text) => Some(text.clone()),
            None => std::fs::read(&source.path)
                .ok()
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        };
        let Some(content) = content else {
            results.failed += 1;
            continue;
        };
        if let Some(file) = search_text(source, &content, &query, &mut kept) {
            results.files.push(file);
        }
    }
    let _ = out.send(results.clone());
    for source in pdfs {
        if stale() {
            return;
        }
        let cached = cache_dir
            .and_then(|dir| text::cache_file(dir, &source.path))
            .is_some_and(|file| file.exists());
        if !cached {
            results.reading = Some(source.title.clone());
            let _ = out.send(results.clone());
        }
        match pdf::text_of(&source.path, cache_dir) {
            Ok(pages) => {
                if let Some(file) = search_pages(source, &pages, &query, &mut kept) {
                    results.files.push(file);
                }
            }
            Err(_) => results.failed += 1,
        }
        results.reading = None;
        let _ = out.send(results.clone());
    }
    results.done = true;
    let _ = out.send(results);
}

fn search_text(
    source: Source,
    content: &str,
    query: &[char],
    kept: &mut usize,
) -> Option<FileHits> {
    let ranges = text::find(&text::fold(content), query);
    if ranges.is_empty() {
        return None;
    }
    let chars: Vec<char> = content.chars().collect();
    let total = ranges.len();
    let mut hits = Vec::new();
    let mut line = 1;
    let mut counted = 0;
    for range in ranges {
        if hits.len() >= HITS_PER_FILE || *kept >= MAX_HITS {
            break;
        }
        line += chars[counted..range.start]
            .iter()
            .filter(|&&c| c == '\n')
            .count();
        counted = range.start;
        hits.push(hit(&chars, range, Location::Line(line)));
        *kept += 1;
    }
    Some(FileHits {
        source,
        hits,
        total,
    })
}

fn search_pages(
    source: Source,
    pages: &[text::PageText],
    query: &[char],
    kept: &mut usize,
) -> Option<FileHits> {
    let mut hits = Vec::new();
    let mut total = 0;
    for (page, page_text) in pages.iter().enumerate() {
        let ranges = text::find(&text::fold(&page_text.text), query);
        if ranges.is_empty() {
            continue;
        }
        total += ranges.len();
        let chars: Vec<char> = page_text.text.chars().collect();
        for range in ranges {
            if hits.len() >= HITS_PER_FILE || *kept >= MAX_HITS {
                break;
            }
            hits.push(hit(&chars, range, Location::Page(page)));
            *kept += 1;
        }
    }
    (total > 0).then_some(FileHits {
        source,
        hits,
        total,
    })
}

fn hit(chars: &[char], range: Range<usize>, location: Location) -> Hit {
    let start = range.start.saturating_sub(BEFORE);
    let end = (range.end + AFTER).min(chars.len());
    let mut before = flatten(&chars[start..range.start]);
    if start > 0 {
        before.insert(0, '…');
    }
    let mut after = flatten(&chars[range.end..end]);
    if end < chars.len() {
        after.push('…');
    }
    Hit {
        location,
        matched: flatten(&chars[range.clone()]),
        range,
        before,
        after,
    }
}

/// Context on one line: whitespace runs (line breaks included) become one space.
fn flatten(chars: &[char]) -> String {
    let mut out = String::with_capacity(chars.len());
    for &c in chars {
        if c.is_whitespace() {
            if !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn source(path: PathBuf, kind: SourceKind, text: Option<&str>) -> Source {
        Source {
            title: path.file_stem().unwrap().to_string_lossy().into_owned(),
            pdf: path.extension().is_some_and(|e| e == "pdf"),
            path,
            kind,
            text: text.map(str::to_owned),
        }
    }

    fn wait(searcher: &Searcher, generation: u64) -> Results {
        let start = Instant::now();
        loop {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "search timed out"
            );
            if let Some(results) = searcher.poll() {
                if results.generation == generation && results.done {
                    return results;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn finds_matches_with_lines_snippets_and_unsaved_text() {
        let temp = tempfile::tempdir().unwrap();
        let chapter = temp.path().join("001.md");
        std::fs::write(
            &chapter,
            "# One\n\nThe harbor froze.\nMara saw the Harbor.\n",
        )
        .unwrap();
        let open = temp.path().join("002.md");
        std::fs::write(&open, "nothing on disk").unwrap();
        let missing = temp.path().join("gone.md");

        let searcher = Searcher::with_cache_dir(None);
        let generation = searcher.search(
            "harbor",
            vec![
                source(chapter.clone(), SourceKind::Chapter, None),
                source(
                    open.clone(),
                    SourceKind::Chapter,
                    Some("A new harbór, unsaved."),
                ),
                source(missing, SourceKind::Note, None),
            ],
        );
        let results = wait(&searcher, generation);
        assert_eq!(results.total(), 3);
        assert_eq!(results.failed, 1);
        let first = &results.files[0];
        assert_eq!(first.source.path, chapter);
        assert_eq!(first.hits[0].location, Location::Line(3));
        assert_eq!(first.hits[1].location, Location::Line(4));
        assert_eq!(first.hits[1].matched, "Harbor");
        assert_eq!(first.hits[0].before, "# One The ");
        assert_eq!(first.hits[0].after, " froze. Mara saw the Harbor. ");
        // Character ranges point into the original text, for selecting it in the editor.
        let text: Vec<char> = "# One\n\nThe harbor froze.\n".chars().collect();
        let r = first.hits[0].range.clone();
        assert_eq!(text[r].iter().collect::<String>(), "harbor");
        assert_eq!(results.files[1].hits[0].matched, "harbór");

        let empty = searcher.search("  ", Vec::new());
        assert_eq!(wait(&searcher, empty).total(), 0);
    }

    #[test]
    fn searches_pdf_text_and_caches_it() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("notes.pdf");
        std::fs::write(&file, crate::reference::pdf::tests::text_pdf()).unwrap();
        let cache = temp.path().join("cache");
        let searcher = Searcher::with_cache_dir(Some(cache.clone()));
        let generation = searcher.search(
            "harbor",
            vec![source(file.clone(), SourceKind::Research, None)],
        );
        let results = wait(&searcher, generation);
        let hit = &results.files[0].hits[0];
        assert_eq!(hit.location, Location::Page(0));
        assert_eq!(hit.matched, "har- bor");
        assert!(text::cache_file(&cache, &file).unwrap().exists());
    }
}
