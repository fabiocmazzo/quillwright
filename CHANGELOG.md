# Changelog

All notable changes to Quillwright are listed here. Versions follow
[semantic versioning](https://semver.org).

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
