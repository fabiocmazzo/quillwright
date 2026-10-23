//! Spell checking with Hunspell dictionaries, through spellbook (pure Rust).
//!
//! Dictionaries come from the user data folder, the system's Hunspell folders, or the
//! bundled en_US. Words the author accepts for a book live in a plain `dictionary.txt`
//! at the project root, one per line, so agents and Git see them like any other file.
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::Range,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
};

/// The project's own word list, at the project root.
pub const PROJECT_WORDS_FILE: &str = "dictionary.txt";
const DEFAULT_LANGUAGE: &str = "en_US";
const BUNDLED_AFF: &str = include_str!("../assets/dictionaries/en_US.aff");
const BUNDLED_DIC: &str = include_str!("../assets/dictionaries/en_US.dic");
const MAX_SUGGESTIONS: usize = 6;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SpellingConfig {
    pub enabled: bool,
    /// Hunspell dictionary name, for example "en_US" or "pt_BR", for projects whose
    /// `metadata.yaml` sets no language. Empty: en_US.
    pub language: String,
}

impl Default for SpellingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            language: String::new(),
        }
    }
}

/// "pt-br" or "pt_BR" → "pt_BR"; "en" → "en".
pub fn normalize(tag: &str) -> String {
    let mut parts = tag.trim().split(['-', '_']);
    let lang = parts.next().unwrap_or_default().to_lowercase();
    match parts.next().filter(|r| !r.is_empty()) {
        Some(region) => format!("{lang}_{}", region.to_uppercase()),
        None => lang,
    }
}

/// Folder where users can add `<language>.aff` and `<language>.dic` files.
pub fn user_dictionary_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "quillwright", "quillwright")
        .map(|dirs| dirs.data_dir().join("dictionaries"))
}

/// Folders searched for dictionaries, the user's folder first.
fn dictionary_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = user_dictionary_dir().into_iter().collect();
    if cfg!(target_os = "linux") {
        for dir in [
            "/usr/share/hunspell",
            "/usr/share/myspell",
            "/usr/share/myspell/dicts",
        ] {
            dirs.push(dir.into());
        }
    }
    if cfg!(target_os = "macos") {
        if let Some(home) = directories::BaseDirs::new() {
            dirs.push(home.home_dir().join("Library/Spelling"));
        }
        dirs.push("/Library/Spelling".into());
    }
    dirs
}

/// Dictionary names found in `dirs`, plus the bundled one, sorted and deduplicated.
fn languages_in(dirs: &[PathBuf]) -> Vec<String> {
    let mut found: Vec<String> = dirs
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "dic" && path.with_extension("aff").is_file())
                .then(|| path.file_stem()?.to_str().map(str::to_string))?
        })
        .collect();
    found.push(DEFAULT_LANGUAGE.into());
    found.sort();
    found.dedup();
    found
}

/// Dictionary names available on this computer.
pub fn available() -> Vec<String> {
    languages_in(&dictionary_dirs())
}

/// The `.aff` and `.dic` text for `language`. A name without a region ("pt") picks the
/// first matching regional dictionary ("pt_BR").
fn find(language: &str, dirs: &[PathBuf]) -> Option<(String, String)> {
    let names = languages_in(dirs);
    let name = names.iter().find(|n| n.as_str() == language).or_else(|| {
        names
            .iter()
            .find(|n| n.starts_with(&format!("{language}_")))
    })?;
    for dir in dirs {
        let aff = dir.join(format!("{name}.aff"));
        let dic = dir.join(format!("{name}.dic"));
        if let (Ok(aff), Ok(dic)) = (read_text(&aff), read_text(&dic)) {
            return Some((aff, dic));
        }
    }
    (name == DEFAULT_LANGUAGE).then(|| (BUNDLED_AFF.into(), BUNDLED_DIC.into()))
}

/// Hunspell files are usually UTF-8; old ones may be Latin-1, which maps byte to char.
fn read_text(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|e| e.into_bytes().iter().map(|&b| b as char).collect());
    Ok(text.trim_start_matches('\u{feff}').to_string())
}

const METADATA_FILE: &str = "metadata.yaml";

/// The `lang` in the project's `metadata.yaml`, normalized ("pt_BR"), if any.
pub fn project_language(project_root: &Path) -> Option<String> {
    let mut meta = crate::export::book::Meta::default();
    let yaml = std::fs::read_to_string(project_root.join(METADATA_FILE)).ok()?;
    let _ = crate::export::book::read_metadata(&yaml, &mut meta);
    (!meta.language.trim().is_empty()).then(|| normalize(&meta.language))
}

/// The language to check: the project's `metadata.yaml`, else the configured default,
/// else en_US.
pub fn resolve_language(default: &str, project_root: &Path) -> String {
    project_language(project_root).unwrap_or_else(|| {
        if default.trim().is_empty() {
            DEFAULT_LANGUAGE.into()
        } else {
            normalize(default)
        }
    })
}

/// `yaml` with its top-level `lang` (or `language`) set to `tag`, keeping every other
/// line, comment, and the order as they are. Adds `lang:` if there is none.
fn with_language(yaml: &str, tag: &str) -> String {
    let mut lines: Vec<String> = yaml.lines().map(str::to_string).collect();
    let key = lines.iter().position(|line| {
        ["lang", "language"].iter().any(|key| {
            line.strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with(':'))
        })
    });
    match key {
        Some(i) => {
            let name = lines[i]
                .split(':')
                .next()
                .unwrap_or("lang")
                .trim_end()
                .to_string();
            lines[i] = format!("{name}: {tag}");
        }
        None => {
            // Keep a trailing document end marker last.
            let at = lines
                .iter()
                .rposition(|line| line.trim() == "...")
                .unwrap_or(lines.len());
            lines.insert(at, format!("lang: {tag}"));
        }
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Set the book's language in `metadata.yaml` ("pt_BR" is written as "pt-BR"), creating
/// the file if needed. Export uses the same value.
pub fn set_project_language(project_root: &Path, language: &str) -> Result<(), String> {
    let path = project_root.join(METADATA_FILE);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.to_string()),
    };
    let text = with_language(&existing, &language.replace('_', "-"));
    crate::file_sync::write_checked(&path, &text, None)
        .map_err(|_| crate::i18n::t!("write-failed", path = path.display().to_string()))
}

/// Byte ranges of the words in `text`. Apostrophes and hyphens inside a word are kept
/// ("don't", "disse-lhe"); tokens with digits are skipped.
pub fn words(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut start = None;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (i, &(at, ch)) in chars.iter().enumerate() {
        let joiner = matches!(ch, '\'' | '’' | '-')
            && start.is_some()
            && chars
                .get(i + 1)
                .is_some_and(|&(_, next)| next.is_alphanumeric());
        if ch.is_alphanumeric() || joiner {
            start.get_or_insert(at);
        } else if let Some(s) = start.take() {
            result.push(s..at);
        }
    }
    if let Some(s) = start {
        result.push(s..text.len());
    }
    result.retain(|range| !text[range.clone()].chars().any(|c| c.is_numeric()));
    result
}

/// The word at `char_index` (a source character index) as a character range.
pub fn word_at(text: &str, char_index: usize) -> Option<(Range<usize>, String)> {
    let byte = text
        .char_indices()
        .nth(char_index)
        .map_or(text.len(), |(i, _)| i);
    let range = words(text)
        .into_iter()
        .find(|r| r.start <= byte && byte <= r.end)?;
    let start = text[..range.start].chars().count();
    let word = text[range].to_string();
    Some((start..start + word.chars().count(), word))
}

type Loaded = Result<spellbook::Dictionary, String>;

#[derive(Default)]
pub struct Speller {
    /// The language asked for, resolved (for example "pt_BR").
    pub language: String,
    dictionary: Option<Arc<spellbook::Dictionary>>,
    loading: Option<mpsc::Receiver<Loaded>>,
    /// Why no dictionary is in use, shown in the View menu.
    pub error: Option<String>,
    project_root: PathBuf,
    /// Lowercase words from `dictionary.txt` and the character sheets.
    project_words: HashSet<String>,
    ignored: HashSet<String>,
    cache: RefCell<HashMap<String, bool>>,
    /// The last word asked for in [`Speller::suggestions`], with its suggestions.
    last_suggestions: RefCell<Option<(String, Vec<String>)>>,
}

impl Speller {
    /// Load the dictionary for `language` in the background, if it is not loaded yet.
    pub fn set_language(&mut self, language: &str) {
        if self.language == language && (self.dictionary.is_some() || self.loading.is_some()) {
            return;
        }
        self.language = language.to_string();
        self.dictionary = None;
        self.error = None;
        self.cache.borrow_mut().clear();
        let (sender, receiver) = mpsc::channel();
        let language = language.to_string();
        std::thread::spawn(move || {
            let result = match find(&language, &dictionary_dirs()) {
                Some((aff, dic)) => {
                    spellbook::Dictionary::new(&aff, &dic).map_err(|e| e.to_string())
                }
                None => Err(crate::i18n::t!(
                    "spelling-no-dictionary",
                    language = language.as_str()
                )),
            };
            let _ = sender.send(result);
        });
        self.loading = Some(receiver);
    }

    /// Take a dictionary loaded in the background. True when something changed.
    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.loading else {
            return false;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.loading = None;
                match result {
                    Ok(dictionary) => self.dictionary = Some(Arc::new(dictionary)),
                    Err(error) => self.error = Some(error),
                }
                self.cache.borrow_mut().clear();
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.loading = None;
                self.error = Some(crate::i18n::t!(
                    "spelling-load-failed",
                    language = self.language.as_str()
                ));
                true
            }
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading.is_some()
    }

    pub fn is_ready(&self) -> bool {
        self.dictionary.is_some()
    }

    /// Read the project's word list and accept the character names as words.
    pub fn set_project(&mut self, root: &Path, names: impl IntoIterator<Item = String>) {
        self.project_root = root.to_path_buf();
        self.project_words = std::fs::read_to_string(root.join(PROJECT_WORDS_FILE))
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim().to_lowercase())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        for name in names {
            for range in words(&name) {
                self.project_words.insert(name[range].to_lowercase());
            }
        }
        self.cache.borrow_mut().clear();
    }

    /// True when `word` is spelled correctly, or when there is no dictionary.
    pub fn is_correct(&self, word: &str) -> bool {
        let Some(dictionary) = &self.dictionary else {
            return true;
        };
        if let Some(&known) = self.cache.borrow().get(word) {
            return known;
        }
        let correct = self.check(dictionary, word);
        self.cache.borrow_mut().insert(word.to_string(), correct);
        correct
    }

    fn check(&self, dictionary: &spellbook::Dictionary, word: &str) -> bool {
        let known = |w: &str| {
            let lower = w.to_lowercase();
            dictionary.check(w)
                || self.project_words.contains(&lower)
                || self.ignored.contains(&lower)
        };
        if known(word) {
            return true;
        }
        let straight = word.replace('’', "'");
        if straight != word && known(&straight) {
            return true;
        }
        // Possessives of names the dictionary does not know: "Mara's".
        if let Some(stem) = straight
            .strip_suffix("'s")
            .filter(|_| self.language.starts_with("en"))
        {
            if known(stem) {
                return true;
            }
        }
        // Hyphenated compounds whose parts are all words.
        straight.contains('-')
            && straight
                .split('-')
                .all(|part| !part.is_empty() && known(part))
    }

    /// Up to six suggestions for a misspelled word.
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let Some(dictionary) = &self.dictionary else {
            return Vec::new();
        };
        let mut found = Vec::new();
        dictionary.suggest(word, &mut found);
        // Hunspell also proposes splits; keep only those made of real words.
        found.retain(|s| !s.contains(' ') || s.split(' ').all(|part| dictionary.check(part)));
        found.truncate(MAX_SUGGESTIONS);
        found
    }

    /// [`Speller::suggest`], remembering the last word so a menu shown every frame
    /// does not search again.
    pub fn suggestions(&self, word: &str) -> Vec<String> {
        if let Some((last, found)) = &*self.last_suggestions.borrow() {
            if last == word {
                return found.clone();
            }
        }
        let found = self.suggest(word);
        *self.last_suggestions.borrow_mut() = Some((word.to_string(), found.clone()));
        found
    }

    /// Accept `word` for this session only.
    pub fn ignore(&mut self, word: &str) {
        self.ignored.insert(word.to_lowercase());
        self.cache.borrow_mut().clear();
    }

    /// Add `word` to the project's `dictionary.txt`.
    pub fn add_to_project(&mut self, word: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = self.project_root.join(PROJECT_WORDS_FILE);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        if !existing.is_empty() && !existing.ends_with('\n') {
            writeln!(file)?;
        }
        writeln!(file, "{word}")?;
        self.project_words.insert(word.to_lowercase());
        self.cache.borrow_mut().clear();
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn bundled_speller() -> Speller {
    let mut speller = Speller {
        language: DEFAULT_LANGUAGE.into(),
        ..Default::default()
    };
    speller.dictionary = Some(Arc::new(
        spellbook::Dictionary::new(BUNDLED_AFF, BUNDLED_DIC).unwrap(),
    ));
    speller
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_keep_inner_apostrophes_and_hyphens_and_skip_numbers() {
        let text = "“Don't,” she said—rain-soaked, at 9pm. -dash- Mara’s 'quoted'";
        let found: Vec<&str> = words(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(
            found,
            [
                "Don't",
                "she",
                "said",
                "rain-soaked",
                "at",
                "dash",
                "Mara’s",
                "quoted"
            ]
        );
        assert_eq!(word_at("one café two", 6), Some((4..8, "café".into())));
        assert_eq!(word_at("one café two", 8), Some((4..8, "café".into())));
        assert_eq!(word_at("one  two", 4), None);
    }

    #[test]
    fn normalize_and_resolve_language() {
        assert_eq!(normalize("pt-br"), "pt_BR");
        assert_eq!(normalize(" en_us "), "en_US");
        assert_eq!(normalize("pt"), "pt");
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve_language("", dir.path()), "en_US");
        assert_eq!(resolve_language("es-es", dir.path()), "es_ES");
        std::fs::write(dir.path().join("metadata.yaml"), "title: X\nlang: pt-BR\n").unwrap();
        // The project's own language wins over the default.
        assert_eq!(resolve_language("", dir.path()), "pt_BR");
        assert_eq!(resolve_language("es_ES", dir.path()), "pt_BR");
    }

    #[test]
    fn setting_the_project_language_keeps_the_rest_of_metadata() {
        assert_eq!(with_language("", "pt-BR"), "lang: pt-BR\n");
        assert_eq!(
            with_language("# Book\ntitle: O Porto\nlang: en\nauthor: Ana", "pt-BR"),
            "# Book\ntitle: O Porto\nlang: pt-BR\nauthor: Ana\n"
        );
        assert_eq!(
            with_language("title: X\nlanguage : en\n", "es-ES"),
            "title: X\nlanguage: es-ES\n"
        );
        // Nested keys and similar names are not the book's language.
        assert_eq!(
            with_language("title: X\nlanguages: [en]\n...\n", "pt-BR"),
            "title: X\nlanguages: [en]\nlang: pt-BR\n...\n"
        );
        let dir = tempfile::tempdir().unwrap();
        set_project_language(dir.path(), "pt_BR").unwrap();
        assert_eq!(project_language(dir.path()).as_deref(), Some("pt_BR"));
        set_project_language(dir.path(), "en_GB").unwrap();
        let text = std::fs::read_to_string(dir.path().join("metadata.yaml")).unwrap();
        assert_eq!(text, "lang: en-GB\n");
    }

    #[test]
    fn finds_user_dictionaries_with_or_without_region_and_falls_back_to_bundled() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        assert!(find("pt_BR", &dirs).is_none());
        assert!(find("en_US", &dirs).is_some_and(|(aff, _)| aff == BUNDLED_AFF));
        std::fs::write(dir.path().join("pt_BR.aff"), "\u{feff}SET UTF-8\n").unwrap();
        std::fs::write(dir.path().join("pt_BR.dic"), "1\nação\n").unwrap();
        // A .dic without its .aff is ignored.
        std::fs::write(dir.path().join("xx_XX.dic"), "1\nword\n").unwrap();
        assert_eq!(languages_in(&dirs), ["en_US", "pt_BR"]);
        let (aff, dic) = find("pt", &dirs).unwrap();
        assert_eq!(aff, "SET UTF-8\n");
        let dictionary = spellbook::Dictionary::new(&aff, &dic).unwrap();
        assert!(dictionary.check("ação"));
    }

    #[test]
    fn checks_words_names_possessives_and_compounds() {
        let mut speller = bundled_speller();
        for word in [
            "harbor",
            "Harbor",
            "don't",
            "don’t",
            "rain-soaked",
            "well-lit",
        ] {
            assert!(speller.is_correct(word), "{word}");
        }
        for word in ["harbr", "recieve", "Quillanor", "Quillanor's"] {
            assert!(!speller.is_correct(word), "{word}");
        }
        let dir = tempfile::tempdir().unwrap();
        speller.set_project(dir.path(), ["Mara Quillanor".to_string()]);
        assert!(speller.is_correct("Quillanor"));
        assert!(speller.is_correct("Quillanor's"));
        assert!(speller.is_correct("QUILLANOR"));
        assert_eq!(speller.suggest("recieve")[0], "receive");
        assert!(speller.suggest("harbr").contains(&"harbor".to_string()));
    }

    #[test]
    fn project_words_are_plain_text_and_ignores_last_for_the_session() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PROJECT_WORDS_FILE), "# Names\nVell").unwrap();
        let mut speller = bundled_speller();
        speller.set_project(dir.path(), []);
        assert!(speller.is_correct("Vell"));
        assert!(!speller.is_correct("Ostrander"));
        speller.add_to_project("Ostrander").unwrap();
        assert!(speller.is_correct("Ostrander"));
        let text = std::fs::read_to_string(dir.path().join(PROJECT_WORDS_FILE)).unwrap();
        assert_eq!(text, "# Names\nVell\nOstrander\n");
        assert!(!speller.is_correct("zorbleglim"));
        speller.ignore("zorbleglim");
        assert!(speller.is_correct("zorbleglim"));
        // A new session (a fresh project load) forgets ignored words but not project words.
        let mut fresh = bundled_speller();
        fresh.set_project(dir.path(), []);
        assert!(fresh.is_correct("Ostrander"));
        assert!(!fresh.is_correct("zorbleglim"));
    }

    #[test]
    fn config_example_and_old_configs_load() {
        let example: crate::AppConfig =
            toml::from_str(include_str!("../docs/config.example.toml")).unwrap();
        assert!(example.spelling.enabled);
        assert!(example.spelling.language.is_empty());
        let old: crate::AppConfig = toml::from_str("[view]\nsidebar = false").unwrap();
        assert!(old.spelling.enabled);
        let set: crate::AppConfig =
            toml::from_str("[spelling]\nenabled = false\nlanguage = 'pt_BR'").unwrap();
        assert!(!set.spelling.enabled);
        assert_eq!(set.spelling.language, "pt_BR");
    }

    #[test]
    fn without_a_dictionary_nothing_is_flagged() {
        let speller = Speller::default();
        assert!(speller.is_correct("zorbleglim"));
        assert!(speller.suggest("zorbleglim").is_empty());
    }
}
