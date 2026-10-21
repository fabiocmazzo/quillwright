// Embeds the application icon in the Windows executable, and lists the files of the
// built-in project kinds (assets/kinds) so `src/kinds.rs` can embed them.
use std::{
    fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/quillwright.ico");
    embed_kinds();
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon/quillwright.ico");
        resource
            .compile()
            .expect("failed to embed the Windows icon");
    }
}

/// Write `$OUT_DIR/kind_files.rs`: `(path relative to assets/kinds, contents)` for every
/// file, sorted, each embedded with `include_str!`.
fn embed_kinds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/kinds");
    // A directory makes Cargo rerun this script when anything inside it changes.
    println!("cargo:rerun-if-changed=assets/kinds");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    let mut out = String::from("pub const KIND_FILES: &[(&str, &str)] = &[\n");
    for file in files {
        let relative = file.strip_prefix(&root).unwrap();
        let name = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        out.push_str(&format!(
            "    ({name:?}, include_str!({:?})),\n",
            file.display().to_string()
        ));
    }
    out.push_str("];\n");
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("kind_files.rs");
    fs::write(dest, out).expect("failed to write the kind file list");
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}
