//! Reference material: research files shown read only beside the manuscript.
//! Loading happens on a background thread; nothing here ever writes to a file the
//! author already has (adding research files only creates new ones).
pub mod pdf;
pub mod text;

use eframe::egui::ColorImage;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc,
    time::SystemTime,
};

/// Text files larger than this are cut, with a note at the end.
const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;
/// Images are scaled down so neither side exceeds this (GPU texture limits).
const MAX_IMAGE_SIDE: u32 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Markdown,
    Image,
    Pdf,
    Other,
}

impl Kind {
    pub fn of(path: &Path) -> Kind {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "txt" | "text" => Kind::Text,
            "md" | "markdown" => Kind::Markdown,
            "png" | "jpg" | "jpeg" | "gif" | "webp" => Kind::Image,
            "pdf" => Kind::Pdf,
            _ => Kind::Other,
        }
    }

    /// A short label for the sidebar.
    pub fn label(self) -> String {
        crate::i18n::t!(match self {
            Kind::Text => "file-text",
            Kind::Markdown => "file-notes",
            Kind::Image => "file-image",
            Kind::Pdf => "file-pdf",
            Kind::Other => "file-other",
        })
    }

    /// Text and Markdown can also be opened in the editor.
    pub fn editable(self) -> bool {
        matches!(self, Kind::Text | Kind::Markdown)
    }

    /// Kinds that open in the Reference panel even from All Files.
    pub fn viewer_only(self) -> bool {
        matches!(self, Kind::Image | Kind::Pdf)
    }
}

pub enum Content {
    Text(String),
    Markdown(String),
    Image(ColorImage),
    Pdf(pdf::Document),
    /// The file can't be shown here; the message says why.
    Unsupported(String),
}

/// Size and modification time, to notice when a reference changes on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

pub fn stamp(path: &Path) -> Option<Stamp> {
    let meta = fs::metadata(path).ok()?;
    Some(Stamp {
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

pub struct Loaded {
    pub path: PathBuf,
    pub stamp: Option<Stamp>,
    pub content: Result<Content, String>,
}

/// Start loading `path` on a background thread.
pub fn load_in_background(path: PathBuf) -> mpsc::Receiver<Loaded> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stamp = stamp(&path);
        let content = load(&path);
        let _ = tx.send(Loaded {
            path,
            stamp,
            content,
        });
    });
    rx
}

pub fn load(path: &Path) -> Result<Content, String> {
    match Kind::of(path) {
        Kind::Text => read_text(path).map(Content::Text),
        Kind::Markdown => read_text(path).map(Content::Markdown),
        Kind::Image => load_image(path).map(Content::Image),
        Kind::Pdf => {
            pdf::Document::open(path, text::default_cache_dir().as_deref()).map(Content::Pdf)
        }
        Kind::Other => Ok(Content::Unsupported(crate::i18n::t!(
            "reference-unsupported"
        ))),
    }
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let cut = bytes.len() > MAX_TEXT_BYTES;
    let bytes = &bytes[..bytes.len().min(MAX_TEXT_BYTES)];
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if cut {
        text.push_str(&format!("\n\n[… {}]", crate::i18n::t!("reference-cut")));
    }
    Ok(text)
}

fn load_image(path: &Path) -> Result<ColorImage, String> {
    let image = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    let image = if image.width().max(image.height()) > MAX_IMAGE_SIDE {
        image.resize(
            MAX_IMAGE_SIDE,
            MAX_IMAGE_SIDE,
            image::imageops::FilterType::Triangle,
        )
    } else {
        image
    };
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Copy `files` into `dir` (created if needed). Never overwrites: a name that is taken
/// gets a number ("map (2).png"). Returns the new paths.
pub fn add_files(dir: &Path, files: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut added = Vec::new();
    for file in files {
        let Some(name) = file.file_name() else {
            continue;
        };
        let mut source = fs::File::open(file)?;
        let (mut target, path) = create_unique(dir, Path::new(name))?;
        io::copy(&mut source, &mut target)?;
        added.push(path);
    }
    Ok(added)
}

fn create_unique(dir: &Path, name: &Path) -> io::Result<(fs::File, PathBuf)> {
    let stem = name
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = name.extension().map(|e| e.to_string_lossy().into_owned());
    for n in 1.. {
        let candidate = match (n, &ext) {
            (1, _) => name.to_path_buf(),
            (_, Some(ext)) => PathBuf::from(format!("{stem} ({n}).{ext}")),
            (_, None) => PathBuf::from(format!("{stem} ({n})")),
        };
        let path = dir.join(candidate);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((file, path)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_the_extension() {
        assert_eq!(Kind::of(Path::new("a/notes.TXT")), Kind::Text);
        assert_eq!(Kind::of(Path::new("a/plan.md")), Kind::Markdown);
        assert_eq!(Kind::of(Path::new("map.JPEG")), Kind::Image);
        assert_eq!(Kind::of(Path::new("book.pdf")), Kind::Pdf);
        assert_eq!(Kind::of(Path::new("song.mp3")), Kind::Other);
        assert_eq!(Kind::of(Path::new("README")), Kind::Other);
    }

    #[test]
    fn loads_text_without_bom_and_images_as_pixels() {
        let temp = tempfile::tempdir().unwrap();
        let text = temp.path().join("clip.txt");
        fs::write(&text, b"\xEF\xBB\xBFCora\xC3\xA7\xC3\xA3o").unwrap();
        match load(&text).unwrap() {
            Content::Text(t) => assert_eq!(t, "Coração"),
            _ => panic!("expected text"),
        }

        let png = temp.path().join("dot.png");
        image::RgbaImage::from_pixel(3, 2, image::Rgba([200, 10, 10, 255]))
            .save(&png)
            .unwrap();
        match load(&png).unwrap() {
            Content::Image(img) => assert_eq!(img.size, [3, 2]),
            _ => panic!("expected an image"),
        }

        let broken = temp.path().join("broken.png");
        fs::write(&broken, b"not an image").unwrap();
        assert!(load(&broken).is_err());
    }

    #[test]
    fn adding_files_never_overwrites() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("map.png");
        fs::write(&outside, "new").unwrap();
        let research = temp.path().join("book/research");
        fs::create_dir_all(&research).unwrap();
        fs::write(research.join("map.png"), "old").unwrap();

        let added = add_files(&research, &[outside.clone(), outside]).unwrap();
        assert_eq!(fs::read_to_string(research.join("map.png")).unwrap(), "old");
        assert_eq!(
            added,
            vec![research.join("map (2).png"), research.join("map (3).png")]
        );
        assert_eq!(fs::read_to_string(&added[0]).unwrap(), "new");
    }
}
