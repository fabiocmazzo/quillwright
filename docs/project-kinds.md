# Project kinds: one core, several kinds of writing

Status: idea, not started. Written in October 2026 for later work.

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
| Main documents | `chapters/` ("Chapters", "New chapter") | `pieces/` ("Documents", "New document") |
| Entity sheets | `characters/` ("Characters") | `parties/` ("Parties": client, opposing party, witnesses, judge) |
| Guide for agents | `AGENT.md` (voice, style, rules for agents) | `AGENT.md` (office style, citation format, tone, "never state a fact without its evidence") |
| Source of truth | `world-rules.md` | `facts.md` (facts, each with its evidence) and `timeline.md` |
| Extra files | — | `client-questions.md` (asked and answered), `evidence.md` (index of exhibits) |
| Assistant actions | Rewrite, Continue the scene, Three alternatives, Check continuity, Ask | Rewrite, Draft from facts, Check against facts, Find contradictions with other documents, Questions for the client, Ask |
| Unit of export | The whole book, all chapters in order | **One document at a time** (a petition is filed alone), with an option to join several |
| Export defaults | A5, title page, each chapter on a new page, scene breaks | A4, no title page, no page breaks between sections, office `reference.docx` with letterhead |
| Typography | Indented paragraphs, no space between them | Paragraph spacing, justified text, numbered headings |
| Word counts | Session and manuscript goals | Less relevant; page count matters more (court page limits) |

The table is a starting point. The legal column should be checked with a practising lawyer
before any code is written.

## The legal case kind in more detail

A possible folder layout:

```text
smith-v-acme/
├── AGENT.md               # office style and rules for agents
├── facts.md               # the facts of the case, each with its evidence
├── timeline.md            # dated events
├── client-questions.md    # questions asked, answers received, date
├── evidence.md            # exhibit list: number, description, file
├── parties/
│   ├── client.md
│   └── opposing-party.md
├── pieces/
│   ├── 001-initial-petition.md
│   ├── 002-reply.md
│   └── 003-closing-arguments.md
├── exhibits/              # PDFs and images, referenced from evidence.md
└── reference.docx         # the office template (letterhead, fonts)
```

What the assistant would add:

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

## How Quillwright knows the kind of a project

The ground rules forbid a manifest or hidden metadata in the project, so the kind must come
from the visible files or from app data outside the project. Options:

1. **Infer from the folders.** `pieces/` or `facts.md` means a legal case; `chapters/` or
   `world-rules.md` means a novel. This needs no extra file and survives copying the folder.
   Ambiguous folders fall back to Novel.
2. **Remember it in the app config,** keyed by project path, like export titles today. This
   keeps the project clean, but the kind is lost when the folder moves or is opened on
   another machine.
3. **A visible line in `AGENT.md`,** for example `Project kind: legal case`. It is plain
   text that people and agents can read, but it mixes app behavior with agent instructions.

Recommendation: use option 1, with option 2 as an override when the inference is wrong
(**Project → Project Kind...**). Revisit option 3 if inference turns out to be fragile.

## Implementation sketch

- A new module `src/kinds.rs` with a data-driven `ProjectKind`. Built-in kinds are constants,
  not user-defined plugins, at least at first. It would hold:
  - the folder roles (main documents, entity sheets, extra context files), with their labels,
    the label of their "New" button, and the template for a new file;
  - the context files the assistant reads, with their size limits;
  - the assistant actions and their prompt texts;
  - the export defaults (unit of export, page size, title page, page breaks between
    documents, typography);
  - the files for New Project, embedded with `include_str!` from
    `assets/project-templates/<kind>/`.
- `manuscript::Index`: replace the fixed chapters, characters, and notes with sections taken
  from the kind. `CHAPTERS_DIR` and `CHARACTERS_DIR` become fields of the kind.
- `project.rs`: New Project offers a choice of kind and copies that kind's template.
- `workspace.rs`: sidebar sections, buttons, and menu labels come from the kind.
- `assistant.rs`: `StoryContext::gather` reads the kind's context files instead of
  hard-coded `AGENT.md` and `world-rules.md`, and `Action` becomes kind-specific (or each
  kind chooses from a shared list).
- `export/`: `Options` gains a unit (whole project or current document) and typography
  settings; the writers already share one model, so only styles and page breaks change.

Keep the Novel kind behaving exactly as today. The existing tests are the safety net, and
each new kind should come with tests for its index and prompts.

## Suggested phases

1. **Prototype without code:** write a legal case template by hand (folders, `AGENT.md`,
   `facts.md`) and let a lawyer use the current app for one or two weeks. Note which labels
   and actions they miss.
2. **Import from Word** (useful for every kind).
3. **`ProjectKind` refactor** with Novel as the only kind; no visible change.
4. **Legal case kind:** template, labels, context files, assistant actions, per-document
   export.
5. **Confidentiality controls** for the assistant.
6. Other kinds, if there is demand.

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

- Should the legal kind use its own vocabulary or neutral words ("Documents", "People",
  "Facts") that also suit other kinds?
- Where do exhibits live, and how are they cited from documents (for example
  `[Exhibit 3](exhibits/03-contract.pdf)`)?
- Should a case share an office-wide `AGENT.md` and `reference.docx` with other cases, for
  example in a parent folder? This affects how context files are found.
- Is a page count (rather than a word count) needed in the status bar for court page limits?
