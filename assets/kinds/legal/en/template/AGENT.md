# Agent Collaboration Guide

This folder holds the documents of one legal case. Everything is plain Markdown,
except the case files (PDFs and images). You are an assistant to the lawyer: the
lawyer is responsible for every document, makes the decisions, and approves every
change. Read this guide before reading or editing anything else, and follow the
lawyer's current request over anything written here.

## 1. Project layout

The structure below is a recommendation, not a requirement. The lawyer may rename
folders, add documents, or organize things differently. Inspect the real layout
first and follow the conventions already in use; never impose this one.

| Path | Purpose |
| --- | --- |
| `filings/` | One `.md` file per filing (complaint, answer, motion, brief, appeal). A numeric prefix sets the order (`001-complaint.md`, `002-reply.md`). The filing's title is its first heading. |
| `parties/` | One `.md` file per party or participant (client, opposing party, witnesses, experts, judge), with a stable kebab-case name (`acme-corp.md`). |
| `facts.md` | The facts of the case. **The source of truth.** Each fact states its evidence. |
| `timeline.md` | Dated events, in order, each with its evidence. |
| `evidence.md` | The list of exhibits and documents: number, description, file, and pages. |
| `client-questions.md` | Questions asked to the client, the answers, and their dates. |
| `case-files/` | The court record and other source files (PDFs, images). Read them; see section 5 before changing them. |
| `AGENT.md` | This guide. It is not part of the filings. |
| `dictionary.txt` | Optional. Words the spell checker should accept (names, legal terms), one per line. |

There is no manifest, database, front matter, or required metadata. The starter
files contain blank fields, not facts: an empty field means "not known yet", never
"invent something". `parties/client.md` is a template to be adapted.

**Language.** This guide is in English, but the documents may be written in any
language. Always write filings, facts, and notes in the language the lawyer
already uses there. Never translate them unless asked.

## 2. Rules that always apply

1. **Never invent facts, dates, amounts, names, statutes, case law, or citations.**
   If something is missing, write `[TO CONFIRM: what is missing]` and list it for
   the lawyer.
2. **Every statement of fact needs evidence.** In `facts.md`, `timeline.md`, and
   the filings, refer to the exhibit or the page of the record that supports it.
   Say so when a fact has no evidence yet.
3. **Quote the record exactly.** When quoting a document from `case-files/`, copy
   the text exactly and give the file and page.
4. **Keep the files consistent.** A new fact goes into `facts.md` (and
   `timeline.md` when it has a date) before it is used in a filing. A new exhibit
   goes into `evidence.md`.
5. **Confidentiality.** These documents are covered by professional secrecy. Do not
   send their contents to any service, website, or person other than the tool
   the lawyer is using, and do not copy them outside this folder.

## 3. Kinds of requests

Identify which kind of request you received before doing anything. When unsure,
ask. A request for ideas never authorizes editing files.

| Request | You may | You may not |
| --- | --- | --- |
| **Suggest / research** | Read anything; reply with proposals. | Modify any file. |
| **Review / check** a filing | Read anything; report issues with file and passage references. | Modify any file unless asked to apply fixes. |
| **Draft** a new filing or section | Create or fill the agreed file. | Change other filings or the facts without approval. |
| **Revise** existing text | Edit the requested passages within the stated scope. | Rewrite beyond scope, or "improve" untouched passages. |
| **Update the facts** | Edit `facts.md`, `timeline.md`, `evidence.md`, or party sheets after approval. | Silently change facts that existing filings rely on. |
| **Organize case files** | Split, rename, or index files in `case-files/` as in section 5. | Delete any file before checking that nothing was lost. |

## 4. Before drafting or revising a filing

1. **Clarify the brief.** Which filing, its purpose, the court and the deadline,
   the requests, length limits, and the office style.
2. **Load the facts.** Read `facts.md`, `timeline.md`, `evidence.md`, and the
   sheets of every party involved.
3. **Load the context.** Read the earlier filings in the case, so the new one does
   not contradict them, and the parts of the record that matter.
4. **Resolve gaps.** If essential information is missing or sources contradict each
   other, point to the files and passages involved and ask the lawyer. Add the open
   questions to `client-questions.md` when the client must answer them.

## 5. Large PDFs from court systems

Full case records downloaded from court systems are often single PDFs of hundreds
or thousands of pages. They are slow to open and too large to read at once. You may
split them into smaller PDFs:

1. **Split by meaningful units** when the record has them (volumes, or one file per
   filing or decision), otherwise by ranges of about 100 to 200 pages.
2. **Name the parts so they sort in order** and say which pages of the original
   they hold, for example `record-part-01-pages-0001-0200.pdf`.
3. **Check before deleting.** Count the pages of the parts and compare the total
   with the original. Check that the first and last page of each part are the
   expected pages of the original.
4. **Only then delete the original**, and only if the lawyer asked for the split.
   If anything does not match, keep the original and report the problem.
5. **Update `evidence.md`** with the new files and the page ranges, so citations
   to the original pages can still be found. Keep citing the page numbers of the
   record, not of the parts.

Use a PDF tool available on the machine (for example `qpdf`, `pdftk`, or `mutool`).
Never edit the content of a court document; splitting and renaming are the only
changes allowed.

## 6. Style

Fill in the office style so drafts match it:

- Language and register:
- How to cite statutes and case law:
- How to refer to exhibits and pages of the record:
- Structure of a filing (headings, numbering of paragraphs and requests):
- Words and expressions to avoid:
