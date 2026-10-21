//! The text of PDF pages: extraction with hayro (each character with its box on the
//! page), search that ignores case and accents, and a cache in the user data folder.
use hayro::hayro_interpret::{
    font::Glyph, hayro_cmap::BfString, BlendMode, ClipPath, Device, GlyphDrawMode, Image, Paint,
    PathDrawMode, SoftMask,
};
use kurbo::{Affine, BezPath, Point};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

/// The text of one page. `boxes[i]` is the box of the `i`-th character of `text`, in PDF
/// points from the page's top-left corner, as `[left, top, right, bottom]`. Spaces and line
/// breaks the extractor inserted have zero-width boxes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageText {
    pub text: String,
    pub boxes: Vec<[f32; 4]>,
}

impl PageText {
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

// ----- Extraction --------------------------------------------------------

/// Collects glyphs as characters. Glyph outlines use 1000 units per em.
#[derive(Default)]
pub(crate) struct Extractor {
    chars: Vec<char>,
    boxes: Vec<[f32; 4]>,
    last: Option<Last>,
}

struct Last {
    baseline: f64,
    start: f64,
    end: f64,
    size: f64,
}

impl Extractor {
    pub(crate) fn finish(self) -> PageText {
        let mut chars = self.chars;
        let mut boxes = self.boxes;
        while chars.last().is_some_and(|c| c.is_whitespace()) {
            chars.pop();
            boxes.pop();
        }
        PageText {
            text: chars.into_iter().collect(),
            boxes,
        }
    }

    fn separate(&mut self, separator: char) {
        if self.chars.last().is_none_or(|c| c.is_whitespace()) {
            return;
        }
        let previous = *self.boxes.last().unwrap();
        self.chars.push(separator);
        self.boxes
            .push([previous[2], previous[1], previous[2], previous[3]]);
    }

    fn add(&mut self, text: &str, transform: Affine, advance: f64) {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_control()).collect();
        if chars.is_empty() {
            return;
        }
        let origin = transform * Point::ZERO;
        let end = transform * Point::new(advance, 0.0);
        let size = (transform * Point::new(0.0, 1000.0) - origin).hypot();
        if !size.is_finite() || size <= 0.0 {
            return;
        }
        if let Some(last) = &self.last {
            let same_line = (origin.y - last.baseline).abs() < 0.5 * size.max(last.size)
                && origin.x > last.start - size;
            if !same_line {
                self.separate('\n');
            } else if origin.x - last.end > 0.2 * size.min(last.size) {
                self.separate(' ');
            }
        }
        let corners = [
            transform * Point::new(0.0, -200.0),
            transform * Point::new(advance, -200.0),
            transform * Point::new(0.0, 800.0),
            transform * Point::new(advance, 800.0),
        ];
        let left = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let right = corners
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let top = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let bottom = corners
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max);
        // A ligature such as "ﬁ" becomes several characters sharing the glyph's box.
        let step = (right - left) / chars.len() as f64;
        for (i, c) in chars.into_iter().enumerate() {
            let x = left + step * i as f64;
            self.chars.push(c);
            self.boxes
                .push([x as f32, top as f32, (x + step) as f32, bottom as f32]);
        }
        self.last = Some(Last {
            baseline: origin.y,
            start: origin.x,
            end: end.x,
            size,
        });
    }
}

impl<'a> Device<'a> for Extractor {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph(
        &mut self,
        glyph: &Glyph<'a>,
        transform: Affine,
        glyph_transform: Affine,
        _: &Paint<'a>,
        _: &GlyphDrawMode,
    ) {
        let Some(unicode) = glyph.as_unicode() else {
            return;
        };
        let text = match unicode {
            BfString::Char(c) => c.to_string(),
            BfString::String(s) => s,
        };
        let advance = match glyph {
            Glyph::Outline(outline) => outline.advance_width().unwrap_or(500.0),
            Glyph::Type3(_) => 500.0,
        };
        let advance = if advance > 0.0 { advance } else { 500.0 };
        self.add(&text, transform * glyph_transform, advance as f64);
    }
    fn draw_image(&mut self, _: Image<'a, '_>, _: Affine) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

// ----- Search ------------------------------------------------------------

/// Text prepared for searching: lowercase, without accents, with whitespace collapsed and
/// words hyphenated across line ends joined. `origin[i]` is the index of the character in
/// the original text that produced `chars[i]`.
#[derive(Clone, Debug, Default)]
pub struct Folded {
    chars: Vec<char>,
    origin: Vec<usize>,
}

pub fn fold(text: &str) -> Folded {
    let source: Vec<char> = text.chars().collect();
    let mut folded = Folded::default();
    let mut i = 0;
    while i < source.len() {
        let c = source[i];
        if matches!(c, '-' | '\u{AD}' | '\u{2010}') && source.get(i + 1) == Some(&'\n') {
            i += 2; // "har-\nbor" reads as "harbor".
            continue;
        }
        if c == '\u{AD}' {
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            if folded.chars.last().is_some_and(|&last| last != ' ') {
                folded.chars.push(' ');
                folded.origin.push(i);
            }
            i += 1;
            continue;
        }
        for decomposed in c.nfkd().filter(|&d| !is_combining_mark(d)) {
            for lower in decomposed.to_lowercase() {
                folded.chars.push(lower);
                folded.origin.push(i);
            }
        }
        i += 1;
    }
    folded
}

/// A search query, folded like the text it is compared with.
pub fn fold_query(query: &str) -> Vec<char> {
    let mut chars = fold(query).chars;
    while chars.last() == Some(&' ') {
        chars.pop();
    }
    if chars.first() == Some(&' ') {
        chars.remove(0);
    }
    chars
}

/// Every match of `query` in `text`, as ranges of character indices in the original text.
pub fn find(text: &Folded, query: &[char]) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    if query.is_empty() || query.len() > text.chars.len() {
        return found;
    }
    let mut start = 0;
    while start + query.len() <= text.chars.len() {
        if text.chars[start..start + query.len()] == *query {
            let end = start + query.len() - 1;
            found.push(text.origin[start]..text.origin[end] + 1);
            start += query.len();
        } else {
            start += 1;
        }
    }
    found
}

/// Rectangles covering `range`, one per line, as `[left, top, right, bottom]` in points.
pub fn highlight(page: &PageText, range: std::ops::Range<usize>) -> Vec<[f32; 4]> {
    let mut rects: Vec<[f32; 4]> = Vec::new();
    for b in page.boxes.get(range).unwrap_or_default() {
        if b[2] <= b[0] {
            continue; // An inserted space or line break.
        }
        match rects.last_mut() {
            Some(r) if (b[1] - r[1]).abs() < 0.5 * (r[3] - r[1]) && b[0] >= r[0] - 1.0 => {
                r[0] = r[0].min(b[0]);
                r[1] = r[1].min(b[1]);
                r[2] = r[2].max(b[2]);
                r[3] = r[3].max(b[3]);
            }
            _ => rects.push(*b),
        }
    }
    rects
}

// ----- Cache -------------------------------------------------------------

const CACHE_HEADER: &str = "quillwright-pdf-text 2";

pub fn default_cache_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "quillwright", "quillwright")
        .map(|dirs| dirs.data_dir().join("text-cache"))
}

/// The cache file for `path` as it is now on disk (its size and modification time are part
/// of the name, so a changed PDF never reads stale text).
pub fn cache_file(dir: &Path, path: &Path) -> Option<PathBuf> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    let id = crate::history::fnv1a(path.to_string_lossy().as_bytes());
    let version = crate::history::fnv1a(format!("{}-{modified}", meta.len()).as_bytes());
    Some(dir.join(format!("{id:016x}-{version:016x}.cache")))
}

/// Read a cache file. With `pages`, a cache for a different number of pages is refused.
pub fn read_cache(file: &Path, pages: Option<usize>) -> Option<Vec<PageText>> {
    let mut data = String::new();
    flate2::read::DeflateDecoder::new(fs::File::open(file).ok()?)
        .read_to_string(&mut data)
        .ok()?;
    let mut lines = data.lines();
    if lines.next()? != CACHE_HEADER {
        return None;
    }
    let count = lines.next()?.parse::<usize>().ok()?;
    if pages.is_some_and(|pages| pages != count) {
        return None;
    }
    let pages = count;
    let mut out = Vec::with_capacity(pages);
    for _ in 0..pages {
        let text = unescape(lines.next()?);
        let boxes = decode_boxes(lines.next()?)?;
        if boxes.len() != text.chars().count() {
            return None;
        }
        out.push(PageText { text, boxes });
    }
    Some(out)
}

/// Boxes in tenths of a point, relative to the previous box: `@top,bottom` starts a run
/// of boxes with that top and bottom; then each box is `width`, or `gap,width` when it
/// does not start where the previous one ended. Text on a line costs a few bytes a letter.
fn encode_boxes(boxes: &[[f32; 4]]) -> String {
    let tenths = |n: f32| (n * 10.0).round() as i32;
    let mut out = String::new();
    let mut vertical = None;
    let mut right = 0;
    for b in boxes {
        let (left, top, end, bottom) = (tenths(b[0]), tenths(b[1]), tenths(b[2]), tenths(b[3]));
        if vertical != Some((top, bottom)) {
            out.push_str(&format!("@{top},{bottom} "));
            vertical = Some((top, bottom));
        }
        let gap = left - right;
        if gap == 0 {
            out.push_str(&format!("{} ", end - left));
        } else {
            out.push_str(&format!("{gap},{} ", end - left));
        }
        right = end;
    }
    out
}

fn decode_boxes(line: &str) -> Option<Vec<[f32; 4]>> {
    let mut boxes = Vec::new();
    let mut vertical = None;
    let mut right = 0i32;
    for token in line.split_ascii_whitespace() {
        let pair = |s: &str| -> Option<(i32, i32)> {
            let (a, b) = s.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        };
        if let Some(rest) = token.strip_prefix('@') {
            vertical = Some(pair(rest)?);
            continue;
        }
        let (top, bottom) = vertical?;
        let (gap, width) = match token.contains(',') {
            true => pair(token)?,
            false => (0, token.parse().ok()?),
        };
        let left = right + gap;
        right = left + width;
        boxes.push([left, top, right, bottom].map(|n| n as f32 / 10.0));
    }
    Some(boxes)
}

/// Write the cache atomically, and remove older caches of the same file.
pub fn write_cache(file: &Path, pages: &[PageText]) -> std::io::Result<()> {
    let dir = file.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let mut out = String::new();
    out.push_str(CACHE_HEADER);
    out.push('\n');
    out.push_str(&pages.len().to_string());
    out.push('\n');
    for page in pages {
        out.push_str(&escape(&page.text));
        out.push('\n');
        out.push_str(&encode_boxes(&page.boxes));
        out.push('\n');
    }
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(out.as_bytes())?;
    let compressed = encoder.finish()?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(&compressed)?;
    temp.persist(file).map_err(|e| e.error)?;
    if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
        let prefix = &name[..name.find('-').map_or(0, |i| i + 1)];
        for entry in fs::read_dir(dir)?.flatten() {
            let other = entry.file_name();
            let other = other.to_string_lossy();
            if !prefix.is_empty() && other.starts_with(prefix) && other != name {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(text: &str) -> PageText {
        let boxes = text
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let x = i as f32 * 5.0;
                if c.is_whitespace() {
                    [x, 0.0, x, 10.0]
                } else {
                    [x, 0.0, x + 5.0, 10.0]
                }
            })
            .collect();
        PageText {
            text: text.into(),
            boxes,
        }
    }

    #[test]
    fn search_ignores_case_accents_ligatures_and_line_hyphens() {
        let text = "O Coração do porto\nficou ﬁrme. A har-\nbor FROZE.";
        let folded = fold(text);
        let at = |query: &str| -> Vec<String> {
            find(&folded, &fold_query(query))
                .into_iter()
                .map(|r| text.chars().skip(r.start).take(r.len()).collect())
                .collect()
        };
        assert_eq!(at("coracao"), vec!["Coração"]);
        assert_eq!(at("CORAÇÃO"), vec!["Coração"]);
        assert_eq!(at("firme"), vec!["ﬁrme"]);
        assert_eq!(at("harbor"), vec!["har-\nbor"]);
        assert_eq!(at("porto ficou"), vec!["porto\nficou"]);
        assert_eq!(at("  froze "), vec!["FROZE"]);
        assert!(at("").is_empty());
        assert!(at("nothing").is_empty());
    }

    #[test]
    fn highlights_merge_characters_on_a_line() {
        let p = page("ab cd");
        assert_eq!(highlight(&p, 0..5), vec![[0.0, 0.0, 25.0, 10.0]]);
        let mut two_lines = page("ab\ncd");
        for b in &mut two_lines.boxes[3..] {
            b[1] += 20.0;
            b[3] += 20.0;
        }
        assert_eq!(highlight(&two_lines, 0..5).len(), 2);
    }

    #[test]
    fn cache_round_trips_and_replaces_older_versions() {
        let temp = tempfile::tempdir().unwrap();
        let pdf = temp.path().join("book.pdf");
        fs::write(&pdf, "one").unwrap();
        let dir = temp.path().join("cache");
        let pages = vec![page("Line one\\two\nnext"), PageText::default()];
        let first = cache_file(&dir, &pdf).unwrap();
        write_cache(&first, &pages).unwrap();
        assert_eq!(read_cache(&first, Some(2)).unwrap(), pages);
        // Boxes that do not touch, and that change line, survive too.
        let odd = vec![PageText {
            text: "ab".into(),
            boxes: vec![[1.5, 2.0, 3.0, 4.5], [10.0, 20.0, 12.5, 30.0]],
        }];
        write_cache(&first, &odd).unwrap();
        assert_eq!(read_cache(&first, None).unwrap(), odd);
        assert!(read_cache(&first, Some(3)).is_none());

        fs::write(&pdf, "changed, longer").unwrap();
        let second = cache_file(&dir, &pdf).unwrap();
        assert_ne!(first, second);
        write_cache(&second, &pages).unwrap();
        assert!(!first.exists());
        assert!(second.exists());
    }
}
