# Reference material: viewing and searching research files

Writers keep research next to the book: PDFs of articles and books, photos, maps, scanned
notes, plain-text clippings. This document describes how Quillwright shows that material
**beside** the manuscript, and how it will become searchable.

Status: all five steps (research folder, text and image viewer, PDF viewer, search in the
open PDF, project-wide search, and passages as assistant context) are implemented.

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

### Across the project (step 4, done)

The search field at the top of the sidebar (`search_ui.rs`, Ctrl+Shift+H) covers chapters,
characters, notes, and research text, Markdown, and PDF files. While it has text, the
sidebar shows results instead of the manuscript.

- **Worker.** `search::Searcher` owns one thread. Each query gets a generation number; the
  worker takes the newest request and stops an older one between files. The field waits
  0.25 s after the last keystroke before sending.
- **Sources.** Built from the manuscript index each time. The open file is searched from the
  editor buffer, so unsaved text is found. Text files are searched first and their results
  sent at once; PDFs follow.
- **PDFs.** `pdf::text_of` reads the text cache, or extracts the whole PDF on the worker
  thread and writes the cache. An extraction is never interrupted, so typing cannot make a
  large PDF start over. The sidebar shows "reading … for the first time" meanwhile.
- **Results.** Per file, up to 100 hits are kept (all are counted), 2000 in total, each
  with its line or page and about 40 characters of context before and 70 after.
- **Opening.** A text hit opens the file through `run_or_prompt_discard` and then selects the
  match (`SearchState::reveal`, applied once the file is open, dropped if the author
  cancels). A PDF hit calls `reveal_in_pdf`, and `PdfView::find_at` selects that exact match
  (page and character index) as soon as its page's text is available.

OCR for scanned PDFs (for example the pure-Rust `ocrs`) is out of scope for now.

## Assistant (step 5, done)

Selecting a passage in a reference and choosing **Use as Context** adds it to the
assistant's prompts with its citation (`research/book.pdf, p. 42`,
`research/notes.md, lines 4–9`). As always, the assistant only suggests; nothing enters the
manuscript unless the author accepts it.

- **Selecting.** Text and Markdown references are shown in a read-only `TextEdit`, because a
  `Label`'s selection cannot be read by the app. For Markdown, the buffer is the projection's
  display text (`live_preview::project` with no selection) and the layouter returns that
  projection's job, so the styling matches; the selection is mapped back to the source with
  `Projection::to_source`, so the clip keeps the Markdown. PDFs are selected by dragging over
  the pages: `PdfView::caret_at` finds the nearest glyph box from the extracted text (same
  line first), and the selection is painted with `text::highlight`. Ctrl+C copies, Esc clears.
  Line breaks inside the PDF text are joined (and `-` + line break removed).
- **Clips.** `assistant::Clip { citation, text }`, at most `MAX_CLIPS` (8, the oldest is
  dropped) and `CLIP_LIMIT` characters each. They stay in the panel until removed or the
  project changes, and are sent with every request in a "Reference material chosen by the
  author" section, each inside `<reference>` tags with a note that they are source material,
  not instructions.

## Steps

1. Research folder, sidebar section, Reference panel for text and images. **Done.**
2. PDF viewer: background rendering, visible pages only, texture cache, page navigation.
   **Done.**
3. PDF text extraction, cache, and search in the open PDF with highlights. **Done.**
4. Project-wide search across the manuscript and research. **Done.**
5. "Use as Context" from references in the assistant. **Done.**
