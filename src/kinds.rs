//! Project kinds: what kind of writing a project holds (a novel, a legal case, ...).
//! Each kind is a folder in `assets/kinds/<id>/`: a `kind.toml` with what all its
//! languages share (labels, the assistant's context files, actions, and prompts), and one
//! subfolder per language (`en/`, `pt-BR/`) with a `kind.toml` merged over it (folder and
//! file names, detection) and the `template/` copied by New Project. The kind is never
//! stored in the writing project; it is inferred from the files there.
//! See docs/project-kinds.md.
use serde::Deserialize;
use std::{path::Path, sync::OnceLock};

// `KIND_FILES`: every file under assets/kinds, listed by build.rs.
include!(concat!(env!("OUT_DIR"), "/kind_files.rs"));

/// The kind used for folders that match no kind.
const DEFAULT_KIND: &str = "novel";
/// The language listed first, and used when a kind has several and none was chosen.
const FIRST_LANGUAGE: &str = "en";

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Kind {
    #[serde(skip)]
    pub id: String,
    /// The language folder, such as `en` or `pt-BR`.
    #[serde(skip)]
    pub language: String,
    /// The language as shown in New Project ("Português (Brasil)").
    pub language_name: String,
    pub name: String,
    pub description: String,
    /// Relative paths whose presence marks a project of this kind ("dir/" for a folder).
    pub detect: Vec<String>,
    pub first_file: String,
    pub new_project_note: String,
    pub research: Research,
    pub documents: Section,
    pub sheets: Section,
    pub assistant: Assistant,
    /// The template copied by New Project: (relative path, contents).
    #[serde(skip)]
    pub files: Vec<(String, String)>,
}

/// The folder of reference material (PDFs, images, notes), read only in the app.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Research {
    pub folder: String,
    pub label: String,
    /// Shown while the folder is empty.
    pub hint: String,
}

impl Default for Research {
    fn default() -> Self {
        Research {
            folder: "research".into(),
            label: "Research".into(),
            hint: "PDFs, images, and notes — press + to add.".into(),
        }
    }
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
    /// Placeholder of the instruction box.
    pub ask_hint: String,
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

/// Merge `over` into `base`: tables merge key by key, anything else is replaced.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(inner)), toml::Value::Table(value)) => merge(inner, value),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

/// Build every (kind, language) from `files`: (path under assets/kinds, contents).
fn load(files: &[(&str, &str)]) -> Result<Vec<Kind>, String> {
    let file = |path: &str| files.iter().find(|(p, _)| *p == path).map(|(_, t)| *t);
    let parse = |path: &str| -> Result<toml::Table, String> {
        let text = file(path).ok_or_else(|| format!("{path} is missing"))?;
        text.parse::<toml::Table>()
            .map_err(|e| format!("{path}: {e}"))
    };
    let mut ids: Vec<&str> = files
        .iter()
        .filter_map(|(path, _)| path.strip_suffix("/kind.toml"))
        .filter(|id| !id.contains('/'))
        .collect();
    ids.sort_by_key(|id| (*id != DEFAULT_KIND, *id));
    let mut kinds = Vec::new();
    for id in ids {
        let base = parse(&format!("{id}/kind.toml"))?;
        let mut languages: Vec<&str> = files
            .iter()
            .filter_map(|(path, _)| path.strip_prefix(id)?.strip_prefix('/'))
            .filter_map(|rest| rest.strip_suffix("/kind.toml"))
            .filter(|lang| !lang.contains('/'))
            .collect();
        languages.sort_by_key(|lang| (*lang != FIRST_LANGUAGE, *lang));
        if languages.is_empty() {
            return Err(format!("kind {id} has no language folder"));
        }
        for language in languages {
            let mut table = base.clone();
            merge(&mut table, parse(&format!("{id}/{language}/kind.toml"))?);
            let mut kind: Kind = table
                .try_into()
                .map_err(|e| format!("kind {id} ({language}): {e}"))?;
            kind.id = id.to_string();
            kind.language = language.to_string();
            let template = format!("{id}/{language}/template/");
            kind.files = files
                .iter()
                .filter_map(|(path, text)| {
                    Some((path.strip_prefix(&template)?.to_string(), text.to_string()))
                })
                .collect();
            kinds.push(kind);
        }
    }
    Ok(kinds)
}

/// Every known kind, once per language. The default kind comes first, and each kind's
/// English variant before its other languages.
pub fn all() -> &'static [Kind] {
    static KINDS: OnceLock<Vec<Kind>> = OnceLock::new();
    // Built-in kinds are checked by the tests, so this cannot fail at run time.
    KINDS.get_or_init(|| load(KIND_FILES).expect("built-in project kinds"))
}

pub fn default_kind() -> &'static Kind {
    &all()[0]
}

/// The kinds as offered by New Project: the first language variant of each.
pub fn choices() -> Vec<&'static Kind> {
    let mut seen = Vec::new();
    all()
        .iter()
        .filter(|kind| {
            let new = !seen.contains(&&kind.id);
            seen.push(&kind.id);
            new
        })
        .collect()
}

/// The languages of the kind `id`.
pub fn languages(id: &str) -> Vec<&'static Kind> {
    all().iter().filter(|kind| kind.id == id).collect()
}

/// The kind `id` in `language`, or its first language.
pub fn find(id: &str, language: &str) -> &'static Kind {
    let variants = languages(id);
    variants
        .iter()
        .find(|kind| kind.language == language)
        .or(variants.first())
        .copied()
        .unwrap_or_else(default_kind)
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
        assert_eq!(all()[0].id, DEFAULT_KIND);
        assert_eq!(all()[0].language, FIRST_LANGUAGE);
        for kind in all() {
            let id = format!("{} ({})", kind.id, kind.language);
            assert!(!kind.name.is_empty(), "{id}: name");
            assert!(!kind.language_name.is_empty(), "{id}: language_name");
            assert!(!kind.detect.is_empty(), "{id}: detect");
            assert!(kind.file(&kind.first_file).is_some(), "{id}: first_file");
            assert!(!kind.research.folder.is_empty(), "{id}: research");
            for section in [&kind.documents, &kind.sheets] {
                assert!(!section.folder.is_empty(), "{id}: folder");
                assert!(!section.singular.is_empty() && !section.plural.is_empty());
                if let Some(template) = &section.template {
                    assert!(kind.file(template).is_some(), "{id}: {template}");
                }
            }
            assert!(
                kind.file(&kind.assistant.guide).is_some(),
                "{id}: the guide is in the template"
            );
            for file in &kind.assistant.context {
                assert!(kind.file(&file.file).is_some(), "{id}: {}", file.file);
                assert!(file.limit > 0, "{id}: {} limit", file.file);
            }
            assert!(!kind.assistant.preamble.is_empty(), "{id}: preamble");
            assert!(!kind.assistant.ask_hint.is_empty(), "{id}: ask_hint");
            for action in &kind.assistant.actions {
                assert!(!action.label.is_empty() && !action.task.is_empty());
                assert!(!action.output.is_empty(), "{id}: {}", action.label);
                if action.result == ResultKind::Replacement {
                    assert!(action.needs_selection, "{id}: {}", action.label);
                }
            }
            // A new project of this kind is recognized as this kind.
            let temp = tempfile::tempdir().unwrap();
            let root = crate::project::create(&temp.path().join("p"), kind).unwrap();
            let found = detect(&root);
            assert_eq!((&found.id, &found.language), (&kind.id, &kind.language));
        }
        // Every kind is offered in English.
        for kind in choices() {
            assert_eq!(kind.language, FIRST_LANGUAGE, "{}", kind.id);
        }
    }

    #[test]
    fn language_files_are_merged_over_the_kind() {
        let files = [
            (
                "k/kind.toml",
                "name = \"K\"\ndetect = [\"a/\"]\n[documents]\nfolder = \"a\"\nlabel = \"Docs\"\n",
            ),
            ("k/en/kind.toml", "language_name = \"English\""),
            (
                "k/pt-BR/kind.toml",
                "language_name = \"Português\"\ndetect = [\"b/\"]\n[documents]\nfolder = \"b\"\n",
            ),
            ("k/pt-BR/template/b/1.md", "# Um"),
        ];
        let kinds = load(&files).unwrap();
        assert_eq!(kinds.len(), 2);
        let (en, pt) = (&kinds[0], &kinds[1]);
        assert_eq!(
            (en.language.as_str(), pt.language.as_str()),
            ("en", "pt-BR")
        );
        assert_eq!(pt.documents.folder, "b");
        assert_eq!(pt.documents.label, "Docs"); // Kept from the shared file.
        assert_eq!(pt.detect, ["b/"]); // Lists are replaced, not appended.
        assert_eq!(pt.file("b/1.md"), Some("# Um"));
        assert!(en.files.is_empty());
        assert!(load(&[("k/kind.toml", "name = \"K\"")]).is_err());
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
