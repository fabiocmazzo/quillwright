# Quillwright's interface texts, in English: the source for every other language.
#
# Each language has a copy of this file in its own folder (assets/locales/<lang>/), with
# the same message ids. A missing message falls back to English, and a test checks that
# every language has all of them. A project kind's own texts (its name, section labels,
# and assistant actions) are in assets/kinds/<kind>/locales/<lang>.ftl.
#
# Variables: $count is a number and chooses plural forms; $words is the same number with
# digit groups ("12,345"), ready to show. $document is the noun for one of the kind's
# documents ("chapter"), and $gender its grammatical gender (masculine, feminine, or
# other), for languages whose sentences agree with it.
# Syntax: https://projectfluent.org/fluent/guide/

## Counts

words = { $count ->
    [one] { $words } word
   *[other] { $words } words
}
# Short word counts in the sidebar: "12.3k".
thousands = { $count }k

## Menus

menu-file = File
menu-view = View
menu-assistant = Assistant
menu-tools = Tools
menu-help = Help
menu-lint = Lint Markdown
menu-format = Format Markdown
menu-shortcuts = Keyboard Shortcuts
menu-add-research = Add Research Files...
    .hint = Copy PDFs, images, and notes into the project's research folder
menu-new-project = New Project...
menu-open-project = Open Project...
menu-project-home = Project Home
menu-new-blank = New Blank Document
menu-open-file = Open File...
menu-save = Save
menu-save-as = Save As...
menu-autosave = Autosave
    .hint = Save the open file a moment after you stop typing
menu-history = Version History...
    .hint = Earlier versions of this file, kept on this computer
menu-export = Export Manuscript...
menu-exit = Exit
menu-assistant-panel = Assistant Panel
menu-assistant-settings = Assistant Settings...
menu-theme = Theme
menu-theme-system = Match System
menu-focus = Focus Mode
menu-sidebar = Sidebar
menu-search = Search Project...
menu-reference = Reference Panel
    .hint = The research file you opened last
menu-typewriter = Typewriter Scrolling
    .hint = Keep the line you are writing in the middle of the screen
menu-dim = Dim Other Paragraphs
    .hint = Fade everything except the paragraph you are writing
menu-line-numbers = Line Numbers
menu-larger = Larger Text
menu-smaller = Smaller Text
menu-default-size = Default Text Size
menu-session-goal = Session goal (words)
    .hint = 0 hides the goal
menu-language = Interface Language
menu-language-system = Match System

## Keyboard shortcuts (Help → Keyboard Shortcuts)

shortcuts-title = Keyboard Shortcuts
shortcut-focus = Focus mode (Esc to leave)
shortcut-sidebar = Show / hide sidebar
shortcut-assistant = AI assistant panel
shortcut-export = Export manuscript
shortcut-send = Send the assistant request
shortcut-typewriter = Typewriter scrolling
shortcut-dim = Dim other paragraphs
shortcut-text-size = Larger / smaller text
shortcut-new-document = New chapter
shortcut-blank = New blank document
shortcut-open = Open file
shortcut-save = Save
shortcut-save-as = Save as
shortcut-undo = Undo / Redo
shortcut-find = Find / Replace
shortcut-search = Search the whole project
shortcut-delete-line = Delete line
shortcut-page = Move one page
shortcut-lint = Lint / Format Markdown
shortcut-help = Show / hide shortcuts
shortcut-quit = Quit

## Sidebar

sidebar-home = Home
    .hint = Project home
sidebar-open = Open…
    .hint = Open another project
sidebar-outline = Outline
sidebar-notes = Notes
sidebar-add-research = Add research files
sidebar-all-files = All files
sidebar-folder-unavailable = This folder is unavailable.
create-file-failed = Could not create the file: { $error }

## Status bar and window title

status-edited = Edited
    .autosave = Saves automatically when you pause
    .unsaved = Unsaved changes (Ctrl+S to save)
status-saved = Saved
status-not-saved = Not saved yet
status-goal = Session goal: { $words } of { $goal } { $count ->
    [one] word
   *[other] words
}
status-session = { $words } this session
    .hint = Session words count only what you type here
status-manuscript = { $words } in manuscript
# "1,234 words in chapter": $document is the kind's noun ("chapter", "filing").
status-words-in = { $count ->
    [one] { $words } word in { $document }
   *[other] { $words } words in { $document }
}
status-reading = { $minutes } min read
status-selected = { $words } selected
status-line = Ln { $line }
title-home = Home
title-untitled = Untitled

## Home screen

home-tagline = A quiet room for long stories.
home-current-project = Current project
home-continue = Continue writing  “{ $title }”
home-new-project = New Project…
home-open-project = Open Project…
home-blank-page = Blank Page
home-recent = Recent projects
home-keys = F11 focus mode   ·   Ctrl+B sidebar   ·   Ctrl+H all shortcuts

## Focus mode

focus-goal = { $words } / { $goal } this session
focus-unsaved = unsaved
focus-hint = Focus mode — press F11 or Esc to leave

## Buttons used in several places

button-save = Save
button-discard = Discard
button-cancel = Cancel
button-close = Close
button-create = Create
button-save-and-exit = Save and Exit
button-discard-and-exit = Discard and Exit

## Unsaved changes

discard-title = Unsaved Changes
discard-exit = You have unsaved changes. Save before exiting?
discard-new-document = You have unsaved changes. Save before starting a new document?
discard-home = You have unsaved changes. Save before going to the project home?
discard-open-file = You have unsaved changes. Save before opening another file?
discard-create-project = You have unsaved changes. Save before creating project { $path }?
discard-open-project = You have unsaved changes. Save before opening project { $path }?
discard-open = You have unsaved changes. Save before opening { $name }?

## File changed on disk

conflict-title = File Changed on Disk
conflict-this-file = This file
conflict-changed = { $name } was changed outside Quillwright (for example, by an agent) while you had unsaved edits.
conflict-deleted = { $name } was deleted or moved outside Quillwright.
conflict-unreadable = { $name } can no longer be read: { $error }
conflict-reload = Reload from Disk
    .hint = Discard your unsaved edits
conflict-keep = Keep My Version
    .hint = Keep editing; saving will overwrite the disk version
watch-degraded = File watching degraded: { $warning }

## Opening and saving

dialog-open-project = Open Project
dialog-choose-parent = Choose Parent Folder
cannot-read = Cannot read { $path }: { $error }
open-failed = Could not open { $path }: { $error }
save-failed = Save error: { $error }
project-create-failed = Could not create project { $path }: { $error }
project-open-failed = Could not open project { $path }: { $error }

## New Project

new-project-title = New Project
new-project-name = Project folder name
new-project-parent = Create inside
new-project-choose = Choose Folder...
new-project-kind = Kind of project
new-project-language = Language of the template
# $files lists the template's folders and files: "chapters/, characters/, AGENT.md".
new-project-creates = Creates { $files }.
new-project-note = Templates are optional. Existing folders will not be overwritten.

## Find and replace

find-title = Find / Replace
find-next = Find Next
find-previous = Find Prev
find-replace = Replace
find-replace-all = Replace All
find-found = Match found
find-none = No match found
find-no-previous = No previous match
find-empty = Enter text to find
find-select = Select a match or use Find
find-mismatch = Selection does not match search text
find-replaced = Replaced selection
find-nothing-to-replace = No matches to replace
find-replaced-all = { $count ->
    [one] Replaced { $count } occurrence
   *[other] Replaced { $count } occurrences
}
line-deleted = Line deleted

## Lint and format tools (Tools menu)

tool-output = Tool Output
tool-output-none = No output available.
tool-no-lint = No lint command configured. Add a [tools] lint entry to config.toml.
tool-no-format = No format command configured. Add a [tools] format entry to config.toml.
tool-empty = Configured tool command is empty.
tool-save-first = Save before running this tool on the current file, or disable *_use_open_file.
tool-not-a-file = Current file path '{ $path }' is not a file.
tool-no-file = No file is currently open. Save the document first or disable *_use_open_file.
tool-temp-failed = Failed to prepare temp file: { $error }
tool-run-failed = Failed to run '{ $tool }': { $error }
project-name-invalid = Enter a folder name, without path separators.
project-name-single = Use a single folder name.
project-parent-missing = Choose an existing parent folder.
button-copy = Copy

## Assistant panel

assistant-title = Assistant
assistant-done = Done
assistant-settings = Settings
assistant-intro = Quillwright runs an AI tool installed on your computer and shows its answer here. Your manuscript only changes when you accept a suggestion.
assistant-choose-tool = Choose a tool
assistant-command = Command
assistant-prompt-argument = Pass the prompt as the last argument (default: stdin)
assistant-command-note = The command runs in the project folder. Login and API keys stay with the tool.
preset-claude-note = Uses your Claude Code login. Install: https://claude.com/claude-code
preset-codex-note = Uses your Codex CLI login, in a read-only sandbox.
preset-ollama-note = Runs a local model; the manuscript never leaves your machine.
assistant-open-document = Open a { $document } to work with the assistant.
assistant-selected = { $count ->
    [one] { $words } word selected
   *[other] { $words } words selected
}
assistant-cursor = cursor on line { $line }
assistant-needs-selection = Select a passage first
assistant-ask = Ask
# $what lists what is sent, starting with assistant-this-document: "this chapter, world
# rules, 2 character sheets".
assistant-sends = Sends { $what } to `{ $tool }`.
assistant-this-document = this { $document }
assistant-sent-passages = { $count ->
    [one] { $count } reference passage
   *[other] { $count } reference passages
}
assistant-passages = Reference passages
assistant-clear = Clear
assistant-remove-passage = Stop sending this passage
assistant-writing = Writing…
assistant-details = Details
assistant-prompt-sent = Prompt sent
assistant-revision = Suggested revision
assistant-continuation = Suggested continuation
assistant-answer = Answer
assistant-replace = Replace selection
assistant-insert = Insert at cursor
assistant-retry = Try again
assistant-other-file = This answer is for another file; open it again to apply.
assistant-empty-answer = The assistant returned an empty answer.
assistant-select-first = Select a passage in the editor first.
assistant-start-failed = Could not start `{ $tool }`: { $error }.
assistant-not-found = Is it installed and on your PATH? Check the assistant settings.
assistant-passage-changed = The selected passage changed since the request. Copy the suggestion instead.
job-lost = Process lost
job-failed = Command failed ({ $status })

## Autosave and version history

autosave-failed = Autosave failed: { $error }
history-copy-failed = Could not keep a history copy: { $error }
history-title = Version History — { $title }
history-read-failed = Could not read this copy: { $error }
history-empty = No earlier versions of this file yet.
    .when = A copy is kept when a save replaces the file (at most every 10 minutes, and always before a large cut), and whenever an agent's change replaces the text in the editor.
history-compared = Compared with the text in the editor: struck-through text is only in the editor, highlighted text is only in this version.
history-restore = Restore This Version
    .hint = Replace the editor text. The current text is kept in the history first.
history-restored = Restored an earlier version
history-today = Today { $time }
history-yesterday = Yesterday { $time }
# Dates of older copies, as chrono formats (https://docs.rs/chrono/latest/chrono/format/strftime/):
# this year, then earlier years. Names of days and months are always in English, so other
# languages use numbers.
history-this-year = %a %-d %b %H:%M
history-older = %-d %b %Y %H:%M

## Spelling

spelling-none = No spelling suggestions
spelling-no-suggestions = No suggestions
spelling-add = Add to Project Dictionary
    .hint = Adds “{ $word }” to { $file } in the project folder
spelling-add-failed = Could not update { $file }: { $error }
spelling-ignore = Ignore for This Session
spelling-check = Check Spelling
    .hint = Underline misspelled words; right-click one for suggestions
spelling-language = Spelling Language
spelling-language-book = This book (saved in metadata.yaml)
spelling-language-default = Default for books without a language
spelling-loading = Loading { $language }…
spelling-checking = Checking { $language }
spelling-folder = Open Dictionaries Folder
    .hint = Add Hunspell .aff and .dic files here for more languages:
        { $path }
spelling-no-dictionary = No dictionary for { $language }
spelling-load-failed = Could not load the { $language } dictionary
write-failed = Could not write { $path }

## Project search (sidebar)

search-hint = Search project
search-searching = Searching…
search-none = No results
search-results = { $count ->
    [one] 1 result
   *[other] { $files ->
        [one] { $count } results in 1 file
       *[other] { $count } results in { $files } files
    }
}
search-reading = reading { $title } for the first time…
search-more = searching…
search-failed = { $count ->
    [one] { $count } file could not be read
   *[other] { $count } files could not be read
}
search-and-more = …and { $count } more
search-note = Note
# A result's page in a PDF.
search-page = p. { $page }

## Reference panel (research files)

dialog-add-research = Add Research Files
reference-unreadable = The file could not be read.
reference-open-failed = This file could not be opened: { $error }
reference-missing = This file was moved or deleted.
reference-add-failed = Could not add research files: { $error }
reference-read-only = read only
reference-edit = Edit
    .hint = Open this file in the editor
reference-actual-size = Actual Size
reference-fit-width = Fit Width
reference-system-viewer = Open with System Viewer
reference-use = Use as Context
    .hint = Send the selected passage, with its source, to the assistant
reference-click-actual = Click for actual size
reference-click-fit = Click to fit the panel
# Where a passage of a text file is, in its citation for the assistant.
reference-line = line { $line }
reference-lines = lines { $first }–{ $last }
reference-unsupported = Quillwright can't show this kind of file.
reference-cut = the rest of this file is not shown
# Research file types in the sidebar.
file-text = Text
file-notes = Notes
file-image = Image
file-pdf = PDF
file-other = File

## PDF viewer

pdf-unreadable = The PDF could not be read.
pdf-no-pages = This PDF has no pages.
pdf-password = This PDF is protected with a password, which Quillwright can't open yet.
pdf-invalid = This file doesn't look like a PDF Quillwright can read.
pdf-page-failed = This page could not be shown.
pdf-page-number = Page { $page }
# Where a passage of a PDF is, in its citation for the assistant (one page is search-page).
pdf-pages = pp. { $first }–{ $last }
pdf-no-text = This PDF has no text to search.
pdf-reading = reading { $percent }%
pdf-no-matches = No matches
pdf-match-of = { $current } of { $total }
pdf-matches = { $count ->
    [one] { $count } match
   *[other] { $count } matches
}
# Before the page number box, and after it: "Page [3] of 120".
pdf-page = Page
    .hint = Type a page number, or drag
pdf-page-count = of { $count }
pdf-zoom-in = Zoom in
pdf-zoom-fit = Fit the page to the panel
pdf-zoom-out = Zoom out
pdf-find = Find in this PDF
pdf-previous = Previous match
pdf-next = Next match

## Export Manuscript

export-title = Export Manuscript
export-book-title = Title
export-author = Author
export-language = Language
    .hint = Used for hyphenation and by e-readers. Empty: the lang in metadata.yaml, or English.
export-page-size = Page size
export-toc = Table of contents
# $documents is the kind's count ("12 chapters"), $words the word count ("80,000 words").
export-summary = { $documents } · { $words }, in sidebar order
export-uses = Uses { $file }
export-styles = Styles from { $file }
export-running = Exporting…
export-start = Export…
export-nothing = There is nothing to export yet.
export-done = Exported { $file }
export-open = Open
export-show = Show in Folder
export-save-failed = Could not save { $path }: { $error }
export-stopped = The export stopped unexpectedly.
export-metadata-ignored = { $file } was ignored: { $error }
export-write-failed = Could not write the file: { $error }
export-no-styles = { $file } has no styles and was ignored.
export-image-skipped = Image not included: { $file }
format-docx = Word (.docx)
    .hint = For editors, agents, and Google Docs
format-epub = EPUB e-book (.epub)
    .hint = For e-readers and beta readers
format-pdf = PDF (.pdf)
    .hint = For printing and sharing a fixed layout
format-odt = OpenDocument (.odt)
    .hint = For LibreOffice
format-html = Web page (.html)
    .hint = A single self-contained page
page-a5 = A5 (148 × 210 mm)
page-a4 = A4 (210 × 297 mm)
page-letter = US Letter (8.5 × 11 in)
page-trade = Trade paperback (6 × 9 in)

## Themes (View → Theme)

theme-paper = Paper
theme-sepia = Sepia
theme-night = Night
reference-loading = Loading…
