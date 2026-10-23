//! PDF pages rendered with hayro (pure Rust) on a background thread.
//!
//! hayro's document and caches borrow each other and use `Rc`, so one thread owns them
//! for the life of the open document. The viewer tells it which pages it needs; the
//! list is replaced every frame, so pages that scrolled away are never rendered.
//! Between renders, the thread extracts the text of each page for search (or reads it from
//! the cache in the user data folder).
use super::text::{self, PageText};
use eframe::egui::ColorImage;
use hayro::{
    hayro_interpret::{
        interpret_page, util::TransformExt, Context, InterpreterCache, InterpreterSettings,
    },
    hayro_syntax::{page::Page, LoadPdfError, Pdf},
    vello_cpu::color::palette::css::WHITE,
    RenderCache, RenderSettings,
};
use std::{
    fs,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Condvar, Mutex, MutexGuard},
    thread,
};

/// Rendered pages never exceed this many pixels on a side.
pub const MAX_SIDE: u32 = 4096;

/// A page rendered at a given width in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub page: usize,
    pub width: u32,
}

pub struct Rendered {
    pub key: Key,
    /// None when the page could not be rendered.
    pub image: Option<ColorImage>,
}

/// What arrived since the last exchange.
#[derive(Default)]
pub struct Update {
    pub rendered: Vec<Rendered>,
    /// (page index, text), in page order.
    pub texts: Vec<(usize, PageText)>,
}

#[derive(Default)]
struct State {
    wanted: Vec<Key>,
    current: Option<Key>,
    finished: Vec<Rendered>,
    texts: Vec<(usize, PageText)>,
    /// Pages whose text has been extracted (or read from the cache).
    text_pages: usize,
    closed: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// An open PDF. Dropping it stops the render thread.
pub struct Document {
    /// Page sizes in points (rotation applied).
    pages: Vec<[f32; 2]>,
    shared: Arc<Shared>,
}

impl Document {
    /// Open `path`. With `cache_dir`, extracted text is cached there.
    pub fn open(path: &Path, cache_dir: Option<&Path>) -> Result<Document, String> {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        let cache_file = cache_dir.and_then(|dir| text::cache_file(dir, path));
        let shared = Arc::new(Shared::default());
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread_shared = shared.clone();
        thread::Builder::new()
            .name("pdf-render".into())
            .spawn(move || run(data, &thread_shared, ready_tx, cache_file))
            .map_err(|e| e.to_string())?;
        let pages = ready_rx
            .recv()
            .map_err(|_| crate::i18n::t!("pdf-unreadable"))??;
        Ok(Document { pages, shared })
    }

    pub fn page_sizes(&self) -> &[[f32; 2]] {
        &self.pages
    }

    /// Collect finished pages and text, and say which pages are still needed, most
    /// important first. `wanted` sees the finished pages, so it can leave them out.
    pub fn exchange(&self, wanted: impl FnOnce(&[Rendered]) -> Vec<Key>) -> Update {
        let mut state = self.shared.lock();
        let rendered = std::mem::take(&mut state.finished);
        let texts = std::mem::take(&mut state.texts);
        let current = state.current;
        state.wanted = wanted(&rendered)
            .into_iter()
            .filter(|key| Some(*key) != current)
            .collect();
        if !state.wanted.is_empty() {
            self.shared.wake.notify_one();
        }
        Update { rendered, texts }
    }

    /// True while a page is being rendered or waiting to be collected. (Text arriving
    /// in the background does not count: it is collected on the next frame.)
    pub fn busy(&self) -> bool {
        let state = self.shared.lock();
        state.current.is_some() || !state.wanted.is_empty() || !state.finished.is_empty()
    }

    /// How many pages have their text ready, counting pages not yet collected.
    pub fn text_pages(&self) -> usize {
        self.shared.lock().text_pages
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.wake.notify_all();
    }
}

fn run(
    data: Vec<u8>,
    shared: &Shared,
    ready: mpsc::SyncSender<Result<Vec<[f32; 2]>, String>>,
    cache_file: Option<PathBuf>,
) {
    let pdf = match Pdf::new(data) {
        Ok(pdf) => pdf,
        Err(error) => {
            let _ = ready.send(Err(load_error(&error)));
            return;
        }
    };
    let pages = pdf.pages();
    if pages.is_empty() {
        let _ = ready.send(Err(crate::i18n::t!("pdf-no-pages")));
        return;
    }
    let sizes: Vec<[f32; 2]> = pages
        .iter()
        .map(|page| {
            let (w, h) = page.render_dimensions();
            [w.max(1.0), h.max(1.0)]
        })
        .collect();
    if ready.send(Ok(sizes.clone())).is_err() {
        return;
    }
    let settings = InterpreterSettings::default();
    let mut cache = RenderCache::new();
    let text_cache = InterpreterCache::new();
    let mut texts: Vec<PageText> = Vec::new();
    if let Some(cached) = cache_file
        .as_deref()
        .and_then(|file| text::read_cache(file, Some(pages.len())))
    {
        let mut state = shared.lock();
        state.text_pages = cached.len();
        state.texts = cached.into_iter().enumerate().collect();
    }
    let mut text_done = shared.lock().text_pages == pages.len();
    loop {
        let key = {
            let mut state = shared.lock();
            loop {
                if state.closed {
                    return;
                }
                if !state.wanted.is_empty() {
                    let key = state.wanted.remove(0);
                    state.current = Some(key);
                    break Some(key);
                }
                if !text_done {
                    break None;
                }
                state = shared.wake.wait(state).unwrap_or_else(|e| e.into_inner());
            }
        };
        let Some(key) = key else {
            // Nothing to render: extract the next page's text.
            let index = texts.len();
            let page_text = panic::catch_unwind(AssertUnwindSafe(|| {
                extract(&pages[index], &text_cache, &settings, sizes[index])
            }))
            .unwrap_or_default();
            texts.push(page_text.clone());
            text_done = texts.len() == pages.len();
            if text_done {
                if let Some(file) = &cache_file {
                    let _ = text::write_cache(file, &texts);
                }
            }
            let mut state = shared.lock();
            state.texts.push((index, page_text));
            state.text_pages = texts.len();
            continue;
        };
        let image = match pages.get(key.page) {
            Some(page) => panic::catch_unwind(AssertUnwindSafe(|| {
                render(page, &cache, &settings, key.width, sizes[key.page])
            }))
            .unwrap_or_else(|_| {
                // A malformed page made hayro panic; start over with a clean cache.
                cache = RenderCache::new();
                None
            }),
            None => None,
        };
        let mut state = shared.lock();
        state.finished.push(Rendered { key, image });
        state.current = None;
    }
}

/// The text of every page of the PDF at `path`: from the cache when there is one, otherwise
/// extracted now (on the calling thread) and cached.
pub fn text_of(path: &Path, cache_dir: Option<&Path>) -> Result<Vec<PageText>, String> {
    let cache_file = cache_dir.and_then(|dir| text::cache_file(dir, path));
    if let Some(cached) = cache_file
        .as_deref()
        .and_then(|f| text::read_cache(f, None))
    {
        return Ok(cached);
    }
    let pdf = Pdf::new(fs::read(path).map_err(|e| e.to_string())?).map_err(|e| load_error(&e))?;
    let settings = InterpreterSettings::default();
    let cache = InterpreterCache::new();
    let pages: Vec<PageText> = pdf
        .pages()
        .iter()
        .map(|page| {
            let (w, h) = page.render_dimensions();
            panic::catch_unwind(AssertUnwindSafe(|| {
                extract(page, &cache, &settings, [w.max(1.0), h.max(1.0)])
            }))
            .unwrap_or_default()
        })
        .collect();
    if let Some(file) = &cache_file {
        let _ = text::write_cache(file, &pages);
    }
    Ok(pages)
}

fn extract<'a>(
    page: &'a Page<'a>,
    cache: &InterpreterCache<'a>,
    settings: &InterpreterSettings,
    size: [f32; 2],
) -> PageText {
    let mut context = Context::new(
        page.initial_transform(true).to_kurbo(),
        kurbo::Rect::new(0.0, 0.0, size[0] as f64, size[1] as f64),
        cache,
        page.xref(),
        settings.clone(),
    );
    let mut extractor = text::Extractor::default();
    interpret_page(page, &mut context, &mut extractor);
    extractor.finish()
}

fn render<'a>(
    page: &'a Page<'a>,
    cache: &RenderCache<'a>,
    settings: &InterpreterSettings,
    width: u32,
    size: [f32; 2],
) -> Option<ColorImage> {
    let scale = (width.clamp(1, MAX_SIDE) as f32 / size[0]).min(MAX_SIDE as f32 / size[1]);
    let pixmap = hayro::render(
        page,
        cache,
        settings,
        &RenderSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
            ..Default::default()
        },
    );
    let (w, h) = (pixmap.width() as usize, pixmap.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    // The background is opaque white, so premultiplied and straight alpha agree.
    Some(ColorImage::from_rgba_premultiplied(
        [w, h],
        pixmap.data_as_u8_slice(),
    ))
}

fn load_error(error: &LoadPdfError) -> String {
    match error {
        LoadPdfError::Decryption(_) => {
            crate::i18n::t!("pdf-password")
        }
        _ => crate::i18n::t!("pdf-invalid"),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A minimal PDF: `pages` pages of 200 × 100 pt, each with a black square in the middle.
    pub(crate) fn sample_pdf(pages: usize) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {pages} >>",
                (0..pages)
                    .map(|i| format!("{} 0 R", 4 + i))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        let content = "0 0 0 rg 80 30 40 40 re f";
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
        for _ in 0..pages {
            objects.push(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 3 0 R >>".into(),
            );
        }
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{object}\nendobj\n", i + 1).bytes());
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
        for offset in offsets {
            out.extend(format!("{offset:010} 00000 n \n").bytes());
        }
        out.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .bytes(),
        );
        out
    }

    /// One 200 × 100 pt page of Helvetica text: two words placed apart on one line, an
    /// accented word, and a word hyphenated across two lines.
    pub(crate) fn text_pdf() -> Vec<u8> {
        let content: &[u8] = b"BT /F1 12 Tf 20 70 Td (Hello) Tj 40 0 Td (World) Tj ET \
            BT /F1 12 Tf 20 50 Td (Cora\xe7\xe3o) Tj ET \
            BT /F1 12 Tf 20 30 Td (har-) Tj 0 -14 Td (bor) Tj ET";
        let mut objects: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>".to_vec(),
        ];
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(content);
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
        objects.push(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 3 0 R \
              /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_vec(),
        );
        objects.push(
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        );
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n", i + 1).bytes());
            out.extend_from_slice(object);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
        for offset in offsets {
            out.extend(format!("{offset:010} 00000 n \n").bytes());
        }
        out.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .bytes(),
        );
        out
    }

    fn wait_for_text(doc: &Document) -> Vec<(usize, PageText)> {
        let start = Instant::now();
        let mut texts = Vec::new();
        while texts.len() < doc.page_sizes().len() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "text was not extracted"
            );
            texts.extend(doc.exchange(|_| Vec::new()).texts);
            thread::sleep(Duration::from_millis(10));
        }
        texts
    }

    #[test]
    fn extracts_text_with_boxes_and_caches_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("text.pdf");
        fs::write(&path, text_pdf()).unwrap();
        let cache = temp.path().join("cache");

        let doc = Document::open(&path, Some(&cache)).unwrap();
        let texts = wait_for_text(&doc);
        let page = &texts[0].1;
        assert_eq!(page.text, "Hello World\nCoração\nhar-\nbor");
        assert_eq!(page.boxes.len(), page.text.chars().count());
        // "H" starts at x = 20 on the baseline 70 pt up, i.e. 30 pt from the top.
        let h = page.boxes[0];
        assert!((h[0] - 20.0).abs() < 0.5, "{h:?}");
        assert!(h[1] < 30.0 && h[3] > 30.0, "{h:?}");
        drop(doc);

        // The cache is written once everything is extracted, and used next time.
        let start = Instant::now();
        while fs::read_dir(&cache).map_or(0, |d| d.count()) == 0 {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "no cache written"
            );
            thread::sleep(Duration::from_millis(10));
        }
        // The cache keeps positions to a tenth of a point.
        let close = |cached: &PageText| {
            cached.text == page.text
                && cached
                    .boxes
                    .iter()
                    .zip(&page.boxes)
                    .all(|(a, b)| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 0.051))
        };
        let file = text::cache_file(&cache, &path).unwrap();
        assert!(close(&text::read_cache(&file, Some(1)).unwrap()[0]));
        let again = Document::open(&path, Some(&cache)).unwrap();
        assert!(close(&wait_for_text(&again)[0].1));
    }

    fn wait_for(doc: &Document, wanted: Vec<Key>) -> Vec<Rendered> {
        let mut done = Vec::new();
        let start = Instant::now();
        let mut first = true;
        while start.elapsed() < Duration::from_secs(30) {
            let missing: Vec<Key> = wanted
                .iter()
                .copied()
                .filter(|k| !done.iter().any(|r: &Rendered| r.key == *k))
                .collect();
            if missing.is_empty() && !first {
                return done;
            }
            first = false;
            done.extend(
                doc.exchange(|finished| {
                    missing
                        .into_iter()
                        .filter(|k| !finished.iter().any(|r| r.key == *k))
                        .collect()
                })
                .rendered,
            );
            thread::sleep(Duration::from_millis(10));
        }
        panic!("pages were not rendered in time");
    }

    #[test]
    fn opens_and_renders_pages_on_request() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.pdf");
        fs::write(&path, sample_pdf(3)).unwrap();
        let doc = Document::open(&path, None).unwrap();
        assert_eq!(doc.page_sizes(), &[[200.0, 100.0]; 3]);

        let key = Key {
            page: 2,
            width: 400,
        };
        let done = wait_for(&doc, vec![key]);
        let image = done[0].image.as_ref().expect("page 3 renders");
        assert_eq!(image.size, [400, 200]);
        let pixel = |x: usize, y: usize| image.pixels[y * 400 + x];
        assert_eq!(pixel(5, 5), eframe::egui::Color32::WHITE);
        assert_eq!(pixel(200, 100), eframe::egui::Color32::BLACK);

        let missing = wait_for(
            &doc,
            vec![Key {
                page: 9,
                width: 100,
            }],
        );
        assert!(missing[0].image.is_none());
    }

    #[test]
    fn explains_files_that_are_not_pdfs() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("fake.pdf");
        fs::write(&path, "hello").unwrap();
        let error = Document::open(&path, None).err().unwrap();
        assert!(error.contains("doesn't look like a PDF"), "{error}");
    }
}
