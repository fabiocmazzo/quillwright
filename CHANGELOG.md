# Changelog

All notable changes to Quillwright are listed here. Versions follow
[semantic versioning](https://semver.org).

## Unreleased

- Fixed: a long project name in the sidebar ran under the **Open…** and **Home** buttons.
  It is now cut short with "…" (hover it for the full path).

## 0.5.0 (2026-10-22)

- **Research material.** A `research/` folder in the project is listed in the sidebar, and
  its files open read only in a new **Reference panel** beside the page: plain text,
  Markdown notes, and PNG, JPEG, GIF, and WebP images. **File → Add Research Files...**
  copies files in without replacing any. Other files open with the system viewer. Images and PDFs in **All Files** open in the panel too.
- **PDF viewer.** PDFs open in the Reference panel, rendered in pure Rust by hayro on a
  background thread, one page at a time as you scroll. Jump to a page, zoom in and out,
  and keep your page when the file changes on disk. Password-protected PDFs are not
  supported yet.
- **Project search.** Ctrl+Shift+H searches the chapters, character sheets, notes, and
  research files (PDFs included) from the sidebar, ignoring case and accents. Results show
  the line or page with some context; a click opens the file with the match selected, or
  the PDF at the match.
- **Search in PDFs.** Matches are highlighted on the page and you step through them with
  Enter. Search ignores case and accents and joins words hyphenated across lines. The text
  is extracted in the background and cached in the app's data folder.
- **Research as assistant context.** Select a passage in the Reference panel (text,
  Markdown, or a PDF page, by dragging) and choose **Use as Context**: it goes to the
  assistant with its citation (`research/book.pdf, p. 42`) until you remove it.
- **Legal case projects.** New Project can create a *Legal case*: filings, party sheets,
  facts, timeline, evidence list, and client questions, with assistant actions to draft
  from the facts, check a filing against them, and list questions for the client. The
  template comes in English or Portuguese (Brazil). Its `AGENT.md` tells agents never to
  invent facts or citations, and how to split huge court PDFs safely.
- New Project asks for the kind of project and, when it has several, the language of the
  template.
- The app id is now `io.github.fabiocmazzo.quillwright`, as Flathub requires: the desktop
  file and icons are named after it, and so is the window class. Run
  `packaging/linux/install-local.sh` again after updating; it removes the old files.
- A MetaInfo file (`packaging/linux/io.github.fabiocmazzo.quillwright.metainfo.xml`)
  describes the app for software stores, and the AppImage includes it.
- Inside a Flatpak, the assistant and lint tools run the programs installed on your
  system through `flatpak-spawn --host`.
- A snap package definition (`snap/snapcraft.yaml`), built and started in CI.
- Fixed: on Windows, the assistant's prompt named sheets with backslashes
  (`characters\mara.md`).
- Internal: the novel layout (folders, sidebar labels, the assistant's context files,
  actions, and prompts) is now described in `assets/kinds/novel/kind.toml`, the first
  *project kind*. Nothing changes for novels; this prepares other kinds of writing.
- The sidebar sorts names naturally: `note-2` comes before `note-10`, and chapters
  numbered without leading zeros (`1-`, `2-`, `10-`) are in reading order.
- The sidebar's scroll bar no longer covers the **+** buttons.
- **The theme follows the system** by default: Paper in light mode and Night in dark mode,
  switching when the system does. **View → Theme → Match System** brings it back after
  choosing a theme. A theme chosen before keeps applying.
- Fixed: the AppImage stopped at startup on systems without `libxkbcommon-x11` (it said
  "Library libxkbcommon-x11.so could not be loaded"). It now carries that library and the
  other X11 libraries the window toolkit loads at run time.

## 0.4.0 (2026-10-20)

- **Spell checking.** Misspelled words are underlined as you write, and a right-click
  offers suggestions, "Add to Project Dictionary", and "Ignore for This Session". It uses
  Hunspell dictionaries through spellbook, in pure Rust and offline. English is built in;
  other languages come from the system or from the app's dictionaries folder (**View →
  Spelling Language → Open Dictionaries Folder**).
- Each book is checked in its own language, the `lang` in its `metadata.yaml`. **View →
  Spelling Language** sets it there, keeping the rest of the file.
- Character names from the character sheets are accepted automatically, and added words go
  to a plain `dictionary.txt` in the project.
- New `[spelling]` config section (`enabled`, `language`).

## 0.3.0 (2026-10-20)

- Upgraded the interface toolkit (egui and eframe) from 0.27 to 0.37. Text is rendered
  noticeably better, and the update brings many releases of fixes to text input, IME,
  accessibility, and window handling. Layout, themes, and shortcuts are unchanged.

## 0.2.0 (2026-10-20)

### Export is now built in

- Export to Word, EPUB, PDF, OpenDocument, and HTML no longer needs pandoc or a PDF engine.
  It is part of Quillwright, works offline, and never downloads anything.
- PDFs are laid out with [typst](https://typst.app) inside the app: justified Literata text
  with hyphenation, a title page, a table of contents with page numbers, and page numbers
  starting at the first chapter.
- The export window has new **Language** and **Page size** fields (A5, 6 × 9 in, A4, US
  Letter).
- Word and OpenDocument files use novel-style paragraphs: indented, with no space between
  them, and the first paragraph after a heading or scene break not indented.
- Exports are written atomically, so a failed export never leaves half a file behind.
  Missing images are listed in the export window.
- `metadata.yaml`, `reference.docx`, and `reference.odt` keep working, including style files
  made for pandoc.
- The `pandoc` and `pdf_engine` config options were removed; `language` and `page_size` were
  added.

### Other changes

- `packaging/linux/install-local.sh` installs Quillwright for your user, with its menu entry
  and icon (needed for the icon on Wayland desktops).

## 0.1.0 (2026-10-19)

First release.

- Writing projects: chapters, character sheets, world rules, and an `AGENT.md` guide for AI
  collaborators.
- Manuscript sidebar, live Markdown styling on a book-like page, Paper, Sepia, and Night
  themes, focus mode, and typewriter scrolling.
- Word counts, session goals, and reading time.
- AI assistant panel that runs Claude Code, Codex, Ollama, or any command, and shows
  suggestions as diffs to accept or reject.
- Safe reloading when agents change files on disk.
- Autosave and version history.
- Export through pandoc.
- Packages for Linux (AppImage and .tar.gz), Windows, and macOS.
