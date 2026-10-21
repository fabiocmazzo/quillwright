//! Project kinds: what kind of writing a project holds (a novel, later a legal case, ...).
//! Each kind is a folder in `assets/kinds/<id>/` with a `kind.toml` (folders, labels, the
//! assistant's context files, actions, and prompts) and a `template/` copied by New
//! Project. The kind is never stored in the writing project; it is inferred from the
//! files there. See docs/project-kinds.md.
use serde::Deserialize;
use std::{path::Path, sync::OnceLock};

/// Built-in kinds: (id, kind.toml, template files as (relative path, contents)).
/// The first one is the default for folders that match no kind.
type Builtin = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const BUILTIN: &[Builtin] = &[(
    "novel",
    include_str!("../assets/kinds/novel/kind.toml"),
    &[
        (
            "AGENT.md",
            include_str!("../assets/kinds/novel/template/AGENT.md"),
        ),
        (
            "world-rules.md",
            include_str!("../assets/kinds/novel/template/world-rules.md"),
        ),
        (
            "characters/example-character.md",
            include_str!("../assets/kinds/novel/template/characters/example-character.md"),
        ),
        (
            "chapters/001-opening.md",
            include_str!("../assets/kinds/novel/template/chapters/001-opening.md"),
        ),
    ],
)];

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Kind {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    pub description: String,
    /// Relative paths whose presence marks a project of this kind ("dir/" for a folder).
    pub detect: Vec<String>,
    pub first_file: String,
    pub new_project_note: String,
    pub documents: Section,
    pub sheets: Section,
    pub assistant: Assistant,
    /// The template copied by New Project: (relative path, contents).
    #[serde(skip)]
    pub files: Vec<(String, String)>,
}

/// A folder of Markdown files shown as one sidebar section.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Section {
    pub folder: String,
    /// Sidebar heading, such as "Chapters".
    pub label: String,
    pub singular: String,
    pub plural: String,
    /// File name for new files, without extension; `{n}` is the number.
    pub new_name: String,
    /// Title (level-1 heading) of new files; `{n}` is the number.
    pub new_title: String,
    pub empty_hint: String,
    /// Home screen button when the section is empty.
    pub first_action: String,
    /// Template file whose body starts new files.
    pub template: Option<String>,
    /// How the assistant panel and prompt name these files ("character sheet").
    pub prompt_singular: String,
    pub prompt_plural: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Assistant {
    /// Who the assistant is and its rules; the start of every prompt.
    pub preamble: String,
    /// A guide for agents sent with every request (AGENT.md).
    pub guide: String,
    pub guide_note: String,
    /// Said before the reference passages the author added.
    pub reference_note: String,
    pub context: Vec<ContextFile>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ContextFile {
    pub file: String,
    pub heading: String,
    /// Named in the panel's list of what was sent ("world rules").
    pub summary: String,
    /// Longest text sent, in characters.
    pub limit: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Action {
    pub label: String,
    pub hint: String,
    pub needs_selection: bool,
    pub result: ResultKind,
    pub task: String,
    pub output: String,
}

/// How an answer is meant to be used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum ResultKind {
    /// Replaces the selection.
    #[serde(rename = "replace")]
    Replacement,
    /// Inserted at the cursor.
    #[serde(rename = "insert")]
    Insertion,
    #[default]
    #[serde(rename = "notes")]
    Notes,
}

impl Section {
    /// "New chapter".
    pub fn new_label(&self) -> String {
        format!("New {}", self.singular)
    }

    /// "New Chapter", for menus.
    pub fn new_menu_label(&self) -> String {
        let title: Vec<String> = self.singular.split(' ').map(capitalize).collect();
        format!("New {}", title.join(" "))
    }

    /// "1 chapter", "3 chapters".
    pub fn count(&self, n: usize) -> String {
        let word = if n == 1 { &self.singular } else { &self.plural };
        format!("{n} {word}")
    }
}

pub fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn parse(id: &str, toml_text: &str, files: &[(&str, &str)]) -> Result<Kind, String> {
    let mut kind: Kind = toml::from_str(toml_text).map_err(|e| format!("kind {id}: {e}"))?;
    kind.id = id.to_string();
    kind.files = files
        .iter()
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect();
    Ok(kind)
}

/// Every known kind; the first is the default.
pub fn all() -> &'static [Kind] {
    static KINDS: OnceLock<Vec<Kind>> = OnceLock::new();
    KINDS.get_or_init(|| {
        BUILTIN
            .iter()
            .map(|(id, toml_text, files)| {
                // Built-in kinds are checked by the tests, so this cannot fail at run time.
                parse(id, toml_text, files).expect("built-in project kind")
            })
            .collect()
    })
}

pub fn default_kind() -> &'static Kind {
    &all()[0]
}

/// The kind of the project at `root`: the first whose `detect` paths exist there, or
/// the default.
pub fn detect(root: &Path) -> &'static Kind {
    all()
        .iter()
        .find(|kind| kind.matches(root))
        .unwrap_or_else(default_kind)
}

impl Kind {
    fn matches(&self, root: &Path) -> bool {
        self.detect
            .iter()
            .any(|entry| match entry.strip_suffix('/') {
                Some(dir) => root.join(dir).is_dir(),
                None => root.join(entry).is_file(),
            })
    }

    /// A template file's contents.
    pub fn file(&self, path: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, text)| text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_kinds_load_and_are_complete() {
        assert_eq!(all()[0].id, "novel");
        for kind in all() {
            let id = &kind.id;
            assert!(!kind.name.is_empty(), "{id}: name");
            assert!(kind.file(&kind.first_file).is_some(), "{id}: first_file");
            for section in [&kind.documents, &kind.sheets] {
                assert!(!section.folder.is_empty(), "{id}: folder");
                assert!(!section.singular.is_empty() && !section.plural.is_empty());
                if let Some(template) = &section.template {
                    assert!(kind.file(template).is_some(), "{id}: {template}");
                }
            }
            assert!(!kind.assistant.preamble.is_empty(), "{id}: preamble");
            for action in &kind.assistant.actions {
                assert!(!action.label.is_empty() && !action.task.is_empty());
                assert!(!action.output.is_empty(), "{id}: {}", action.label);
                if action.result == ResultKind::Replacement {
                    assert!(action.needs_selection, "{id}: {}", action.label);
                }
            }
            // Template files are listed in the embedded table with the paths New Project
            // writes; each must exist in the kind's template folder.
            let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/kinds")
                .join(id)
                .join("template");
            for (path, text) in &kind.files {
                assert_eq!(&std::fs::read_to_string(dir.join(path)).unwrap(), text);
            }
        }
    }

    #[test]
    fn detects_the_kind_from_visible_files_only() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(detect(temp.path()).id, "novel"); // The default.
        std::fs::create_dir(temp.path().join("chapters")).unwrap();
        assert_eq!(detect(temp.path()).id, "novel");
        let novel = default_kind();
        assert!(novel.matches(temp.path()));
        // A file named like a folder entry does not count as the folder.
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join("chapters"), "").unwrap();
        assert!(!novel.matches(other.path()));
    }

    #[test]
    fn section_labels() {
        let chapters = &default_kind().documents;
        assert_eq!(chapters.new_label(), "New chapter");
        assert_eq!(chapters.new_menu_label(), "New Chapter");
        assert_eq!(chapters.count(1), "1 chapter");
        assert_eq!(chapters.count(3), "3 chapters");
    }
}
