//! Local version history: timestamped copies of each file, kept outside the project.
//!
//! Copies live in the user's data directory (for example
//! `~/.local/share/quillwright/history/`), so agents and Git never see them.
//! Each file gets its own folder, named after the file plus a hash of its full path.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How many copies to keep per file.
pub const KEEP: usize = 50;
/// Saves closer together than this share one copy.
pub const INTERVAL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub time: SystemTime,
}

impl Snapshot {
    pub fn read(&self) -> io::Result<String> {
        fs::read_to_string(&self.path)
    }
}

/// Where history is stored on this machine.
pub fn default_root() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "quillwright", "quillwright")
        .map(|dirs| dirs.data_dir().join("history"))
}

/// The history folder for one file.
pub fn folder(root: &Path, file: &Path) -> PathBuf {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let safe: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    root.join(format!(
        "{safe}-{:016x}",
        fnv1a(file.to_string_lossy().as_bytes())
    ))
}

/// Copies of `file`, newest first.
pub fn list(root: &Path, file: &Path) -> Vec<Snapshot> {
    let Ok(entries) = fs::read_dir(folder(root, file)) else {
        return Vec::new();
    };
    let mut snapshots: Vec<Snapshot> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "md" {
                return None;
            }
            let millis: u64 = path.file_stem()?.to_str()?.parse().ok()?;
            Some(Snapshot {
                time: UNIX_EPOCH + Duration::from_millis(millis),
                path,
            })
        })
        .collect();
    snapshots.sort_by_key(|s| std::cmp::Reverse(s.time));
    snapshots
}

/// Store a copy of `text` for `file`.
///
/// Skips empty text and text identical to the newest copy. Unless `force` is set,
/// also skips when the newest copy is more recent than [`INTERVAL`].
/// Returns whether a copy was written.
pub fn record(
    root: &Path,
    file: &Path,
    text: &str,
    force: bool,
    now: SystemTime,
) -> io::Result<bool> {
    if text.trim().is_empty() {
        return Ok(false);
    }
    let existing = list(root, file);
    if let Some(latest) = existing.first() {
        let age = now.duration_since(latest.time).unwrap_or_default();
        if !force && age < INTERVAL {
            return Ok(false);
        }
        if latest.read().is_ok_and(|old| old == text) {
            return Ok(false);
        }
    }

    let dir = folder(root, file);
    fs::create_dir_all(&dir)?;
    // Remember which file this folder belongs to, for anyone browsing it by hand.
    let source = dir.join("source.txt");
    if !source.exists() {
        fs::write(&source, format!("{}\n", file.display()))?;
    }
    let mut millis = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    // Never reuse a name, even for two copies in the same millisecond.
    if let Some(latest) = existing.first() {
        let latest_millis = latest
            .time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        millis = millis.max(latest_millis + 1);
    }
    let target = dir.join(format!("{millis}.md"));
    let temp = dir.join(format!(".{millis}.tmp"));
    fs::write(&temp, text)?;
    fs::rename(&temp, &target)?;

    for old in list(root, file).into_iter().skip(KEEP) {
        let _ = fs::remove_file(old.path);
    }
    Ok(true)
}

/// When a copy was made, in local time: "Today 14:32", "Yesterday 09:05", "Mon 12 Oct 18:40".
pub fn time_label(time: SystemTime, now: SystemTime) -> String {
    use chrono::{DateTime, Local};
    let local = |t: SystemTime| DateTime::<Local>::from(t).naive_local();
    when(local(time), local(now))
}

fn when(time: chrono::NaiveDateTime, now: chrono::NaiveDateTime) -> String {
    use chrono::Datelike;
    let days = (now.date() - time.date()).num_days();
    let clock = time.format("%H:%M");
    match days {
        0 => format!("Today {clock}"),
        1 => format!("Yesterday {clock}"),
        _ if time.year() == now.year() => time.format("%a %-d %b %H:%M").to_string(),
        _ => time.format("%-d %b %Y %H:%M").to_string(),
    }
}

/// Stable across Rust versions, unlike `DefaultHasher`.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000 + secs)
    }

    #[test]
    fn records_skips_duplicates_and_respects_interval() {
        let root = tempfile::tempdir().unwrap();
        let file = Path::new("/novel/chapters/001-opening.md");

        assert!(record(root.path(), file, "one", false, at(0)).unwrap());
        // Same text: never stored twice.
        assert!(!record(root.path(), file, "one", true, at(5)).unwrap());
        // Different text, but too soon.
        assert!(!record(root.path(), file, "two", false, at(60)).unwrap());
        // Forced copies ignore the interval.
        assert!(record(root.path(), file, "two", true, at(61)).unwrap());
        // After the interval, a normal save is stored.
        assert!(record(root.path(), file, "three", false, at(61 + 601)).unwrap());
        // Empty text is not worth keeping.
        assert!(!record(root.path(), file, "  \n", true, at(2_000)).unwrap());

        let texts: Vec<_> = list(root.path(), file)
            .iter()
            .map(|s| s.read().unwrap())
            .collect();
        assert_eq!(texts, ["three", "two", "one"]);
        assert!(folder(root.path(), file).join("source.txt").exists());
    }

    #[test]
    fn keeps_only_the_newest_copies() {
        let root = tempfile::tempdir().unwrap();
        let file = Path::new("/novel/chapters/002.md");
        for i in 0..KEEP + 5 {
            record(root.path(), file, &format!("v{i}"), true, at(i as u64)).unwrap();
        }
        let snapshots = list(root.path(), file);
        assert_eq!(snapshots.len(), KEEP);
        assert_eq!(snapshots[0].read().unwrap(), format!("v{}", KEEP + 4));
    }

    #[test]
    fn same_millisecond_does_not_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let file = Path::new("/novel/a.md");
        record(root.path(), file, "a", true, at(0)).unwrap();
        record(root.path(), file, "b", true, at(0)).unwrap();
        assert_eq!(list(root.path(), file).len(), 2);
    }

    #[test]
    fn files_with_the_same_name_are_kept_apart() {
        let root = tempfile::tempdir().unwrap();
        let a = Path::new("/one/notes.md");
        let b = Path::new("/two/notes.md");
        assert_ne!(folder(root.path(), a), folder(root.path(), b));
    }

    #[test]
    fn time_labels() {
        let t = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        let now = t("2026-10-19 16:00");
        assert_eq!(when(t("2026-10-19 09:05"), now), "Today 09:05");
        assert_eq!(when(t("2026-10-18 23:59"), now), "Yesterday 23:59");
        assert_eq!(when(t("2026-10-12 18:40"), now), "Mon 12 Oct 18:40");
        assert_eq!(when(t("2025-12-31 08:00"), now), "31 Dec 2025 08:00");
    }
}
