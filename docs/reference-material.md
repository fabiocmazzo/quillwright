# Reference material: viewing and searching research files

Writers keep research next to the book: PDFs of articles and books, photos, maps, scanned
notes, plain-text clippings. This document describes how Quillwright shows that material
**beside** the manuscript, and how it will become searchable.

Status: steps 1 (research folder, text and image viewer), 2 (PDF viewer), and 3 (search
in the open PDF) are implemented. Steps 4 and 5 are planned.

## Principles

- **Read only.** Reference files are never written by the app. Nothing in the reference
  viewer goes through saving, autosave, or version history, so the rule "never lose or
  silently overwrite the author's text" is not put at risk.
- **Beside, not instead of.** The viewer is a side panel. The chapter stays open in the
  editor while the author reads.
- **Plain files, no lock-in.** Research is an ordinary `research/` folder in the project.
  There is no manifest and no metadata; the sidebar lists what is on disk.
- **App data stays out of the project.** Caches (rendered pages, extracted PDF text) go in the
  user data folder, like version history.
- **Pure Rust, offline.** No external converters or C libraries. Quillwright already depends
  (through typst) on `hayro`, a pure-Rust PDF renderer, and on `image`.

## Architecture

The single open *document* (`content`, `file_path`, `modified`, `disk_conflict`, ...) stays as
it is on `MarkdownApp`. Reference viewing is a separate feature with its own state, following
the pattern of the assistant and export:

| Piece | Role |
| --- | --- |
| `src/reference/mod.rs` | `Kind` (by extension), loading a file into a `Content` on a background thread, copying files into `research/` |
| `src/reference/pdf.rs` | `Document`: a thread that owns the parsed PDF, renders pages, and extracts text |
| `src/reference/text.rs` | Text extraction, search folding, highlights, the text cache |
| `src/pdf_view.rs` | The PDF viewer: stacked pages, page requests, textures, navigation, zoom |
| `src/reference_ui.rs` | The Reference side panel, the sidebar's Research section actions, "Add Research Files…" |
| `manuscript::Index::research` | Files under `research/` (recursive), sorted by path |

Splitting the document state into a `Document` struct is only worth doing when Quillwright
needs to *edit* two files at once (tabs or a split editor).

### Opening

- Files in the sidebar's **Research** section open in the Reference panel. Text and Markdown
  have an **Edit** button that opens them in the editor instead.
- In **All Files**, images and PDFs open in the Reference panel; everything else keeps opening
  in the editor.
- Unsupported kinds show a short message and an **Open with System Viewer** button.

### Reloading

The file watcher already reports tree changes, which rebuild the manuscript index. After a
rebuild, the panel compares the open reference's modification time and size and reloads it if
they changed (an agent may have added notes, for example).

## Formats

| Format | How |
| --- | --- |
| `.txt` | Shown as wrapped, selectable text in the book font. |
| `.md`, `.markdown` | Styled with `live_preview::project` without a caret, so all syntax is hidden. |
| PNG, JPEG, GIF (first frame), WebP | Decoded by `image` on a background thread, scaled down to at most 4096 px, shown fitted to the panel width or at 100 %. |
| PDF | `hayro::render` rasterizes only the visible pages (plus one on each side), on a background thread, at the screen's scale. Textures are kept for pages near the current one. |

### PDF rendering

hayro's parsed document, pages, and render cache borrow each other and use `Rc`, so they
cannot move between threads. `pdf::Document::open` starts a thread that reads and parses
the file, reports the page sizes, and then waits for requests. Each frame the viewer
computes which pages are visible and at what width in pixels (rounded up to 128 px, so
small panel resizes reuse the textures), and calls `Document::exchange`: it returns the
pages rendered since the last frame and replaces the list of wanted pages. Because the list
is replaced, pages that scrolled out of view are dropped before they are rendered. While a
new width is on its way, the old texture is shown scaled.

hayro's default features embed the 14 standard PDF fonts and the CJK CMaps (about 600 KB),
so PDFs that do not embed their fonts still show text. A panic inside hayro on one page is
caught; that page shows "This page could not be shown" and the others keep working.

hayro renders on the CPU, does not open password-protected PDFs, and still lacks a few rare
features. Scanned PDFs have no text layer, so they can be viewed but not searched.

## Search

### In the open PDF (step 3, done)

1. **Extraction.** When no page is waiting to be rendered, the PDF thread interprets the
   next page with `text::Extractor`, a hayro `Device` that receives each glyph with
   `Glyph::as_unicode()` and its transform. It produces a `PageText { text, boxes }` with
   one box per character, in PDF points from the page's top-left corner (the same transform
   the renderer uses, so boxes line up with the image). Spaces are inserted where the gap
   between glyphs is wider than a fifth of the font size, and line breaks where the baseline
   changes. Ligatures ("ﬁ") become several characters sharing the glyph's box.
2. **Cache.** When every page is done, the text is written to
   `<data dir>/quillwright/text-cache/<fnv of path>-<fnv of size and mtime>.cache`:
   deflate-compressed, with boxes delta-encoded in tenths of a point (about 650 bytes a page
   in tests). Writing a new cache removes older ones for the same file. The next time the
   PDF opens, its text is available at once.
3. **Search.** `text::fold` lowercases, removes accents (NFKD without combining marks),
   collapses whitespace, and joins a hyphen followed by a line break, keeping a map back to
   the original characters. Matching is a linear scan, fast enough for long books. Matches
   are kept in page order; as more pages are read, their matches are inserted. The first
   match shown is the first one from the page being read.
4. **Highlights.** Each match becomes one rectangle per line (`text::highlight`), drawn over
   the page; the current match is orange.

### Across the project (step 4)

**Project-wide search** covers chapters, characters, notes, and research in one result
list. A result opens the chapter at the position, or the PDF at the page with the match
highlighted. The PDF text cache can be read without opening the PDF, so research PDFs
that were opened once are searchable right away; others can be extracted in the background.

OCR for scanned PDFs (for example the pure-Rust `ocrs`) is out of scope for now.

## Assistant (step 5)

Selecting a passage in a reference and choosing **Use as Context** adds it to the next
assistant prompt with its citation (`research/book.pdf, p. 42`). As always, the assistant
only suggests; nothing enters the manuscript unless the author accepts it.

## Steps

1. Research folder, sidebar section, Reference panel for text and images. **Done.**
2. PDF viewer: background rendering, visible pages only, texture cache, page navigation.
   **Done.**
3. PDF text extraction, cache, and search in the open PDF with highlights. **Done.**
4. Project-wide search across the manuscript and research.
5. "Use as Context" from references in the assistant.
