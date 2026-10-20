# Quillwright

**A focused writing environment for novels and long-form fiction — plain Markdown, built to work alongside AI agents.**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org)

---

Quillwright is a native desktop app, written in Rust with [`egui`](https://docs.rs/egui), that treats a book as a folder of Markdown files: one file per chapter, one per character, and a document for the rules of your world. It gives you a calm, book-like editing surface with live Markdown styling, and it stays in sync with the disk, so an AI agent (Claude Code, Codex, Aider, or any other tool) can work on the same project while you write.

![Quillwright with the Paper theme: manuscript sidebar, chapter outline, and the page](screenshots/paper.png)

Your work is never locked into a proprietary format. Everything is plain UTF-8 Markdown that you can version with Git, open in any editor, or hand to any tool.

## Highlights

- **Writing projects.** *New Project* creates a ready-to-use structure for a book — chapters, character sheets, world rules, and an `AGENT.md` guide for AI collaborators. *Open Project* opens any existing folder as-is.
- **Manuscript sidebar.** Chapters are listed in reading order by their real titles, with word counts. Below them, an outline of the current chapter's headings (click to jump), the characters, and project notes. **+** buttons create the next numbered chapter or a new character sheet. The full file tree is still one click away.
- **A page, not a text box.** Live Markdown styling with a serif typeface (Literata by default) on a centered column. Markdown syntax is hidden except on the line you are editing. Scene breaks (`***`, `---`) are drawn as a centered ⁂.
- **Writing themes.** *Paper* (warm light), *Sepia*, and *Night* (soft dark), switchable from **View → Theme**. The page is set apart from the "desk" around it.
- **Focus tools.** **Focus mode** (F11) goes full screen and hides everything except the text. **Typewriter scrolling** keeps the line you are writing in the middle of the screen. **Dim other paragraphs** fades everything except the current paragraph.
- **Progress you can see.** The status bar shows words in the chapter, reading time, words in the whole manuscript, words selected, and the words you have written this session. Set a **session goal** to get a progress bar.
- **A project home.** The start screen continues the chapter you were writing and lists your recent projects.
- **AI assistant panel (Ctrl+K).** Rewrite a selection, continue a scene, get three alternatives, or check a chapter against your world rules and character sheets, using the AI tool you already have (Claude Code, Codex, a local Ollama model, or any command). Suggestions are shown as a diff and never touch the manuscript until you accept them.
- **Agent-aware file sync.** Quillwright watches the project folder. When an agent (or any other program) changes a file, the sidebar updates immediately and the open chapter reloads automatically, keeping your cursor near where it was. If you have unsaved edits, you are asked whether to reload or keep your version — nothing is merged or overwritten silently.
- **Export the manuscript** to Word, EPUB, PDF, OpenDocument, or a web page, built in with nothing else to install: chapters in order, a title page, an optional table of contents, each chapter on a new page, and scene breaks set as a centered *\* \* \**.
- **Autosave and version history.** Your chapter is saved a moment after you stop typing, when you switch to another window, and before you close the file or the app. Earlier versions of each file are kept on your computer, and **File → Version History...** shows what changed and restores any of them.
- **Safe saving.** Files are saved atomically (written to a temporary file, then renamed), so agents never read a half-written chapter. Saving, including autosave, refuses to overwrite a file that changed on disk since you opened it, and asks you instead.
- **Typography for long reading.** Literata and [Atkinson Hyperlegible](https://github.com/googlefonts/atkinson-hyperlegible) are bundled, or use any installed font. Text size changes live (Ctrl+] / Ctrl+[).
- **Optional line numbers**, find & replace, lint & format with configurable external tools (defaults to [`rumdl`](https://github.com/fibnas/rumdl)), and unsaved-changes protection everywhere.

![The Night theme](screenshots/night.png)

## A writing project

**File → New Project...** asks for a folder name and a parent directory, then creates:

```text
my-novel/
├── AGENT.md                  # How AI agents should collaborate on this book
├── world-rules.md            # Setting, rules, chronology, open questions
├── chapters/
│   └── 001-opening.md        # One file per chapter, ordered by prefix
└── characters/
    └── example-character.md  # Character sheet template
```

- **Chapters** — one Markdown file each. Numeric prefixes (`001-`, `002-`, …) set the reading order; the chapter title is the file's first heading.
- **Characters** — one sheet per character: identity, motivation, voice, relationships, and how they change over the story (with the chapter where each change happens).
- **World rules** — what is possible and impossible, costs and consequences, established facts, timeline, narrative conventions, and the decisions still pending.

Chapter titles come from each file's first heading, so renaming a chapter is just editing its heading. Use **+** in the sidebar (or **Ctrl+Shift+N**) to add the next chapter — it gets the next number automatically.

The starter files are templates with empty fields, not story content. Everything can be renamed, reorganized, or deleted: there is no manifest, database, or required metadata. Existing folders are never overwritten, and *Open Project* never adds or moves files.

The last opened project is remembered between sessions.

## Saving and version history

**Autosave** is on by default. The open file is saved about two seconds after you stop typing, as soon as the window loses focus, and before you open another file, switch projects, or quit. The status bar shows *Edited* while a save is pending and *Saved* afterwards. Turn it off with **File → Autosave**; you are then asked before closing with unsaved changes. Untitled documents are never autosaved: use **Save As** once to give them a file.

**Version history** keeps earlier versions of each file you edit in Quillwright:

- before a save replaces the file (at most one copy every 10 minutes, but always before a cut that removes a quarter of the text or more);
- before a change made by an agent replaces the text in the editor (both the last saved version and any unsaved edits);
- before you restore an older version, so a restore can be undone the same way.

**File → Version History...** lists the copies of the open file with their date and word count, and shows each one compared with the text in the editor. **Restore This Version** puts it back in the editor; autosave then writes it to disk.

Copies live outside the project, in `~/.local/share/quillwright/history/` on Linux (one folder per file, with a `source.txt` naming the original path), so agents and Git never see them. The 50 newest copies of each file are kept. History is a safety net for recent work, not a backup: keep the project under Git or a backup tool as well.

## Exporting the manuscript

**File → Export Manuscript...** (Ctrl+Shift+E) joins your chapters in sidebar order and writes the book in the format you choose. Everything is built into Quillwright: there is nothing else to install, and nothing is sent anywhere.

| Format | Good for |
| --- | --- |
| Word (.docx) | Editors, agents, and Google Docs |
| EPUB (.epub) | E-readers and beta readers |
| PDF (.pdf) | Printing and sharing a fixed layout |
| OpenDocument (.odt) | LibreOffice |
| Web page (.html) | A single self-contained file |

The export:

- starts with a title page from the **Title** and **Author** you enter (remembered per project), and an optional table of contents;
- starts each chapter on a new page. A chapter file that does not begin with a `# Heading` gets one from its title in the sidebar;
- turns scene breaks (`***` or `---`) into a centered *\* \* \** instead of a horizontal line;
- uses curly quotes and real dashes, and keeps footnotes, tables, and images (paths relative to the chapter or the project);
- includes unsaved edits in the open chapter, so you never need to save first.

The window also asks for the book's **language** (`en`, `pt-BR`, ...), used for hyphenation, the title of the table of contents, and e-reader metadata, and a **page size** for PDF, Word, and OpenDocument: A5 (the default), trade paperback (6 × 9 in), A4, or US Letter. PDFs are laid out with [typst](https://typst.app), built in, in justified Literata with hyphenation and page numbers. Word and OpenDocument files use indented paragraphs without space between them, as editors expect.

**Customising the result**, with optional files in the project root:

- `metadata.yaml`: book details, for example:

  ```yaml
  title: The Salt Road
  subtitle: A Novel
  author: Ana Lima
  lang: pt-BR
  date: 2026
  rights: © 2026 Ana Lima
  description: A short blurb for e-readers.
  ```

  The title, author, and language typed in the export window take precedence.
- `reference.docx` / `reference.odt`: a document whose styles (fonts, sizes, headings) replace the built-in ones in Word or OpenDocument exports. Export once, adjust the styles in your word processor (*Body Text*, *First Paragraph*, *Heading 1*, *Title*, ...), and save the file in the project root under that name. Style files made for pandoc work too.

In Word exports, the table of contents is filled in when the file is opened: Word asks whether to update the fields (choose *Yes*); in LibreOffice, use *Tools → Update → Update All*. OpenDocument tables of contents list the chapters but get page numbers only after *Update All*. EPUB, PDF, and HTML tables of contents are complete.

Images are included when they are PNG, JPEG, or GIF files in the project; images on the web are never downloaded. Anything left out is listed in the export window.

## Writing with AI agents

There are two ways to work with AI in Quillwright, and both use tools you already have rather than an API key stored in the app:

- the **assistant panel** (Ctrl+K) for quick, focused requests about the chapter you are writing;
- a **full agent** running in a terminal next to the app, for larger work across many files.

### The assistant panel

Press **Ctrl+K** (or **Assistant → Assistant Panel**) to open the panel beside the page. It offers four writing actions plus a free-form request:

| Action | What it sends | What comes back |
| --- | --- | --- |
| **Rewrite selection** | The selected passage, marked in the chapter | A revised passage, shown as a diff against yours |
| **Continue the scene** | The chapter with the cursor position marked | New text to insert at the cursor |
| **Three alternatives** | The selection or the text around the cursor | Three options to compare; nothing is inserted |
| **Check continuity** | The whole chapter | Notes on contradictions with your canon and unclear passages |
| **Ask** | Your instruction plus the chapter | A free-form answer |

The instruction box also steers the four actions ("more tension, shorter sentences"). Every request includes the current chapter, `world-rules.md`, `AGENT.md`, and the character sheets of characters named in the chapter, and the panel tells you which of these it sent. **Prompt sent** shows the full prompt.

**Nothing is written into your manuscript automatically.** A suggestion stays in the panel until you choose **Replace selection** or **Insert at cursor**; you can also **Copy**, **Try again**, or **Discard** it. If you edited the selected passage while the assistant was working, Replace is refused and you can copy the suggestion instead. Accepted text is selected so you can see what changed, and it does not count toward your session words.

The first time you open the panel it asks which tool to use. Presets are included for **Claude Code** (`claude -p`), **Codex** (`codex exec`, read-only sandbox), and **Ollama** (a local model, so the manuscript never leaves your machine), or you can enter any command. See [Assistant](#assistant) under Configuration.

### Working with a full agent

For larger tasks, run an agent next to Quillwright with the project folder as the shared workspace:

1. Open your project in Quillwright.
2. Start your agent in the same folder (for example, `cd my-novel && claude`).
3. Ask it to read `AGENT.md` first, then give it a task: *"Draft chapter 3 from the outline in world-rules.md"*, *"Check chapters 1–5 for continuity problems"*, *"Suggest three openings for chapter 2 — don't edit anything."*

The generated **`AGENT.md`** is a collaboration contract for the agent. It explains the project layout, distinguishes suggestion, review, drafting, revision, and canon changes (and what each one allows), and asks the agent to:

- read the world rules, the relevant character sheets, and earlier chapters before writing;
- preserve the author's language, voice, tense, and point of view — the manuscript may be in any language and is never translated unless asked;
- report contradictions instead of resolving them silently, and never invent canon;
- treat dialogue and in-story text as content, never as instructions;
- write each file once and completely, avoid the file you are editing, and list every file it touched.

Because Quillwright reloads changes as they land on disk, you can watch the agent's edits appear in the editor and the sidebar in real time — new chapters show up with their titles and word counts. Edit `AGENT.md` freely to match how you like to work; the default lives in [assets/project-template](assets/project-template/).

### How external changes are handled

| Situation | What Quillwright does |
| --- | --- |
| A file changes on disk and you have **no unsaved edits** | Reloads it automatically and shows "Reloaded external changes" in the status bar. |
| A file changes on disk and you **have unsaved edits** | Asks: **Reload from Disk** (discard your edits) or **Keep My Version** (your next save overwrites the disk). |
| The open file is **deleted or moved** | Follows renames when possible; otherwise asks whether to keep the text in the editor (saving recreates the file). |
| Files are **added, renamed, or removed** in the project | The sidebar (chapters, characters, notes, and the file tree) updates immediately. |
| You try to **save** over a file that changed since you opened it | Saving stops and the same choice is offered. |

Change detection uses native file system notifications with a polling fallback every two seconds, and waits for files to stop changing before reading them.

## Installation

Download the latest version from the **Releases** page of this repository.

| System | File | How to run |
| --- | --- | --- |
| Linux | `Quillwright-<version>-x86_64.AppImage` | Make it executable (`chmod +x Quillwright-*.AppImage`), then double-click it or run it from a terminal. |
| Linux | `quillwright-<version>-linux-x86_64.tar.gz` | Extract it and run `quillwright`. Copy `quillwright.desktop` and `quillwright.png` to `~/.local/share/applications/` and `~/.local/share/icons/` for a menu entry. |
| Windows | `quillwright-<version>-windows-x86_64.zip` | Extract it and run `quillwright.exe`. |
| macOS | `Quillwright-<version>-macos-universal.dmg` | Open it and drag Quillwright to Applications. Works on Apple Silicon and Intel Macs. |

The Linux builds need a desktop with GTK 3 (used for file dialogs) and glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36, or later).

**First launch on Windows and macOS:** the builds are not signed with a paid developer certificate yet, so the system warns you the first time.

- **Windows:** in the *Windows protected your PC* dialog, choose **More info → Run anyway**.
- **macOS:** right-click Quillwright in Applications and choose **Open**, then **Open** again. If macOS says the app is damaged, run `xattr -dr com.apple.quarantine /Applications/Quillwright.app` once in Terminal.

Every release includes a `SHA256SUMS.txt` file for checking downloads.

### Building from source

You need a recent stable Rust toolchain. On Linux, also install the GTK 3 development headers (used by the native file dialogs):

```bash
sudo dnf install gtk3-devel        # Fedora / RHEL
sudo apt install libgtk-3-dev      # Debian / Ubuntu
sudo pacman -S gtk3                # Arch
```

Then build and install:

```bash
git clone <repository-url> quillwright
cd quillwright
cargo install --path .
```

Or run it directly with `cargo run --release`. To publish a release, see [docs/RELEASING.md](docs/RELEASING.md).

## Usage

```bash
quillwright                       # open the last project
quillwright chapters/003-storm.md # open a specific file
```

If the file passed on the command line does not exist, an empty editor opens with that path ready for saving.

### Keyboard shortcuts

Press **Ctrl+H** in the app to see all shortcuts.

| Shortcut | Action |
| --- | --- |
| F11 (Esc to leave) | Focus mode |
| Ctrl+K | Assistant panel |
| Ctrl+Shift+E | Export manuscript |
| Ctrl+Enter | Send the request (in the assistant panel) |
| Ctrl+B | Show / hide the sidebar |
| Ctrl+Shift+T | Typewriter scrolling |
| Ctrl+Shift+D | Dim other paragraphs |
| Ctrl+] / Ctrl+[ | Larger / smaller text |
| Ctrl+Shift+N | New chapter |
| Ctrl+N / Ctrl+O | New blank document / Open file |
| Ctrl+S | Save |
| Ctrl+Shift+S or Alt+S | Save as |
| Ctrl+Z / Ctrl+Y | Undo / Redo |
| Ctrl+F / Ctrl+R | Find / Replace |
| Ctrl+D | Delete line |
| Page Up / Page Down | Move one page |
| Ctrl+Shift+L / Ctrl+Shift+F | Lint / Format Markdown |
| Ctrl+Q | Quit |

Ctrl+Plus / Ctrl+Minus zoom the whole interface (egui's built-in zoom); Ctrl+] / Ctrl+[ change only the manuscript text.

## Configuration

Quillwright reads an optional config file at `~/.config/quillwright/config.toml` (if that does not exist yet, an existing `~/.config/md-echo/config.toml` is imported). Start from the example:

```bash
mkdir -p ~/.config/quillwright
cp docs/config.example.toml ~/.config/quillwright/config.toml
```

Most options can also be changed from the **View** menu; changes are saved automatically.

### Theme

```toml
[theme]
base = "night"          # "paper", "sepia", or "night" ("light"/"dark" also work)
# Optional overrides, applied on top of any theme:
# background = "#1F2026" # the page
# panel = "#16171B"      # sidebar, menus, status bar
# text = "#DCD7CE"
# accent = "#D9A066"     # cursor, selection, highlights
```

### Editor

```toml
[editor]
font_family = "Literata"   # or "Atkinson Hyperlegible", or any installed family
font_size = 18.0
line_height = 1.4          # multiple of font size
line_numbers = false       # show source line numbers in a gutter
max_width = 760            # maximum text column width, in points
centered = true            # center the column; false aligns it left
typewriter = false         # keep the current line vertically centered
dim_paragraphs = false     # fade paragraphs other than the current one
autosave = true            # save after a pause in typing (File → Autosave)
autosave_delay = 2.0       # seconds without typing before saving
```

- Line numbers count Markdown source lines, not wrapped display rows.
- Font names are matched case-insensitively. Missing families or styles fall back to the bundled Literata faces. Sizes are clamped to 6–96 and line height to 1–3.
- Restart the app after changing the font family. Text size can be changed live.

### View

```toml
[view]
sidebar = true             # manuscript sidebar (Ctrl+B)
session_goal = 0           # words per session; 0 hides the progress bar
```

Session words count only what you type in Quillwright (net of deletions); text written by an agent and reloaded from disk is not counted. The counter starts at zero each time the app starts.

### Lint & format tools

```toml
[tools]
lint = ["rumdl", "check"]
lint_use_open_file = false
format = ["rumdl", "fmt"]
format_use_open_file = false
```

Commands receive a temporary copy of the current buffer, or the open file itself when `*_use_open_file = true` (the file must be saved first). Output appears in the Tool Output window.

### Assistant

```toml
[assistant]
command = ["claude", "-p"]
# command = ["codex", "exec", "--skip-git-repo-check", "--sandbox", "read-only", "-"]
# command = ["ollama", "run", "llama3.1"]
prompt_as_argument = false
```

The command runs in the project folder. The prompt is written to its standard input, or appended as the last argument when `prompt_as_argument = true`; the answer is read from standard output. Any program that works this way can be used, including your own scripts. The assistant can also be configured from **Assistant → Assistant Settings...**. Quillwright stores no API keys: authentication is handled by the tool you choose.

### Export

```toml
[export]
language = ""              # "en", "pt-BR", ...; empty uses metadata.yaml, or English
page_size = "a5"           # "a5", "trade" (6 × 9 in), "a4", or "letter"
```

The export window also stores your last format, author, table-of-contents choice, book titles per project, and the last export folder here.

## Known limitations

The live editor is a custom surface over the plain Markdown buffer, not a full rich-text renderer:

- Images show their alt text; tables are not laid out as grids; HTML and entities are not rendered.
- Revealing the source of the active line can change wrapping and heading height.
- The whole document is re-parsed while typing, so very large single files may feel slower — one file per chapter keeps things fast.
- External changes are never merged with unsaved edits; you choose one version.
- Variable-font axes and switching font families without a restart are not supported.
- Paragraph dimming only applies while the page has keyboard focus.
- There is no spell checker yet.
- Version history only covers files open in Quillwright. Changes an agent makes to other files are not copied, and renaming a file starts a new history for it.

## Tech stack

- [`eframe`](https://docs.rs/eframe) / [`egui`](https://docs.rs/egui) — native UI
- [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) — Markdown parsing
- [`notify`](https://crates.io/crates/notify) — file system watching
- [`similar`](https://crates.io/crates/similar) — diffing to keep the cursor in place on reload
- [`fontdb`](https://crates.io/crates/fontdb) / [`ab_glyph`](https://crates.io/crates/ab_glyph) — font discovery and loading
- [`rfd`](https://crates.io/crates/rfd) — native file dialogs

Bundled fonts are licensed under the SIL Open Font License 1.1; see [font sources and checksums](assets/fonts/README.md) and [licenses](assets/fonts/licenses/).

## Credits

Quillwright started as a fork of [md-echo](https://github.com/fibnas/md-echo) by Frank Stallion, a minimalist Markdown editor, and keeps its MIT license.

## License

[MIT](./LICENSE)
