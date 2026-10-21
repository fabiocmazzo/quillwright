# Reference material: viewing and searching research files

Writers keep research next to the book: PDFs of articles and books, photos, maps, scanned
notes, plain-text clippings. This document describes how Quillwright shows that material
**beside** the manuscript, and how it will become searchable.

Status: step 1 (research folder, text and image viewer) is implemented. Steps 2 to 5 are
planned.

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
| `src/reference.rs` | `Kind` (by extension), loading a file into a `Content` on a background thread, copying files into `research/` |
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
| PDF (step 2) | `hayro::render` rasterizes only the visible pages, on a background thread, at the screen's scale. A small LRU cache keeps textures for nearby pages. |

hayro renders on the CPU, does not open password-protected PDFs, and still lacks a few rare
features. Scanned PDFs have no text layer, so they can be viewed but not searched.

## Search (steps 3 and 4)

hayro passes each glyph to a `Device` together with `Glyph::as_unicode()` and its transform.
The `extract_html` example in `hayro-interpret` shows the technique.

1. **Extract** per page a `PageText { text: String, boxes: Vec<(Range<usize>, Rect)> }` on a
   background thread. The boxes turn each match into a highlighted rectangle on the rendered
   page.
2. **Cache** it in `<data dir>/quillwright/text-cache/<fnv hash of path>-<mtime>-<size>`.
3. **Search** with a linear scan in memory (fast enough for hundreds of PDFs, no index
   needed). Normalize case and diacritics (so "coracao" finds "coração"), and join words
   hyphenated across line ends and ligatures (fi, fl).
4. **Project-wide search** covers chapters, characters, notes, and research in one result
   list. A result opens the chapter at the position, or the PDF at the page with the match
   highlighted.

OCR for scanned PDFs (for example the pure-Rust `ocrs`) is out of scope for now.

## Assistant (step 5)

Selecting a passage in a reference and choosing **Use as Context** adds it to the next
assistant prompt with its citation (`research/book.pdf, p. 42`). As always, the assistant
only suggests; nothing enters the manuscript unless the author accepts it.

## Steps

1. Research folder, sidebar section, Reference panel for text and images. **Done.**
2. PDF viewer: background rendering, visible pages only, texture cache, page navigation.
3. PDF text extraction, cache, and search in the open PDF with highlights.
4. Project-wide search across the manuscript and research.
5. "Use as Context" from references in the assistant.
