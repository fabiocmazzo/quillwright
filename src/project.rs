//! A project is an ordinary folder. Templates are optional Markdown conventions.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const FIRST_CHAPTER: &str = "chapters/001-opening.md";
const FILES: [(&str, &str); 4] = [
    (
        "AGENT.md",
        include_str!("../assets/project-template/AGENT.md"),
    ),
    (
        "world-rules.md",
        include_str!("../assets/project-template/world-rules.md"),
    ),
    (
        "characters/example-character.md",
        include_str!("../assets/project-template/example-character.md"),
    ),
    (
        FIRST_CHAPTER,
        include_str!("../assets/project-template/001-opening.md"),
    ),
];

pub struct NewProject {
    pub name: String,
    pub parent: PathBuf,
    pub error: Option<String>,
}

pub fn destination(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\', '\0']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Enter a folder name, without path separators.",
        ));
    }
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use a single folder name.",
        ));
    }
    if !parent.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Choose an existing parent folder.",
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

/// Create only a new directory; never merge into or overwrite an existing project.
pub fn create(path: &Path) -> io::Result<PathBuf> {
    fs::create_dir(path)?;
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let result = (|| {
        for name in ["chapters", "characters"] {
            let dir = path.join(name);
            fs::create_dir(&dir)?;
            dirs.push(dir);
        }
        for (name, content) in FILES {
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

    #[test]
    fn creates_only_the_recommended_markdown_scaffold() {
        let temp = tempfile::tempdir().unwrap();
        let path = destination(temp.path(), "My novel").unwrap();
        let root = create(&path).unwrap();
        assert!(root.join("chapters").is_dir());
        assert!(root.join("characters").is_dir());
        for (name, content) in FILES {
            assert_eq!(fs::read_to_string(root.join(name)).unwrap(), content);
        }
        assert_eq!(fs::read_dir(root).unwrap().count(), 4);
    }

    #[test]
    fn existing_projects_and_files_are_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let root = create(&temp.path().join("book")).unwrap();
        fs::write(root.join(FIRST_CHAPTER), "author edits").unwrap();
        assert_eq!(
            create(&root).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read_to_string(root.join(FIRST_CHAPTER)).unwrap(),
            "author edits"
        );
        let file = temp.path().join("file");
        fs::write(&file, "keep").unwrap();
        assert!(create(&file).is_err());
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
