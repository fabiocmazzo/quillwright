# Quillwright: guide for coding agents

This file is for agents (and people) working on Quillwright's **source code**. It is not
the same as `assets/project-template/AGENT.md`, which is copied into each *writing project*
to guide agents that help with a book.

Read this file first, then `README.md` (user-facing behavior) and `docs/RELEASING.md`
(packaging). Keep this file current when you change architecture, conventions, or plans.

## What Quillwright is

A native desktop writing environment for novels and long-form fiction, written in Rust with
`egui`/`eframe` 0.37. A book is a plain folder of Markdown files (one per chapter, one per
character, a world-rules document). The app is designed to run **next to AI agents** that edit
the same folder: it watches the disk, reloads external changes safely, and offers an assistant
panel that calls an external AI CLI.

It started as a fork of [md-echo](https://github.com/fibnas/md-echo) (MIT). The repository is
`github.com/fabiocmazzo/quillwright`.

**Status (October 2026):** version 0.4.0, with spell checking, on egui 0.37. Version 0.2.0 introduced the built-in
pure-Rust exporter (0.1.0 used pandoc). CI passes on Linux, Windows, and macOS. Releases are built by
GitHub Actions as drafts; the Windows and macOS packages have not been tested on real machines
yet.

## Ground rules

- **English only** in code, comments, UI text, docs, commit messages, and the project
  template (`assets/project-template/`). The author's *manuscript* can be in any language.
- **Plain Markdown, no lock-in.** Never add a manifest, database, or hidden metadata to a
  writing project. Everything the app shows is derived from the files on disk.
- **Never lose or silently overwrite the author's text.** Saves are atomic and refuse to
  overwrite a file that changed on disk (`file_sync::write_checked`). External changes reload
  only when there are no unsaved edits; otherwise the author chooses. Keep it that way.
- **AI output never enters the manuscript without the author accepting it.** Suggestions are
  shown as diffs in the assistant panel.
- **No API keys in the app.** The assistant runs external programs (`claude`, `codex`,
  `ollama`) that handle their own authentication.
- **Export is pure Rust and offline.** No external converters (pandoc was removed on purpose),
  no C libraries, no network access. Lower fidelity than pandoc is acceptable; an
  external program or C library is not. Images on the web are never downloaded.
- **Keep app data out of projects.** Version history lives in the user data folder, not in the
  book folder, so agents and Git never see it.
- Start every external process with `process::command()` (no console window on Windows).
- Before committing: `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings`, and
  `cargo test --locked` must all be clean. CI enforces this on three operating systems.

## Commands

```bash
cargo run                                # debug build
cargo run --release -- path/to/file.md   # open a file directly
cargo test                               # 84 unit tests, about 1 s
cargo fmt && cargo clippy --all-targets --locked -- -D warnings
packaging/linux/build.sh 0.4.0           # AppImage + .tar.gz in dist/ (after a release build)
packaging/linux/install-local.sh         # install the release build, menu entry, and icon in ~/.local
```

Linux builds need GTK 3 headers (`rfd` uses GTK file dialogs).

Cross-checking with `cargo check --target x86_64-pc-windows-msvc` or `aarch64-apple-darwin`
no longer works from Linux: typst depends on `psm`/`stacker`, whose build scripts assemble a
small stack-switching routine with the target's toolchain. Rely on CI for Windows and macOS.

**Run the app without touching the user's real settings** by pointing the XDG folders at a
scratch directory:

```bash
XDG_CONFIG_HOME=/tmp/qw/cfg XDG_DATA_HOME=/tmp/qw/data cargo run -- /tmp/qw/book/chapters/001.md
```

## Repository map

| Path | What it is |
| --- | --- |
| `src/main.rs` | Entry point, `MarkdownApp` state, `AppConfig`, the frame loop (`update`), saving, disk sync, find/replace, lint/format tools, dialogs |
| `src/workspace.rs` | Menus, manuscript sidebar, status bar, home screen, the page, focus mode, keyboard shortcut list (`SHORTCUTS`) |
| `src/editor.rs` | Custom editor: edits the Markdown *source* while displaying a styled *projection*; selection, undo, IME, navigation |
| `src/live_preview.rs` | Builds the styled projection (hides syntax except on the active line) and maps display offsets to source offsets |
| `src/manuscript.rs` | Project index: chapters, characters, notes, titles, word counts, outline, new chapter/character files |
| `src/project.rs` | New Project scaffold from `assets/project-template/`; opening folders |
| `src/file_sync.rs` | Background file watcher (`notify` plus polling), reconciliation of disk vs editor, atomic checked writes, cursor mapping |
| `src/saving.rs` | Autosave, save-before-leaving, Version History window |
| `src/history.rs` | Version history storage (timestamped copies, retention) |
| `src/assistant.rs` | Assistant logic: presets, prompt building, story context, applying suggestions, background process runner (`Job`) |
| `src/assistant_ui.rs` | Assistant side panel; also `diff_job` (word diff rendering), reused by Version History |
| `src/export/mod.rs` | `export()` entry point: options, `metadata.yaml`, reference styles, `Format`, `PageSize`, `ExportConfig` |
| `src/export/book.rs` | Document model (`Book`, `Chapter`, `Block`, `Inline`), the pulldown-cmark parser, image loading, `metadata.yaml` reading |
| `src/export/html.rs`, `epub.rs` | Web page and EPUB 3 writers (shared XHTML writer and CSS) |
| `src/export/docx.rs`, `odt.rs` | Word (docx-rs) and OpenDocument (hand-written XML) writers |
| `src/export/pdf.rs` | PDF: the book as typst markup, compiled by typst used as a library |
| `src/export/xml.rs` | XML escaping and zip package helpers |
| `src/export_ui.rs` | Export Manuscript window |
| `src/spelling.rs` | Spell checking: finding and loading Hunspell dictionaries (spellbook), word tokenizing, the project's `dictionary.txt`, check cache, suggestions |
| `src/spelling_ui.rs` | Editor right-click menu, View → Check Spelling / Spelling Language, refreshing the language and word list with the index |
| `src/reference/mod.rs` | Research files: kinds by extension, background loading (text, images, PDFs), copying files into `research/` without overwriting |
| `src/reference/pdf.rs` | PDF `Document`: a hayro render thread that owns the parsed file and renders requested pages |
| `src/pdf_view.rs` | PDF viewer in the Reference panel: stacked pages, visible-page requests, texture cache, page jump, zoom |
| `src/reference_ui.rs` | Reference side panel (read only), Add Research Files, `open_with_system` |
| `src/theme.rs` | Paper / Sepia / Night palettes and egui styling |
| `src/fonts.rs` | Bundled fonts (Literata, Atkinson Hyperlegible) and system font fallback |
| `src/process.rs` | `command()`: `std::process::Command` without a console window on Windows |
| `assets/project-template/` | Files created by New Project (embedded with `include_str!`) |
| `assets/fonts/`, `assets/icon/` | Embedded fonts and the app icon (SVG source plus generated PNG/ICO) |
| `assets/dictionaries/` | The bundled en_US Hunspell dictionary (SCOWL, from LibreOffice) and its copyright notice |
| `packaging/`, `.github/workflows/` | Release packaging scripts and CI / release workflows |
| `docs/config.example.toml` | Documented example config; keep it in sync with the config structs |
| `build.rs` | Embeds the icon in the Windows executable |

## Architecture

### One app struct, many `impl` blocks

`MarkdownApp` (in `main.rs`) holds all state. Features add `impl MarkdownApp` blocks in their
own modules (`workspace.rs`, `saving.rs`, `assistant_ui.rs`, `export_ui.rs`) and keep their
private state in a sub-struct field (`assistant`, `saving`, `export`). Follow this pattern for
new features instead of growing `main.rs`.

### The frame (`eframe::App::ui` for `MarkdownApp`)

eframe hands the app a root `Ui`. Panels (`egui::Panel`, `CentralPanel`) are shown inside that
`Ui`, so `show_menu_bar`, `show_status_bar`, `show_sidebar`, and `show_assistant_panel` take it;
windows and areas are still shown on the `Context`. The renderer is glow (OpenGL), as it has
always been: `Cargo.toml` turns off eframe's default wgpu, and lists `winit` directly only to
keep its default features (Wayland decorations, loading libwayland at runtime).

In order: apply theme → `sync_with_disk` (watcher reports) → `poll_assistant` → handle a
window close request (save first or ask) → keyboard shortcuts, which mostly set `pending_*`
flags → `run_pending_actions` → rebuild the manuscript index if `index_dirty` → panels (menu,
status bar, sidebar, assistant) → the page (home screen or editor) → windows and dialogs →
Version History → Export → `after_frame` (word counts, window title, `last_edit` time) →
`run_autosave`.

Actions that need egui's input lock released are deferred through `pending_*` flags.
Anything that would discard unsaved text goes through `run_or_prompt_discard(DiscardAction)`,
which saves silently when possible and otherwise asks.

### Editor model

`content` is the Markdown source; `original_content` is what we believe is on disk;
`modified` compares them (plus `base_missing`). The editor stores selections as **source
character indices**. `live_preview::project` produces a display `LayoutJob` and a mapping
between display and source offsets, so hidden syntax never creates invisible cursor stops.
Set `external_load = true` whenever the buffer is replaced by something other than typing
(open, reload, restore, accepted AI text), so session word counts stay honest.

### Disk sync

`file_sync::Monitor` runs on a background thread. It watches the project recursively, except
for broad folders (filesystem root, home: `file_sync::is_broad`), and reports a fresh read of
the open file. `apply_disk` decides: unchanged / accept / reload / conflict
(`file_sync::reconcile`). Conflicts open the "File Changed on Disk" dialog. Saving uses
`save_in_place`, which re-reads the disk and refuses to clobber a changed file.

### Saving and history

Autosave (`config.editor.autosave`, default on, `autosave_delay` 2 s) saves after a typing
pause, on focus loss, and before leaving a file or the app. It never saves untitled buffers,
files in conflict, or files deleted on disk. History copies go to
`<data dir>/quillwright/history/<stem>-<fnv hash of path>/<unix millis>.md`. A copy is made
before a save replaces the file (at most every 10 minutes, or always for a cut of at least a
quarter), before an external reload replaces the text, and before a restore. The newest 50
copies per file are kept.

### Assistant

The prompt contains the chapter with `⟦CURSOR⟧` or `⟦SELECTION START⟧`/`⟦SELECTION END⟧`
markers, `AGENT.md`, `world-rules.md`, and the sheets of characters named in the chapter. All
of it is truncated or windowed for size. `Job::run` streams stdout and stderr from any command
and supports cancel; export reuses it. Replacing a selection re-finds the original passage if
the buffer moved, and refuses when the passage is ambiguous.

### Spelling

`spelling::Speller` lives on `MarkdownApp`. `refresh_spelling` runs whenever the manuscript
index is rebuilt: it resolves the language (`metadata.yaml` `lang`, else the config default,
else en_US; the language menu writes `lang:` into `metadata.yaml` for a book, and into the
config only outside a project),
loads the dictionary on a background thread (`poll_spelling` picks it up), and reads the
project's `dictionary.txt` plus the character names. Dictionaries are searched in the user
data folder (`<data dir>/quillwright/dictionaries`), the system Hunspell folders, and the
bundled en_US. `live_preview::project_checked` underlines unknown words; code, links, raw
HTML, and the word that ends at the caret are skipped. Results are cached per word, so
checking every frame is cheap. The editor records the source index of a right click
(`Editor::context_index`), and suggestions are applied with `Editor::replace_chars`, which
keeps undo working. Added words are appended to `dictionary.txt` (plain text, visible to
agents and Git); ignored words last for the session.

### Reference material

Research files live in the project's `research/` folder (`manuscript::RESEARCH_DIR`;
`Index::research` lists them recursively). They open in the Reference panel
(`reference_ui::ReferenceState`), which is **read only** and separate from the edited
document: nothing there goes through saving, autosave, or history. `reference::load_in_background`
reads text or decodes images (the `image` crate, with the same features typst enables) on a
thread; `poll_reference` turns the result into a texture. `refresh_reference` runs after each
index rebuild and reloads the file if its size or modification time changed, or closes the
panel when another project is opened. In All Files, images and PDFs open in the panel. The
design and next steps (search) are in `docs/reference-material.md`.

PDFs: hayro's parsed document and caches borrow each other and use `Rc`, so
`reference::pdf::Document` keeps them on one thread for the life of the view. Each frame,
`PdfView` lists the visible pages plus one on each side (`Key { page, width }`, the width
rounded up to 128 px) and calls `Document::exchange`, which collects finished pages and
*replaces* the request list, so pages that scrolled away are never rendered. Textures are
kept within 6 pages of the current one. A hayro panic on one page is caught and that page
shows a message. Remember to `request_repaint` when results arrive: the frame that receives
them has already painted placeholders.

### Export

`export_ui` collects the chapters in sidebar order (the editor buffer for the open chapter) and
runs `export::export` on a background thread; the result is written atomically next to the
destination. `book::Book::build` parses each chapter with pulldown-cmark (the editor's parser,
with smart punctuation) into a small tree shared by every format. Each level-1 heading starts
a chapter; text before the first one becomes a chapter named after the file. Thematic breaks
are scene breaks. Raw HTML is dropped; images are read from `chapters/` or the project root.
Values typed in the window override `metadata.yaml`, which only fills empty fields.

Writers: HTML (one page, images as data URIs) and EPUB 3 (plus `toc.ncx`) share
`html::Writer`; DOCX uses docx-rs with pandoc-compatible style IDs, so a `reference.docx`
replaces `word/styles.xml`; ODT is hand-written XML, and a `reference.odt` replaces
`styles.xml`; PDF writes typst markup in which **all author text is a typst string literal**
(`pdf::string`), so nothing in the manuscript is interpreted as typst code. The typst `World`
serves only the generated source, the bundled Literata fonts, and the book's images.

### Configuration

`AppConfig` is TOML at `ProjectDirs("com", "quillwright", "quillwright")`: on Linux
`~/.config/quillwright/config.toml`. Every config struct uses `#[serde(default)]`, so old
files keep loading; keep that when adding fields. An md-echo config is imported once if no
Quillwright config exists. When you add an option, update `docs/config.example.toml` and the
README.

## Testing

- Unit tests live next to the code (`#[cfg(test)] mod tests`). `editor.rs` has a `Harness`
  that drives a headless `egui::Context` with synthetic events. Headless frames must clear
  `textures_delta` (or call `drop_without_applying_deltas`), or egui panics in debug builds.
- **Tests must pass on Linux, Windows, and macOS.** Lessons learned:
  - egui changes shortcuts per OS (on macOS, Ctrl+key are Emacs bindings and Ctrl+End does
    nothing). Use `Modifiers::COMMAND` combinations that work everywhere. To check locally,
    temporarily add `ctx.set_os(egui::os::OperatingSystem::Mac)` (or `Windows`) in
    `Harness::new` and run the tests.
  - Command-line tools differ: BSD `sleep` on macOS takes a single argument. Windows CI has
    Git's GNU tools on PATH.
  - Leave generous timeouts for process and file-watch tests; CI machines are slow.
- **Manual GUI testing on Linux** works with `xdotool` and ImageMagick `import` against an
  X11 window. Run the app with `env -u WAYLAND_DISPLAY` so it uses XWayland, isolate
  `XDG_CONFIG_HOME`/`XDG_DATA_HOME`, then activate the window (`xdotool windowactivate
  --sync`) before typing. The first click after launch often only focuses the window, and
  native file dialogs are separate windows (find them with `xdotool search --name`).
- On Wayland the window icon passed in code is ignored: the compositor matches the app id
  (`quillwright`, set in `main.rs`) to an installed `quillwright.desktop` and its themed
  icon. Run `packaging/linux/install-local.sh` to see the icon during development.
- Window icons on X11 must be small (the app uses `assets/icon/quillwright-128.png`; 512 px
  silently fails).

## Release

See `docs/RELEASING.md`. In short: bump `version` in `Cargo.toml`, run `cargo build` to update
`Cargo.lock`, commit, push, wait for CI, then push a `vX.Y.Z` tag matching the version. The
release workflow builds an AppImage and `.tar.gz` (Ubuntu 22.04), a Windows `.zip`, and a
universal macOS `.dmg` (ad-hoc signed, not notarized), and attaches them to a **draft**
release that a maintainer publishes.

Without `gh` authentication, the GitHub API allows 60 requests per hour and does not serve job
logs. Poll gently.

## Leftovers from md-echo (safe to clean up, not yet removed)

- `crates/egui_commonmark/`: a vendored crate that is not part of the build.
- `docs/config.toml`: an old md-echo config, superseded by `docs/config.example.toml`.
- `assets/mdecho-screengrab.png`: an old screenshot, not referenced.
- `.cargoignore`: not a Cargo feature (Cargo uses `exclude` in `Cargo.toml`).

## Suggested next steps

Roughly in priority order. Each one should keep the ground rules above.

A longer-term idea, *project kinds* (Novel, Legal case, ...) sharing one core, is described in
`docs/project-kinds.md`.

1. **Verify the 0.4.0 packages on real Windows and macOS machines.** Check the icon, the
   absence of a console window, file dialogs, the first-launch working folder (should be the
   home folder), exporting every format (PDF uses typst in-process), and running the assistant.
2. **Reference material, steps 3 to 5** (`docs/reference-material.md`): PDF text
   extraction and search with highlights, project-wide search, and "Use as Context" for
   the assistant.
3. **Review agent changes (assistant "phase 2").** When an external change reloads the open
   chapter, highlight the changed passages and offer accept or revert per passage. The pieces
   exist: the history copy taken before reload (`reload_from_disk`), word diffs
   (`assistant_ui::diff_job`, `similar`), and display mapping in `live_preview`. This is the
   project's main differentiator.
4. **Spell checking polish.** A wavy underline (painted over the galley), bundling more
   dictionaries (pt_BR and es are a few MB each; check each license), grammar for English
   through `harper-core`, and checking the whole manuscript from the sidebar.
5. **Render Markdown in assistant answers.** Continuity notes currently show raw `**` and `-`.
   Reuse `live_preview` styling for read-only text, as the Reference panel already does
   (`live_preview::project` with no selection hides all syntax).
6. **Update GitHub Actions** that still target Node.js 20 (`actions/checkout`,
   `upload-artifact`, `download-artifact`, `softprops/action-gh-release`). Read each
   changelog before bumping a major version.
7. **Manuscript navigation:** drag to reorder chapters (renumbering the `001-` prefixes),
   search across the whole project, and a word count per character or POV.
8. **Export polish:** fonts beyond Literata's Latin, Greek, and Cyrillic coverage in PDFs
   (load system fonts lazily through `fontdb`), drop caps or small caps for chapter openings,
   running headers in PDF, page numbers in the ODT table of contents, and validating EPUBs
   with epubcheck in CI.
9. **Code signing and notarization** for Windows and macOS once certificates exist (see
   `docs/RELEASING.md`), plus distribution through Flathub, the AUR, and Homebrew.
10. **Dependency upgrades:** `rfd` 0.11 pulls `block 0.1.6`, which triggers a
   future-incompatibility warning on macOS. When upgrading egui again, remember that the
   custom editor relies on egui text internals (`Galley`, `CCursorRange`, cursor movement,
   IME events), so expect API changes and re-run the editor tests on all three OS modes.
11. **Clean up the md-echo leftovers** listed above, and consider splitting `main.rs`
    (find/replace and lint/format tools could move to their own modules).

## Known limitations (user-facing)

Documented in the README ("Known limitations" and "Exporting the manuscript"): spell
checking without grammar, tables and images are not laid out in the editor, version history only covers files
opened in the app (renaming a file starts a new history), and the Word table of contents must
be updated by the user after export.
