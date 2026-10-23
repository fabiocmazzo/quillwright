# Project kinds: one core, several kinds of writing

Status (October 2026): the Novel kind is described by a `kind.toml` (with no visible change;
its prompts were checked to be identical to the ones built before), and a Legal case kind
exists in English and Portuguese (Brazil), chosen in New Project. Export per kind, user
kinds, and a lawyer's feedback are next.

## Why

Quillwright was built for novels, but most of what makes it useful is not specific to
fiction. A lawyer has the same problems as a novelist:

- many documents that have to agree with each other;
- facts that must stay consistent from one document to the next;
- questions that keep coming back;
- old drafts that are hard to find.

Petitions, replies, and appeals in one case are like the chapters of a book. The facts of the
case are like the world rules. The parties are like the characters.

The idea is **not** to build legal software. Quillwright would stay a writing environment, and
a *project kind* would adapt it to one kind of writing. The first kinds would be "Novel"
(today's behavior) and "Legal case". Others could follow later.

## Principle

A kind is a set of **conventions and defaults**, not a fork of the code. Everything stays
plain Markdown in a plain folder, and the ground rules in `AGENTS.md` apply unchanged:

- no lock-in;
- never lose the author's text;
- AI output enters a document only after the author accepts it;
- no API keys in the app;
- app data stays out of the project.

A project of one kind can be opened, edited, and versioned as an ordinary folder without
Quillwright.

## What is already generic

These parts already work for any kind of writing:

- the editor, autosave, version history, and safe reloading of external changes;
- the file watcher and the ability to work next to an AI agent that edits the same folder;
- the assistant runner (`assistant::Job`), presets, and the diff review of suggestions;
- the document model and writers in `src/export/`;
- find and replace, focus mode, themes, and word counts.

## What a kind defines

| Aspect | Novel (today) | Legal case |
| --- | --- | --- |
| Main documents | `chapters/` ("Chapters", "New chapter") | `filings/` or `pecas/` ("Filings", "New filing") |
| Entity sheets | `characters/` ("Characters") | `parties/` or `partes/` ("Parties": client, opposing party, witnesses, judge) |
| Research | `research/` ("Research") | `case-files/` or `autos/` ("Case Files": the court record, exhibits) |
| Guide for agents | `AGENT.md` (voice, style, rules for agents) | `AGENT.md` (office style, citation format, tone, "never state a fact without its evidence") |
| Source of truth | `world-rules.md` | `facts.md` (facts, each with its evidence) and `timeline.md` |
| Extra files | — | `client-questions.md` (asked and answered), `evidence.md` (index of exhibits) |
| Assistant actions | Rewrite, Continue the scene, Three alternatives, Check continuity, Ask | Rewrite, Draft from facts, Check against facts, Find contradictions with other documents, Questions for the client, Ask |
| Unit of export | The whole book, all chapters in order | **One document at a time** (a petition is filed alone), with an option to join several |
| Export defaults | A5, title page, each chapter on a new page, scene breaks | A4, no title page, no page breaks between sections, office `reference.docx` with letterhead |
| Typography | Indented paragraphs, no space between them | Paragraph spacing, justified text, numbered headings |
| Word counts | Session and manuscript goals | Less relevant; page count matters more (court page limits) |

The legal column is implemented except export, which is still the novel's. It should be
checked with a practising lawyer.

## The legal case kind in more detail

The English template (the Portuguese one uses `pecas/`, `partes/`, `autos/`, `fatos.md`,
`cronologia.md`, `provas.md`, and `perguntas-ao-cliente.md`):

```text
smith-v-acme/
├── AGENT.md               # office style and rules for agents
├── facts.md               # the facts of the case, each with its evidence
├── timeline.md            # dated events
├── client-questions.md    # questions asked, answers received, date
├── evidence.md            # exhibit list: number, description, file, pages
├── parties/
│   └── client.md
├── filings/
│   └── 001-complaint.md
├── case-files/            # the court record and exhibits (created when files are added)
└── reference.docx         # optional: the office template (letterhead, fonts)
```

The `AGENT.md` sets the rules for agents: never invent facts, statutes, case law, or
citations; every fact needs its evidence; quote the record exactly; keep the files
consistent; confidentiality. It also covers **large PDFs from court systems**, which are
often single files of thousands of pages: agents may split them into parts named by page
range, must check that the parts add up to every page of the original (page counts, and the
first and last page of each part), may delete the original only after that check and only
when the lawyer asked for the split, and record the ranges in the evidence list so that
citations to the original pages stay valid.

What the assistant adds (all but the second exist):

- **Check against facts:** compares the open document with `facts.md` and `timeline.md` and
  lists statements that are missing, contradicted, or have no evidence. This is the legal
  version of *Check continuity*.
- **Find contradictions with other documents:** the same check across earlier pieces in the
  case, for example a reply that contradicts the initial petition.
- **Draft from facts:** drafts a section, such as the statement of facts, from `facts.md`.
  It is shown as an insertion to accept or reject, never written directly.
- **Questions for the client:** lists what is still unknown, without repeating questions
  already answered in `client-questions.md`.

## Prerequisites that matter for lawyers

1. **Import from Word.** A lawyer's past work is in `.docx` files, and without import the
   kind is hard to adopt. A pure-Rust option is `docx_rs::read_docx`, which converts
   paragraphs, headings, emphasis, lists, footnotes, and tables to Markdown. Novelists would
   benefit too, because many drafts start in Word.
2. **Faithful Word output.** Exports go to courts and clients. `reference.docx` already
   replaces the styles; letterhead, headers, and footers from the reference document should
   also be kept.
3. **Confidentiality.** Client data sent to a cloud AI tool may raise professional secrecy
   and data protection issues (for example LGPD in Brazil or GDPR in Europe). The kind should:
   - make the local preset (Ollama) the suggested default;
   - show which files go into each prompt (the assistant panel already lists the context
     sources);
   - offer a per-project switch that turns off cloud presets.

   Quillwright stores no keys and sends nothing on its own, and that should be said clearly
   in the UI.
4. **PDF for filing.** Courts often require PDF (in Brazil, PJe). The built-in typst export
   already covers this; A4 and office styles are needed. PDF/A could be useful later.

## Kinds are folders, like themes

Each kind is a folder with a shared `kind.toml` and one folder per language. Each language
folder has its own `kind.toml`, merged over the shared one, and a `template/` folder that
New Project copies. Adding a kind or a language is content work (Markdown and TOML), not
code: `build.rs` lists every file under `assets/kinds/` and the app embeds them.

```text
assets/kinds/                    # built-in kinds, embedded in the executable
├── novel/
│   ├── kind.toml                # shared: assistant prompts and actions
│   ├── locales/                 # interface texts: en.ftl, pt-BR.ftl, es.ftl, zh-CN.ftl
│   └── en/
│       ├── kind.toml            # language_name = "English"
│       └── template/            # copied as-is by New Project
│           ├── AGENT.md
│           ├── world-rules.md
│           ├── chapters/001-opening.md
│           └── characters/example-character.md
└── legal/
    ├── kind.toml
    ├── en/
    │   ├── kind.toml            # folders filings/, parties/, case-files/; facts.md, ...
    │   └── template/
    └── pt-BR/
        ├── kind.toml            # folders pecas/, partes/, autos/; fatos.md, ...
        └── template/            # includes metadata.yaml with lang: pt-BR
```

### Languages

A language folder holds what changes with the language of the project: its
`language_name` (shown in New Project), the names of the folders and files it creates
(`detect`, `first_file`, `[documents]`, `[sheets]`, `[research]` folders, the
`[[assistant.context]]` files), and its template. Merging works table by table; any other
value, including a list, is replaced as a whole.

When a kind has more than one language, New Project asks which one; otherwise it does not.
The `en` variant is listed first and is required for every built-in kind.

The labels the app shows ("Filings", "New filing", "Check against facts") follow the
*interface* language, not the template's: they are Fluent messages in the kind's
`locales/<lang>.ftl`, named after the kind's id (`legal-documents`, `legal-action-check`;
see "Interface language" in `AGENTS.md`). What stays in English in every language is the
assistant's prompts. The prompts tell the model to write in the language of the documents, which
works for any language, and keep one set of prompts per kind to maintain. A language may
still override any value, so translated prompts are possible if they turn out to work
better.

Later, kinds can also be read from the user data folder
(`<data dir>/quillwright/kinds/<id>/`, the same place as extra dictionaries), so an office
can keep its own kind with its own `AGENT.md` and `reference.docx` without rebuilding the
app. A user kind with the same id as a built-in one replaces it.

The kind folder belongs to the app, never to the writing project: the project stays a plain
folder of Markdown files with no manifest.

### `kind.toml`

The TOML replaces what used to be hard-coded (folder names, the assistant's context files,
actions, and prompts); the kind's name, description, and labels are in `locales/`. Every
table uses `#[serde(default)]`, so a kind written for an older version keeps loading.

```toml
# A project with any of these (relative paths; a trailing / means a folder) is this kind.
detect = ["chapters/", "characters/", "world-rules.md"]
# File opened after New Project creates the folder.
first_file = "chapters/001-opening.md"

[documents]                      # the main documents: counted, searched, exported in order
folder = "chapters"
singular = "chapter"             # how the (English) prompt names one
new_name = "chapter-{n}"         # new files are NNN-chapter-N.md, after the highest number
new_title = "Chapter {n}"

[sheets]                         # entity sheets: characters, parties, ...
folder = "characters"
singular = "character"
new_name = "new-character"       # new-character.md, new-character-2.md, ...
new_title = "New character"
template = "characters/example-character.md"   # body reused for new sheets

[assistant]
role = "helping an author with a work of fiction"
guide = "AGENT.md"               # sent with every request, with its own instructions

[[assistant.context]]            # other files sent with every request
file = "world-rules.md"
heading = "World rules"
limit = 24000

[[assistant.actions]]
id = "continue"                  # labeled by the message novel-action-continue
result = "insert"                # replace (needs a selection) | insert | notes
task = "Continue the story from the cursor position..."
output = "Reply with only the new text to insert at the cursor..."
```

**Ask** (the free-form request) stays built in for every kind.

### How Quillwright knows the kind of a project

The ground rules forbid a manifest or hidden metadata in the project, so the kind is
inferred from the visible files: the first kind whose `detect` list matches wins, and a
folder that matches nothing is treated as a Novel (today's behavior). This needs no extra
file and survives copying the folder. If inference turns out to be wrong in practice, an
override remembered in the app config by project path (**Project → Project Kind...**) can be
added later.

### Code

- `src/kinds.rs`: `Kind` (one per kind and language, deserialized from the merged TOML),
  its template files, `all()`, `choices()` and `languages(id)` for New Project, `find`, and
  `detect(root)`. `build.rs` writes the list of embedded files (`$OUT_DIR/kind_files.rs`).
- `manuscript::Index::build(root, kind)` reads the kind's documents and sheets folders
  instead of the constants `CHAPTERS_DIR` and `CHARACTERS_DIR`; `new_document` and
  `new_sheet` take their names and titles from the kind.
- `project::create(path, kind)` copies the kind's template.
- `workspace.rs`: sidebar sections, the File menu, and counts use the kind's labels, read
  from its `locales/` in the interface language.
- `assistant.rs`: `Context::gather` reads the kind's guide and context files, and the
  writing actions (id, task, output format, result kind) come from the kind.
- Export defaults (unit of export, page size, typography) stay in code until the Legal kind
  needs them. Also still novel-specific: export looks for images in `chapters/`, the
  keyboard shortcut list says "New chapter", and the home screen tagline speaks of stories.
- The test `builtin_kinds_load_and_are_complete` checks every kind and language: required
  fields, that the template has every file the TOML names, and that a new project of the
  variant is detected as that variant.

Keep the Novel kind behaving exactly as today. The existing tests are the safety net, and
the prompts built from `kind.toml` must be identical to the ones built before the change.

## Suggested phases

1. **The Novel kind as data.** Move the template to `assets/kinds/novel/`, describe it in
   `kind.toml`, and read folders, labels, context files, actions, and prompts from it. No
   visible change. **Done.**
2. **Legal kind, languages, and the kind choice in New Project.** English and Portuguese
   (Brazil) templates; Filings, Parties, Case Files; the actions Draft from facts, Check
   against facts, and Questions for the client. **Done.**
3. **A lawyer's feedback:** use it on a real case for one or two weeks and note which
   labels, files, and actions are missing or wrong.
4. **Export per kind:** one filing at a time (or several joined), A4, no title page, no
   page breaks between sections, justified paragraphs with spacing; images from the kind's
   documents folder.
5. **User kinds** from the data folder.
6. **Import from Word** (useful for every kind).
7. **Confidentiality controls** for the assistant.
8. "Find contradictions with other filings": an action that sends the earlier filings
   too, which needs a way to choose them and a larger context budget.
9. Other kinds, if there is demand.

## Other kinds to consider later

- **Non-fiction / thesis:** chapters, a sources file, citations, numbered headings. A
  bibliography would need a pure-Rust citation formatter.
- **Screenplay:** scenes and characters fit, but screenplay formatting (Fountain) is a large
  feature of its own.
- **Technical documentation:** probably out of scope; many tools already exist.

## Non-goals

- No case management, court deadlines, filing integration, case-law databases, or billing.
- No legal advice from the app; the assistant only checks consistency and helps with
  drafting.
- No database or proprietary format, for any kind.

## Open questions

- How should filings link to exhibits (for example
  `[Exhibit 3](case-files/03-contract.pdf)`), and should a click open the PDF at the page?
- Should a case share an office-wide `AGENT.md` and `reference.docx` with other cases, for
  example in a parent folder? This affects how context files are found.
- Is a page count (rather than a word count) needed in the status bar for court page limits?
