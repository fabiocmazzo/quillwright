//! AI writing assistance through an external command (Claude Code, Codex, Ollama, ...).
//! Quillwright builds a prompt with the relevant story context, runs the configured
//! command in the background, and shows the answer as a suggestion. Nothing is written
//! to the manuscript until the author accepts it.
pub use crate::kinds::ResultKind;
use crate::kinds::{self, Kind};
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AssistantConfig {
    /// Command and arguments. The prompt is written to stdin unless
    /// `prompt_as_argument` is true, in which case it is appended as the last argument.
    pub command: Option<Vec<String>>,
    pub prompt_as_argument: bool,
}

pub struct Preset {
    pub name: &'static str,
    pub command: &'static [&'static str],
    pub prompt_as_argument: bool,
    pub note: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Claude Code",
        command: &["claude", "-p"],
        prompt_as_argument: false,
        note: "Uses your Claude Code login. Install: https://claude.com/claude-code",
    },
    Preset {
        name: "Codex",
        command: &[
            "codex",
            "exec",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "-",
        ],
        prompt_as_argument: false,
        note: "Uses your Codex CLI login, in a read-only sandbox.",
    },
    Preset {
        name: "Ollama (local)",
        command: &["ollama", "run", "llama3.1"],
        prompt_as_argument: false,
        note: "Runs a local model; the manuscript never leaves your machine.",
    },
];

/// A request: one of the kind's writing actions (`kinds::Action`), or a free-form Ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Kind(kinds::Action),
    Ask,
}

impl Action {
    pub fn needs_selection(&self) -> bool {
        match self {
            Action::Kind(action) => action.needs_selection,
            Action::Ask => false,
        }
    }

    /// How the answer is meant to be used.
    pub fn result_kind(&self) -> ResultKind {
        match self {
            Action::Kind(action) => action.result,
            Action::Ask => ResultKind::Notes,
        }
    }
}

/// A passage from a research file that the author chose to send with requests.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    /// Where it comes from, such as `research/harbor.pdf, p. 42`.
    pub citation: String,
    pub text: String,
}

/// Longest clip sent, in characters.
pub const CLIP_LIMIT: usize = 6_000;
/// Most clips kept at once.
pub const MAX_CLIPS: usize = 8;

/// A context file the kind sends with every request (a novel's world rules).
#[derive(Clone, Debug, Default)]
pub struct ContextFile {
    pub file: String,
    pub heading: String,
    pub summary: String,
    pub text: String,
}

/// Story context gathered from the project folder.
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// The guide for agents (AGENT.md): (file name, contents).
    pub guide: Option<(String, String)>,
    pub files: Vec<ContextFile>,
    /// Sheets named in the document (characters): (relative path, contents).
    pub sheets: Vec<(String, String)>,
    /// How sheets are called, singular and plural ("character sheet").
    pub sheet_words: (String, String),
    /// Reference passages the author added with "Use as Context".
    pub clips: Vec<Clip>,
}

impl Context {
    /// Read the kind's guide (AGENT.md) and context files (world-rules.md), and the sheets
    /// whose title is named in `text`. `sheets` are (path, title).
    pub fn gather(root: &Path, kind: &Kind, text: &str, sheets: &[(PathBuf, String)]) -> Self {
        let read = |name: &str| {
            std::fs::read_to_string(root.join(name))
                .ok()
                .filter(|s| !s.trim().is_empty())
        };
        let mut found = Vec::new();
        for (path, title) in sheets {
            let mentioned = title
                .split_whitespace()
                .filter(|word| word.chars().count() >= 3)
                .any(|word| text.contains(word));
            if mentioned && found.len() < 8 {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    let relative = path.strip_prefix(root).unwrap_or(path);
                    // The same on every system: `characters/mara.md`, never with backslashes.
                    let relative: Vec<_> = relative
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect();
                    found.push((relative.join("/"), contents));
                }
            }
        }
        let spec = &kind.assistant;
        Context {
            guide: Some(&spec.guide)
                .filter(|name| !name.is_empty())
                .and_then(|name| read(name).map(|s| (name.clone(), truncate(&s, 16_000)))),
            files: spec
                .context
                .iter()
                .filter_map(|file| {
                    read(&file.file).map(|text| ContextFile {
                        file: file.file.clone(),
                        heading: file.heading.clone(),
                        summary: file.summary.clone(),
                        text: truncate(&text, file.limit.max(1)),
                    })
                })
                .collect(),
            sheets: found
                .into_iter()
                .map(|(path, text)| (path, truncate(&text, 8_000)))
                .collect(),
            sheet_words: (
                kind.sheets.prompt_singular.clone(),
                kind.sheets.prompt_plural.clone(),
            ),
            clips: Vec::new(),
        }
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        for file in &self.files {
            parts.push(file.summary.clone());
        }
        let (singular, plural) = &self.sheet_words;
        match self.sheets.len() {
            0 => {}
            1 => parts.push(format!("1 {singular}")),
            n => parts.push(format!("{n} {plural}")),
        }
        if let Some((name, _)) = &self.guide {
            parts.push(name.clone());
        }
        match self.clips.len() {
            0 => {}
            1 => parts.push("1 reference passage".into()),
            n => parts.push(format!("{n} reference passages")),
        }
        parts.join(", ")
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max_chars).collect();
        format!("{cut}\n[… truncated …]")
    }
}

/// What the author asked for, captured when the request was sent.
#[derive(Clone, Debug)]
pub struct Request {
    pub action: Action,
    pub instruction: String,
    pub file: Option<PathBuf>,
    pub chapter_title: String,
    /// Selection as character indices into the buffer at request time.
    pub start: usize,
    pub end: usize,
    pub selected: String,
}

const CURSOR: &str = "⟦CURSOR⟧";
const SEL_START: &str = "⟦SELECTION START⟧";
const SEL_END: &str = "⟦SELECTION END⟧";

/// The chapter with the selection or cursor marked, windowed if it is very long.
fn marked_chapter(text: &str, start: usize, end: usize) -> String {
    let byte = |i: usize| text.char_indices().nth(i).map_or(text.len(), |(b, _)| b);
    let (a, b) = (byte(start), byte(end));
    let marked = if a == b {
        format!("{}{CURSOR}{}", &text[..a], &text[a..])
    } else {
        format!(
            "{}{SEL_START}{}{SEL_END}{}",
            &text[..a],
            &text[a..b],
            &text[b..]
        )
    };
    const LIMIT: usize = 60_000;
    if marked.chars().count() <= LIMIT {
        return marked;
    }
    // Keep most of the text before the marker (what has happened) and some after.
    let marker = marked.find('⟦').unwrap_or(0);
    let marker_char = marked[..marker].chars().count();
    let from = marker_char.saturating_sub(45_000);
    let to = (marker_char + 15_000).min(marked.chars().count());
    let window: String = marked.chars().skip(from).take(to - from).collect();
    format!("[… earlier text omitted …]\n{window}\n[… later text omitted …]")
}

pub fn build_prompt(request: &Request, chapter: &str, context: &Context, kind: &Kind) -> String {
    let has_selection = request.start != request.end;
    let instruction = request.instruction.trim();
    let document = &kind.documents.singular;
    let (task, output) = match &request.action {
        Action::Kind(action) => (action.task.clone(), action.output.as_str()),
        Action::Ask => (
            if has_selection {
                format!(
                    "Answer the author's request about the selected passage (between \
                     ⟦SELECTION START⟧ and ⟦SELECTION END⟧) and the {document}."
                )
            } else {
                format!(
                    "Answer the author's request about the {document}. The cursor position \
                     is marked ⟦CURSOR⟧."
                )
            },
            "If the request asks for new or replacement manuscript text, reply with only that \
             text, ready to paste. Otherwise reply concisely.",
        ),
    };
    let spec = &kind.assistant;

    let mut p = String::new();
    p.push_str(spec.preamble.trim());
    p.push_str("\n\n");
    p.push_str("# Task\n\n");
    p.push_str(&task);
    p.push_str("\n\n");
    if !instruction.is_empty() {
        p.push_str("# Author's instruction\n\n");
        p.push_str(instruction);
        p.push_str("\n\n");
    }
    p.push_str("# Output format\n\n");
    p.push_str(output);
    p.push_str("\n\n");

    for file in &context.files {
        p.push_str(&format!("# {} ({})\n\n", file.heading, file.file));
        p.push_str(file.text.trim());
        p.push_str("\n\n");
    }
    if !context.sheets.is_empty() {
        p.push_str(&format!(
            "# {}\n\n",
            kinds::capitalize(&context.sheet_words.1)
        ));
        for (path, text) in &context.sheets {
            p.push_str(&format!("## {path}\n\n{}\n\n", text.trim()));
        }
    }
    if let Some((name, guide)) = &context.guide {
        p.push_str(&format!("# Project collaboration guide ({name})\n\n"));
        if !spec.guide_note.trim().is_empty() {
            p.push_str(spec.guide_note.trim());
            p.push_str("\n\n");
        }
        p.push_str(guide.trim());
        p.push_str("\n\n");
    }
    if !context.clips.is_empty() {
        p.push_str("# Reference material chosen by the author\n\n");
        if !spec.reference_note.trim().is_empty() {
            p.push_str(spec.reference_note.trim());
            p.push_str("\n\n");
        }
        for clip in &context.clips {
            p.push_str(&format!(
                "## {}\n\n<reference>\n{}\n</reference>\n\n",
                clip.citation,
                truncate(clip.text.trim(), CLIP_LIMIT)
            ));
        }
    }

    let location = request
        .file
        .as_ref()
        .map(|path| format!(" ({})", path.display()))
        .unwrap_or_default();
    let tag = document.replace(' ', "-");
    p.push_str(&format!(
        "# Current {document}: \"{}\"{location}\n\n<{tag}>\n{}\n</{tag}>\n",
        request.chapter_title,
        marked_chapter(chapter, request.start, request.end)
    ));
    if has_selection {
        p.push_str(&format!(
            "\n# Selected passage\n\n<selection>\n{}\n</selection>\n",
            request.selected
        ));
    }
    p
}

/// Remove wrappers models like to add: a single surrounding code fence or quotes.
pub fn clean_output(text: &str) -> String {
    let trimmed = text.trim();
    let unfenced = trimmed
        .strip_prefix("```")
        .and_then(|rest| rest.split_once('\n'))
        .and_then(|(_, body)| body.trim_end().strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let unquoted = unfenced
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .filter(|s| !s.contains('"'))
        .unwrap_or(unfenced);
    unquoted
        .replace(CURSOR, "")
        .replace(SEL_START, "")
        .replace(SEL_END, "")
        .trim()
        .to_string()
}

fn char_to_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(b, _)| b)
}

/// Replace the originally selected passage. If the buffer changed, find the passage again.
/// Returns the character range of the inserted text.
pub fn apply_replacement(
    content: &mut String,
    request: &Request,
    replacement: &str,
) -> Result<(usize, usize), String> {
    let (a, b) = (
        char_to_byte(content, request.start),
        char_to_byte(content, request.end),
    );
    let byte_start =
        if content.get(a..b) == Some(request.selected.as_str()) {
            a
        } else {
            let mut matches = content.match_indices(&request.selected);
            match (matches.next(), matches.next()) {
                (Some((found, _)), None) => found,
                _ => return Err(
                    "The selected passage changed since the request. Copy the suggestion instead."
                        .into(),
                ),
            }
        };
    content.replace_range(byte_start..byte_start + request.selected.len(), replacement);
    let start = content[..byte_start].chars().count();
    Ok((start, start + replacement.chars().count()))
}

/// Insert new paragraphs at a character index, keeping blank lines around them.
/// Returns the character range of the inserted text (without the added spacing).
pub fn apply_insertion(content: &mut String, at: usize, text: &str) -> (usize, usize) {
    let at = char_to_byte(content, at.min(content.chars().count()));
    let before = &content[..at];
    let after = &content[at..];
    let prefix = if before.is_empty() || before.ends_with("\n\n") {
        ""
    } else if before.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let suffix = if after.is_empty() || after.starts_with("\n\n") {
        ""
    } else if after.starts_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let start = before.chars().count() + prefix.chars().count();
    content.insert_str(at, &format!("{prefix}{text}{suffix}"));
    (start, start + text.chars().count())
}

/// Split a command line typed in the settings field. Supports '…' and "…" quoting.
pub fn split_command(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for ch in line.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => {
                quote = Some(ch);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (None, c) => current.push(c),
        }
    }
    if started || !current.is_empty() {
        parts.push(current);
    }
    parts
}

pub fn join_command(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| {
            if part.is_empty() || part.contains(char::is_whitespace) {
                format!("\"{part}\"")
            } else {
                part.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ----- Background process ------------------------------------------------------

enum Event {
    Stdout(String),
    Stderr(String),
    Done(Result<(), String>),
}

/// A running assistant command. Output streams in as it arrives.
pub struct Job {
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
    events: Receiver<Event>,
    pub output: String,
    pub stderr: String,
    pub finished: Option<Result<(), String>>,
}

impl Job {
    pub fn start(
        config: &AssistantConfig,
        cwd: &Path,
        prompt: String,
        ctx: egui::Context,
    ) -> io::Result<Job> {
        let mut parts = config
            .command
            .clone()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No command configured"))?;
        if config.prompt_as_argument {
            parts.push(prompt);
            Job::run(&parts, cwd, None, ctx)
        } else {
            Job::run(&parts, cwd, Some(prompt), ctx)
        }
    }

    /// Run any command in the background, writing `input` to its stdin.
    pub fn run(
        parts: &[String],
        cwd: &Path,
        input: Option<String>,
        ctx: egui::Context,
    ) -> io::Result<Job> {
        let (program, args) = parts
            .split_first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Empty command"))?;
        let mut command = crate::process::user_program(program, cwd);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(Some(child)));
        let cancelled = Arc::new(AtomicBool::new(false));
        let (tx, events) = mpsc::channel();

        thread::spawn(move || {
            if let (Some(mut stdin), Some(input)) = (stdin, input) {
                let _ = stdin.write_all(input.as_bytes());
            }
            // Dropping stdin closes it, so the command knows the prompt is complete.
        });
        let out = stdout.map(|pipe| spawn_reader(pipe, tx.clone(), ctx.clone(), Event::Stdout));
        let err = stderr.map(|pipe| spawn_reader(pipe, tx.clone(), ctx.clone(), Event::Stderr));
        {
            let child = Arc::clone(&child);
            let cancelled = Arc::clone(&cancelled);
            thread::spawn(move || {
                for reader in [out, err].into_iter().flatten() {
                    let _ = reader.join();
                }
                let result = loop {
                    let status = match child.lock().unwrap().as_mut() {
                        Some(c) => c.try_wait(),
                        None => break Err("Process lost".to_string()),
                    };
                    match status {
                        Ok(Some(_)) if cancelled.load(Ordering::SeqCst) => {
                            break Err("Cancelled".into())
                        }
                        Ok(Some(status)) if status.success() => break Ok(()),
                        Ok(Some(status)) => break Err(format!("Command failed ({status})")),
                        Ok(None) => thread::sleep(Duration::from_millis(30)),
                        Err(err) => break Err(err.to_string()),
                    }
                };
                let _ = tx.send(Event::Done(result));
                ctx.request_repaint();
            });
        }
        Ok(Job {
            child,
            cancelled,
            events,
            output: String::new(),
            stderr: String::new(),
            finished: None,
        })
    }

    /// Apply pending output. Returns true if anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        for event in self.events.try_iter() {
            changed = true;
            match event {
                Event::Stdout(text) => self.output.push_str(&text),
                Event::Stderr(text) => self.stderr.push_str(&text),
                Event::Done(result) => self.finished = Some(result),
            }
        }
        changed
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if self.finished.is_none() {
            self.cancel();
        }
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    mut pipe: R,
    tx: Sender<Event>,
    ctx: egui::Context,
    wrap: fn(String) -> Event,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut pending = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let read = match pipe.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&buffer[..read]);
            // Only forward complete UTF-8 sequences; keep a split character for later.
            let valid = match std::str::from_utf8(&pending) {
                Ok(text) => text.len(),
                Err(err) if err.error_len().is_none() => err.valid_up_to(),
                Err(_) => pending.len(),
            };
            if valid > 0 {
                let text = String::from_utf8_lossy(&pending[..valid]).into_owned();
                pending.drain(..valid);
                if tx.send(wrap(text)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        }
        if !pending.is_empty() {
            let _ = tx.send(wrap(String::from_utf8_lossy(&pending).into_owned()));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn novel() -> &'static Kind {
        kinds::default_kind()
    }

    /// The Novel kind's action with this label.
    fn kind_action(label: &str) -> Action {
        let action = novel().assistant.actions.iter().find(|a| a.label == label);
        Action::Kind(action.expect(label).clone())
    }

    fn rewrite() -> Action {
        kind_action("Rewrite selection")
    }

    fn request(action: Action, text: &str, start: usize, end: usize) -> Request {
        Request {
            action,
            instruction: String::new(),
            file: Some("chapters/001-the-harbor.md".into()),
            chapter_title: "The Harbor".into(),
            start,
            end,
            selected: text.chars().skip(start).take(end - start).collect(),
        }
    }

    #[test]
    fn prompt_marks_selection_and_includes_canon() {
        let chapter = "Mara waited. The tide rose.";
        let req = request(rewrite(), chapter, 13, 27);
        let context = Context {
            guide: None,
            files: vec![ContextFile {
                file: "world-rules.md".into(),
                heading: "World rules".into(),
                summary: "world rules".into(),
                text: "No magic.".into(),
            }],
            sheets: vec![("characters/mara.md".into(), "# Mara".into())],
            sheet_words: ("character sheet".into(), "character sheets".into()),
            clips: vec![Clip {
                citation: "research/harbor.pdf, p. 3".into(),
                text: "The harbor froze in 1889.".into(),
            }],
        };
        let prompt = build_prompt(&req, chapter, &context, novel());
        assert!(prompt.contains(
            "## research/harbor.pdf, p. 3\n\n<reference>\nThe harbor froze in 1889.\n</reference>"
        ));
        assert!(context.summary().contains("1 reference passage"));
        assert!(prompt.contains("Mara waited. ⟦SELECTION START⟧The tide rose.⟦SELECTION END⟧"));
        assert!(prompt.contains("No magic."));
        assert!(prompt.contains("## characters/mara.md"));
        assert!(prompt.contains("<selection>\nThe tide rose.\n</selection>"));
        assert!(prompt.contains("Never translate"));

        let cont = build_prompt(
            &request(kind_action("Continue the scene"), chapter, 12, 12),
            chapter,
            &context,
            novel(),
        );
        assert!(cont.contains("Mara waited.⟦CURSOR⟧ The tide rose."));
        assert!(!cont.contains("<selection>"));
        let without = Context {
            clips: Vec::new(),
            ..context
        };
        assert!(!build_prompt(&req, chapter, &without, novel()).contains("# Reference material"));
    }

    #[test]
    fn long_chapters_are_windowed_around_the_cursor() {
        let chapter = "word ".repeat(30_000);
        let at = chapter.chars().count() - 10;
        let marked = marked_chapter(&chapter, at, at);
        assert!(marked.contains(CURSOR));
        assert!(marked.starts_with("[… earlier text omitted …]"));
        assert!(marked.chars().count() < 61_000);
    }

    #[test]
    fn context_includes_only_mentioned_characters() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("characters")).unwrap();
        std::fs::write(root.join("world-rules.md"), "# World\nNo magic.").unwrap();
        std::fs::write(root.join("characters/mara.md"), "# Mara Venn").unwrap();
        std::fs::write(root.join("characters/tobias.md"), "# Tobias Rell").unwrap();
        let characters = vec![
            (root.join("characters/mara.md"), "Mara Venn".to_string()),
            (root.join("characters/tobias.md"), "Tobias Rell".to_string()),
        ];
        let context = Context::gather(root, novel(), "Mara looked at the sea.", &characters);
        assert_eq!(context.sheets.len(), 1);
        assert!(context.sheets[0].0.ends_with("mara.md"));
        assert_eq!(context.files[0].text, "# World\nNo magic.");
        assert_eq!(context.summary(), "world rules, 1 character sheet");
    }

    #[test]
    fn output_cleanup() {
        assert_eq!(clean_output("```markdown\nHello.\n```\n"), "Hello.");
        assert_eq!(clean_output("  \"Quoted.\"  "), "Quoted.");
        assert_eq!(
            clean_output("She said \"hi\" and \"bye\"."),
            "She said \"hi\" and \"bye\"."
        );
        assert_eq!(clean_output("Text ⟦CURSOR⟧"), "Text");
    }

    #[test]
    fn replacement_follows_moved_text_and_refuses_ambiguity() {
        let original = "Alpha. Beta. Gamma.";
        let req = request(rewrite(), original, 7, 12);
        let mut content = original.to_string();
        assert_eq!(apply_replacement(&mut content, &req, "BETA!"), Ok((7, 12)));
        assert_eq!(content, "Alpha. BETA! Gamma.");

        // The author typed before the passage: it is found again.
        let mut moved = "New. Alpha. Beta. Gamma.".to_string();
        assert!(apply_replacement(&mut moved, &req, "B").is_ok());
        assert_eq!(moved, "New. Alpha. B Gamma.");

        let mut gone = "Alpha. Gamma.".to_string();
        assert!(apply_replacement(&mut gone, &req, "B").is_err());
        assert_eq!(gone, "Alpha. Gamma.");
    }

    #[test]
    fn insertion_keeps_paragraph_spacing() {
        let mut text = "One.\n\nTwo.".to_string();
        let range = apply_insertion(&mut text, 4, "New.");
        assert_eq!(text, "One.\n\nNew.\n\nTwo.");
        assert_eq!(range, (6, 10));
        let mut end = "One.".to_string();
        apply_insertion(&mut end, 4, "New.");
        assert_eq!(end, "One.\n\nNew.");
        let mut empty = String::new();
        assert_eq!(apply_insertion(&mut empty, 0, "New."), (0, 4));
        assert_eq!(empty, "New.");
    }

    #[test]
    fn command_line_splitting() {
        assert_eq!(split_command("claude -p"), ["claude", "-p"]);
        assert_eq!(
            split_command(r#"ollama run "my model" '' x"#),
            ["ollama", "run", "my model", "", "x"]
        );
        let parts = split_command(r#"a "b c""#);
        assert_eq!(split_command(&join_command(&parts)), parts);
    }

    #[test]
    fn job_streams_output_from_stdin() {
        let config = AssistantConfig {
            command: Some(vec!["cat".into()]),
            prompt_as_argument: false,
        };
        let mut job = Job::start(
            &config,
            Path::new("."),
            "olá mundo".into(),
            egui::Context::default(),
        )
        .unwrap();
        for _ in 0..1000 {
            job.poll();
            if job.finished.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(job.finished, Some(Ok(())));
        assert_eq!(job.output, "olá mundo");
    }

    #[test]
    fn job_can_be_cancelled_and_reports_missing_commands() {
        // One argument only: BSD sleep (macOS) rejects more, and would exit before
        // being cancelled.
        let config = AssistantConfig {
            command: Some(vec!["sleep".into(), "30".into()]),
            prompt_as_argument: false,
        };
        let mut job = Job::start(
            &config,
            Path::new("."),
            String::new(),
            egui::Context::default(),
        )
        .unwrap();
        job.cancel();
        // Generous timeouts: CI machines can be slow to start processes.
        for _ in 0..1000 {
            job.poll();
            if job.finished.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(job.finished, Some(Err("Cancelled".into())));

        let missing = AssistantConfig {
            command: Some(vec!["quillwright-no-such-command".into()]),
            prompt_as_argument: false,
        };
        assert!(Job::start(
            &missing,
            Path::new("."),
            String::new(),
            egui::Context::default()
        )
        .is_err());
    }

    #[test]
    fn legal_kind_sends_its_own_files_and_actions() {
        let legal = kinds::find("legal", "pt-BR");
        assert_eq!(legal.language, "pt-BR");
        let temp = tempfile::tempdir().unwrap();
        let root = crate::project::create(&temp.path().join("caso"), legal).unwrap();
        std::fs::write(
            root.join("fatos.md"),
            "# Fatos\n\n1. Contrato assinado (doc. 1).",
        )
        .unwrap();
        let sheets = vec![(root.join("partes/cliente.md"), "Ana Souza".to_string())];
        let text = "Ana Souza firmou o contrato.";
        let context = Context::gather(&root, legal, text, &sheets);
        assert_eq!(
            context.summary(),
            "facts, timeline, evidence list, client questions, 1 party sheet, AGENT.md"
        );
        let action = legal
            .assistant
            .actions
            .iter()
            .find(|a| a.label == "Check against facts")
            .unwrap();
        let req = Request {
            action: Action::Kind(action.clone()),
            instruction: String::new(),
            file: Some("pecas/001-peticao-inicial.md".into()),
            chapter_title: "Petição inicial".into(),
            start: 0,
            end: 0,
            selected: String::new(),
        };
        let prompt = build_prompt(&req, text, &context, legal);
        assert!(prompt.contains("helping a lawyer"));
        assert!(prompt.contains("# Facts (fatos.md)\n\n# Fatos"));
        assert!(prompt.contains("# Party sheets\n\n## partes/cliente.md"));
        assert!(prompt.contains("# Current filing: \"Petição inicial\""));
        assert!(prompt.contains("<filing>\n⟦CURSOR⟧Ana Souza"));
        assert!(!prompt.contains("fiction"));
    }
}
