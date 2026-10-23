//! The interface language. Texts are Fluent messages: the app's in
//! `assets/locales/<lang>/quillwright.ftl`, and each project kind's (its name, section
//! labels, assistant actions) in `assets/kinds/<id>/locales/<lang>.ftl`. English is the
//! source; a message missing in another language falls back to English, then to its key.
//!
//! The language is the config's `language`, or the system's when that is empty. It is
//! the language of the app only: a book's own language (`metadata.yaml`) decides its
//! spelling dictionary and the words in exported files.
use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    OnceLock,
};

/// Look up a message in the interface language: `t!("key")`, or with arguments,
/// `t!("key", count = n, name = "…")`. Numbers choose plural forms; text is inserted
/// as is.
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::text($key, None)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = fluent_bundle::FluentArgs::new();
        $(args.set(stringify!($name), $value);)+
        $crate::i18n::text($key, Some(&args))
    }};
}
pub(crate) use t;

pub struct Language {
    /// The folder in `assets/locales/` and the config value, such as `pt-BR`.
    pub id: &'static str,
    /// The language's name in that language, for the menu.
    pub name: &'static str,
    /// Digit group separator for word counts ("12,345" or "12.345").
    pub group_separator: char,
    app: &'static str,
}

/// The interface languages, English first.
pub const LANGUAGES: &[Language] = &[
    Language {
        id: "en",
        name: "English",
        group_separator: ',',
        app: include_str!("../assets/locales/en/quillwright.ftl"),
    },
    Language {
        id: "pt-BR",
        name: "Português (Brasil)",
        group_separator: '.',
        app: include_str!("../assets/locales/pt-BR/quillwright.ftl"),
    },
    Language {
        id: "es",
        name: "Español",
        group_separator: '.',
        app: include_str!("../assets/locales/es/quillwright.ftl"),
    },
    Language {
        id: "zh-CN",
        name: "简体中文",
        group_separator: ',',
        app: include_str!("../assets/locales/zh-CN/quillwright.ftl"),
    },
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// One bundle per entry of [`LANGUAGES`].
fn bundles() -> &'static [FluentBundle<FluentResource>] {
    static BUNDLES: OnceLock<Vec<FluentBundle<FluentResource>>> = OnceLock::new();
    BUNDLES.get_or_init(|| {
        LANGUAGES
            .iter()
            .map(|language| {
                // The built-in files are checked by the tests, so errors cannot happen here.
                build(language.id, &sources(language)).0
            })
            .collect()
    })
}

/// The Fluent files of `language`: the app's, then every kind's.
fn sources(language: &Language) -> Vec<(String, &'static str)> {
    let mut files = vec![(format!("{}/quillwright.ftl", language.id), language.app)];
    let suffix = format!("/locales/{}.ftl", language.id);
    for (path, text) in crate::kinds::KIND_FILES {
        if path.ends_with(&suffix) {
            files.push((path.to_string(), *text));
        }
    }
    files
}

/// A bundle from `files`, and the errors found in them (what could be read is kept).
fn build(id: &str, files: &[(String, &str)]) -> (FluentBundle<FluentResource>, Vec<String>) {
    let langid = id.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    // Without this, inserted values are wrapped in Unicode isolation marks, which the
    // fonts draw as boxes.
    bundle.set_use_isolating(false);
    let mut errors = Vec::new();
    for (name, text) in files {
        let resource = match FluentResource::try_new(text.to_string()) {
            Ok(resource) => resource,
            Err((resource, errs)) => {
                errors.extend(errs.iter().map(|e| format!("{name}: {e:?}")));
                resource
            }
        };
        if let Err(errs) = bundle.add_resource(resource) {
            errors.extend(errs.iter().map(|e| format!("{name}: {e}")));
        }
    }
    (bundle, errors)
}

/// Sets the interface language: an id from [`LANGUAGES`], or "" for the system's.
pub fn set_language(id: &str) {
    let chosen = if id.is_empty() {
        sys_locale::get_locale().map_or(0, |locale| negotiate(&locale))
    } else {
        LANGUAGES.iter().position(|l| l.id == id).unwrap_or(0)
    };
    CURRENT.store(chosen, Ordering::Relaxed);
}

pub fn current() -> &'static Language {
    &LANGUAGES[CURRENT.load(Ordering::Relaxed)]
}

/// The best interface language for a locale such as `pt-BR`, `pt_PT.UTF-8`, or
/// `zh-Hans-CN`: the same tag, else the same language, else English.
fn negotiate(locale: &str) -> usize {
    let tag = locale
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('_', "-");
    if let Some(i) = LANGUAGES
        .iter()
        .position(|l| l.id.eq_ignore_ascii_case(&tag))
    {
        return i;
    }
    let primary = tag.split('-').next().unwrap_or_default();
    LANGUAGES
        .iter()
        .position(|l| {
            l.id.split('-')
                .next()
                .is_some_and(|p| p.eq_ignore_ascii_case(primary))
        })
        .unwrap_or(0)
}

fn format(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    attribute: Option<&str>,
    args: Option<&FluentArgs>,
) -> Option<String> {
    let message = bundle.get_message(key)?;
    let pattern = match attribute {
        Some(name) => message.get_attribute(name)?.value(),
        None => message.value()?,
    };
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned(),
    )
}

/// The message `key`, or the attribute `key.name`, in the interface language (see
/// [`t!`]).
pub fn text(key: &str, args: Option<&FluentArgs>) -> String {
    let found = match key.split_once('.') {
        Some((message, attribute)) => lookup(message, Some(attribute), args),
        None => lookup(key, None, args),
    };
    found.unwrap_or_else(|| key.to_string())
}

/// The message or attribute (`.hint`, `.gender`) `key`, in the interface language or else
/// English; `None` when neither has it.
pub fn lookup(key: &str, attribute: Option<&str>, args: Option<&FluentArgs>) -> Option<String> {
    let bundles = bundles();
    format(
        &bundles[CURRENT.load(Ordering::Relaxed)],
        key,
        attribute,
        args,
    )
    .or_else(|| format(&bundles[0], key, attribute, args))
}

/// `n` with digit groups, as the interface language writes them: "12,345" or "12.345".
pub fn group(n: usize) -> String {
    let digits = n.to_string();
    let separator = current().group_separator;
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::FluentValue;

    /// Every message (and attribute) id in `bundle`'s files.
    fn ids(language: &Language) -> Vec<String> {
        use fluent_syntax::ast::Entry;
        let mut ids = Vec::new();
        for (_, text) in sources(language) {
            let resource = FluentResource::try_new(text.to_string()).unwrap();
            for entry in resource.entries() {
                if let Entry::Message(message) = entry {
                    let id = message.id.name;
                    if message.value.is_some() {
                        ids.push(id.to_string());
                    }
                    for attribute in &message.attributes {
                        ids.push(format!("{id}.{}", attribute.id.name));
                    }
                }
            }
        }
        ids.sort();
        ids
    }

    #[test]
    fn every_language_parses_and_has_every_message() {
        let english = ids(&LANGUAGES[0]);
        assert!(english.len() > 100);
        for language in LANGUAGES {
            let (_, errors) = build(language.id, &sources(language));
            assert!(errors.is_empty(), "{}: {errors:#?}", language.id);
            let have = ids(language);
            let missing: Vec<_> = english.iter().filter(|id| !have.contains(id)).collect();
            assert!(missing.is_empty(), "{} lacks {missing:?}", language.id);
            let extra: Vec<_> = have.iter().filter(|id| !english.contains(id)).collect();
            assert!(extra.is_empty(), "{} has unknown {extra:?}", language.id);
        }
    }

    #[test]
    fn every_message_formats_with_typical_arguments() {
        // Catches references to missing messages and misspelled variables, which only
        // show at format time.
        // Every variable the English messages use; the app passes these.
        let mut variables: Vec<String> = Vec::new();
        for (_, text) in sources(&LANGUAGES[0]) {
            for part in text.split('$').skip(1) {
                let name: String = part
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                if !name.is_empty() && !variables.contains(&name) {
                    variables.push(name);
                }
            }
        }
        for (i, language) in LANGUAGES.iter().enumerate() {
            let bundle = &bundles()[i];
            for id in ids(language) {
                let (key, attribute) = match id.split_once('.') {
                    Some((key, attribute)) => (key.to_string(), Some(attribute.to_string())),
                    None => (id.clone(), None),
                };
                let message = bundle.get_message(&key).unwrap();
                let pattern = match &attribute {
                    Some(name) => message.get_attribute(name).unwrap().value(),
                    None => message.value().unwrap(),
                };
                for count in [1, 2, 5] {
                    let mut args = FluentArgs::new();
                    for name in &variables {
                        let value = match name.as_str() {
                            // Numbers, which choose plural forms.
                            "count" | "files" | "line" | "page" | "first" | "last" | "current"
                            | "percent" | "minutes" => FluentValue::from(count),
                            _ => FluentValue::from("x"),
                        };
                        args.set(name.clone(), value);
                    }
                    let mut errors = Vec::new();
                    bundle.format_pattern(pattern, Some(&args), &mut errors);
                    assert!(errors.is_empty(), "{} {id}: {errors:?}", language.id);
                }
            }
        }
    }

    #[test]
    fn negotiates_system_locales() {
        let id = |locale| LANGUAGES[negotiate(locale)].id;
        assert_eq!(id("pt-BR"), "pt-BR");
        assert_eq!(id("pt_PT.UTF-8"), "pt-BR");
        assert_eq!(id("es-MX"), "es");
        assert_eq!(id("zh-Hans-CN"), "zh-CN");
        assert_eq!(id("zh_TW"), "zh-CN");
        assert_eq!(id("en-GB"), "en");
        assert_eq!(id("de-DE"), "en");
        assert_eq!(id("C"), "en");
    }

    #[test]
    fn plural_forms_follow_the_language() {
        let words = |i: usize, n: usize| {
            let mut args = FluentArgs::new();
            args.set("count", n);
            args.set("words", n.to_string());
            format(&bundles()[i], "words", None, Some(&args)).unwrap()
        };
        assert_eq!(words(0, 1), "1 word");
        assert_eq!(words(0, 3), "3 words");
        assert_eq!(words(1, 1), "1 palavra");
        assert_eq!(words(1, 3), "3 palavras");
        assert_eq!(words(3, 3), "3 字");
    }
}
