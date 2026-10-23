//! A project is an ordinary folder. Templates are optional Markdown conventions, one per
//! project kind (`crate::kinds`).
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::i18n::t;
use crate::kinds::Kind;

pub struct NewProject {
    pub name: String,
    pub parent: PathBuf,
    /// The kind and language chosen.
    pub kind: &'static Kind,
    pub error: Option<String>,
}

pub fn destination(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\', '\0']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            t!("project-name-invalid"),
        ));
    }
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            t!("project-name-single"),
        ));
    }
    if !parent.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            t!("project-parent-missing"),
        ));
    }
    Ok(parent.join(name))
}

/// Validate an existing folder without requiring or creating a scaffold.
pub fn open(path: &Path) -> io::Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    fs::read_dir(&path)?;
    Ok(path)
}

/// Create only a new directory with the kind's template; never merge into or overwrite
/// an existing project.
pub fn create(path: &Path, kind: &Kind) -> io::Result<PathBuf> {
    fs::create_dir(path)?;
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let result = (|| {
        let folders = [&kind.documents.folder, &kind.sheets.folder];
        let parents = kind.files.iter().filter_map(|(name, _)| {
            Path::new(name)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .and_then(Path::to_str)
        });
        for name in folders.into_iter().map(String::as_str).chain(parents) {
            let dir = path.join(name);
            if name.is_empty() || dir.is_dir() {
                continue;
            }
            fs::create_dir_all(&dir)?;
            dirs.push(dir);
        }
        for (name, content) in &kind.files {
            let file = path.join(name);
            let mut handle = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file)?;
            files.push(file);
            handle.write_all(content.as_bytes())?;
        }
        open(path)
    })();
    if result.is_err() {
        // Roll back our own entries only. Never recursively remove a directory
        // that could have received unrelated files while creation was in progress.
        for file in files.iter().rev() {
            let _ = fs::remove_file(file);
        }
        for dir in dirs.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
        let _ = fs::remove_dir(path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds;

    const FIRST_CHAPTER: &str = "chapters/001-opening.md";

    #[test]
    fn creates_only_the_recommended_markdown_scaffold() {
        let temp = tempfile::tempdir().unwrap();
        let path = destination(temp.path(), "My novel").unwrap();
        let novel = kinds::default_kind();
        let root = create(&path, novel).unwrap();
        assert!(root.join("chapters").is_dir());
        assert!(root.join("characters").is_dir());
        assert_eq!(novel.first_file, FIRST_CHAPTER);
        assert_eq!(novel.files.len(), 4);
        for (name, content) in &novel.files {
            assert_eq!(&fs::read_to_string(root.join(name)).unwrap(), content);
        }
        assert_eq!(fs::read_dir(root).unwrap().count(), 4);
    }

    #[test]
    fn existing_projects_and_files_are_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let novel = kinds::default_kind();
        let root = create(&temp.path().join("book"), novel).unwrap();
        fs::write(root.join(FIRST_CHAPTER), "author edits").unwrap();
        assert_eq!(
            create(&root, novel).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read_to_string(root.join(FIRST_CHAPTER)).unwrap(),
            "author edits"
        );
        let file = temp.path().join("file");
        fs::write(&file, "keep").unwrap();
        assert!(create(&file, novel).is_err());
        assert_eq!(fs::read_to_string(file).unwrap(), "keep");
    }

    #[test]
    fn opens_arbitrary_folders_without_changing_them() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("manuscript.md"), "text").unwrap();
        assert_eq!(
            open(temp.path()).unwrap(),
            fs::canonicalize(temp.path()).unwrap()
        );
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        assert!(open(&temp.path().join("missing")).is_err());
        assert!(open(&temp.path().join("manuscript.md")).is_err());
    }

    #[test]
    fn folder_names_cannot_escape_the_selected_parent() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "",
            " ",
            ".",
            "..",
            "../other",
            "/absolute",
            "a/b",
            "a\\b",
            "a\0b",
        ] {
            assert!(destination(temp.path(), name).is_err(), "{name:?}");
        }
        assert_eq!(
            destination(temp.path(), " Unpublished café ").unwrap(),
            temp.path().join("Unpublished café")
        );
    }
}
