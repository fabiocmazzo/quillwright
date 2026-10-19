//! Filesystem events are hints: reconcile against fresh disk contents off the UI thread.
use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecursiveMode, Watcher,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

const DEBOUNCE: Duration = Duration::from_millis(150);
const POLL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disk {
    Text(String),
    Missing,
    Unreadable(String),
}

pub fn read(path: &Path) -> Disk {
    match fs::read_to_string(path) {
        Ok(text) => Disk::Text(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Disk::Missing,
        Err(err) => Disk::Unreadable(err.to_string()),
    }
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Unchanged,
    Accept,
    Reload,
    Conflict,
}

pub fn reconcile(base: &str, local: &str, disk: &Disk, composing: bool) -> Decision {
    match disk {
        Disk::Text(text) if text == local => Decision::Accept,
        Disk::Text(text) if text == base => Decision::Unchanged,
        Disk::Text(_) if local == base && !composing => Decision::Reload,
        _ => Decision::Conflict,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub generation: u64,
    pub root: PathBuf,
    pub file: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub target: Target,
    pub root: PathBuf,
    pub file: Option<PathBuf>,
    pub disk: Option<Disk>,
    pub warning: Option<String>,
}

enum Message {
    Configure(Target),
    Check,
    Event(notify::Result<Event>),
    Stop,
}

pub struct Monitor {
    tx: Sender<Message>,
    pub updates: Receiver<Update>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Monitor {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let (updates_tx, updates) = mpsc::channel();
        let events_tx = tx.clone();
        let worker = thread::spawn(move || run(rx, events_tx, updates_tx, ctx));
        Self {
            tx,
            updates,
            worker: Some(worker),
        }
    }
    pub fn configure(&self, target: Target) {
        let _ = self.tx.send(Message::Configure(target));
    }
    pub fn check(&self) {
        let _ = self.tx.send(Message::Check);
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn relocated(path: &Path, renames: &[(PathBuf, PathBuf)]) -> PathBuf {
    // Prefer the original name after atomic replacement (including backup-rename saves).
    if path.exists() {
        return path.to_owned();
    }
    let mut result = path.to_owned();
    for (from, to) in renames {
        if let Ok(suffix) = result.strip_prefix(from) {
            result = to.join(suffix);
        }
    }
    if result.exists() {
        result
    } else {
        path.to_owned()
    }
}

fn run(rx: Receiver<Message>, tx: Sender<Message>, output: Sender<Update>, ctx: egui::Context) {
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(Message::Event(event));
    });
    let mut watched: Vec<PathBuf> = Vec::new();
    let mut target: Option<Target> = None;
    let mut deadline = Instant::now() + POLL;
    let mut burst_start = None;
    let mut renames = Vec::new();
    let mut warning = watcher.as_ref().err().map(ToString::to_string);
    let mut last = None;
    let mut tree_changed = false;
    loop {
        let message = rx.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        match message {
            Ok(Message::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(Message::Configure(next)) => {
                // Drop pending events from the old project/document generation.
                renames.clear();
                last = None;
                burst_start = None;
                if let Ok(watcher) = &mut watcher {
                    for path in watched.drain(..) {
                        let _ = watcher.unwatch(&path);
                    }
                    warning = None;
                    let mut paths = vec![(next.root.clone(), RecursiveMode::Recursive)];
                    if let Some(parent) = next.root.parent() {
                        paths.push((parent.to_owned(), RecursiveMode::NonRecursive));
                    }
                    if let Some(parent) = next.file.as_deref().and_then(Path::parent) {
                        if !parent.starts_with(&next.root) {
                            paths.push((parent.to_owned(), RecursiveMode::NonRecursive));
                        }
                    }
                    for (path, mode) in paths {
                        if watched.contains(&path) {
                            continue;
                        }
                        match watcher.watch(&path, mode) {
                            Ok(()) => watched.push(path),
                            Err(err) => warning = Some(err.to_string()),
                        }
                    }
                }
                target = Some(next);
                tree_changed = true;
                deadline = Instant::now() + DEBOUNCE;
            }
            Ok(Message::Check) => {
                tree_changed = true;
                deadline = Instant::now();
            }
            Ok(Message::Event(Ok(event))) => {
                if matches!(event.kind, EventKind::Access(_)) {
                    continue;
                }
                if let Some(t) = &target {
                    if !event.paths.is_empty()
                        && !event.paths.iter().any(|p| {
                            p.starts_with(&t.root)
                                || t.root.starts_with(p)
                                || t.file.as_ref().is_some_and(|file| p == file)
                        })
                    {
                        continue;
                    }
                }
                if matches!(
                    event.kind,
                    EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                ) && event.paths.len() == 2
                {
                    renames.push((event.paths[0].clone(), event.paths[1].clone()));
                }
                let now = Instant::now();
                let start = *burst_start.get_or_insert(now);
                deadline = (now + DEBOUNCE).min(start + Duration::from_millis(750));
                tree_changed = true;
            }
            Ok(Message::Event(Err(err))) => {
                warning = Some(err.to_string());
                deadline = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(t) = &target {
                    let root = relocated(&t.root, &renames);
                    let file = t.file.as_ref().map(|p| relocated(p, &renames));
                    let disk = file.as_deref().map(read);
                    // Wait for two matching reads; don't display a partially written UTF-8 file.
                    if let Some(path) = &file {
                        thread::sleep(Duration::from_millis(40));
                        if disk != Some(read(path)) {
                            deadline = Instant::now() + DEBOUNCE;
                            continue;
                        }
                    }
                    let update = Update {
                        target: t.clone(),
                        root,
                        file,
                        disk,
                        warning: warning.clone(),
                    };
                    // Fallback polling also wakes the tree if native watching failed.
                    if tree_changed || warning.is_some() || last.as_ref() != Some(&update) {
                        if output.send(update.clone()).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                    last = Some(update);
                }
                renames.clear();
                burst_start = None;
                tree_changed = false;
                deadline = Instant::now() + POLL;
            }
        }
    }
}

/// Save through a temporary file in the same directory, so agents see a complete file.
/// `expected` is checked again just before replacement. This is not an OS-wide lock.
pub fn write_checked(path: &Path, text: &str, expected: Option<&Disk>) -> Result<(), Disk> {
    use io::Write;
    let result = (|| -> io::Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))?;
        file.write_all(text.as_bytes())?;
        if let Ok(metadata) = fs::metadata(path) {
            file.as_file().set_permissions(metadata.permissions())?;
        }
        file.as_file().sync_all()?;
        if expected.is_some_and(|previous| &read(path) != previous) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "File changed while saving",
            ));
        }
        file.persist(path).map_err(|err| err.error)?;
        Ok(())
    })();
    result.map_err(|err| {
        if err.kind() == io::ErrorKind::Interrupted {
            read(path)
        } else {
            Disk::Unreadable(err.to_string())
        }
    })
}

/// Map a character index in `old` to the matching position in `new` after an
/// external reload. Unchanged lines keep their column; a cursor inside a changed
/// block moves to the start of the replacement block.
pub fn map_cursor(old: &str, new: &str, index: usize) -> usize {
    fn line_starts(text: &str) -> Vec<usize> {
        let mut starts = vec![0];
        let mut count = 0;
        for line in text.split_inclusive('\n') {
            count += line.chars().count();
            starts.push(count);
        }
        starts
    }
    if old.is_empty() {
        return 0; // A newly filled document starts reading from the top.
    }
    let old_starts = line_starts(old);
    let new_starts = line_starts(new);
    let new_len = *new_starts.last().unwrap_or(&0);
    let diff = similar::TextDiff::configure()
        .timeout(Duration::from_millis(50))
        .diff_lines(old, new);
    for op in diff.ops() {
        let (tag, old_lines, new_lines) = op.as_tag_tuple();
        let old_start = old_starts[old_lines.start];
        let old_end = old_starts[old_lines.end];
        if index < old_start || index >= old_end {
            continue;
        }
        let new_start = new_starts[new_lines.start];
        return match tag {
            similar::DiffTag::Equal => new_start + (index - old_start),
            _ => new_start,
        }
        .min(new_len);
    }
    // Cursor at the very end of the old text.
    if index >= *old_starts.last().unwrap_or(&0) {
        new_len
    } else {
        index.min(new_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_reloads_only_clean_buffers() {
        let disk = Disk::Text("agent".into());
        assert_eq!(reconcile("base", "base", &disk, false), Decision::Reload);
        assert_eq!(reconcile("base", "base", &disk, true), Decision::Conflict);
        assert_eq!(reconcile("base", "mine", &disk, false), Decision::Conflict);
        assert_eq!(
            reconcile("base", "mine", &Disk::Text("base".into()), false),
            Decision::Unchanged
        );
        assert_eq!(
            reconcile("base", "mine", &Disk::Text("mine".into()), false),
            Decision::Accept
        );
        assert_eq!(
            reconcile("base", "base", &Disk::Missing, false),
            Decision::Conflict
        );
    }

    #[test]
    fn cursor_survives_edits_elsewhere() {
        let old = "one\ntwo\nthree\n";
        let new = "zero\none\ntwo\nthree\n";
        // "th|ree" -> still "th|ree" after a line is inserted above.
        assert_eq!(map_cursor(old, new, 10), 15);
        // Cursor inside a rewritten line moves to the start of its replacement.
        assert_eq!(map_cursor(old, "one\nTWO!\nthree\n", 5), 4);
        assert_eq!(map_cursor(old, "one\n", old.chars().count()), 4);
        assert_eq!(map_cursor("", "text", 0), 0);
    }

    #[test]
    fn monitor_reports_external_writes() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let file = root.join("chapter.md");
        fs::write(&file, "draft").unwrap();
        let monitor = Monitor::new(egui::Context::default());
        let target = Target {
            generation: 1,
            root: root.clone(),
            file: Some(file.clone()),
        };
        monitor.configure(target.clone());
        // Initial report after configuring.
        let first = monitor
            .updates
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert_eq!(first.disk, Some(Disk::Text("draft".into())));
        // Simulate an agent replacing the file atomically.
        thread::sleep(Duration::from_millis(100));
        write_checked(&file, "agent edit", None).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let update = monitor
                .updates
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            assert_eq!(update.target, target);
            if update.disk == Some(Disk::Text("agent edit".into())) {
                break;
            }
            assert!(Instant::now() < deadline, "external write was not reported");
        }
    }

    #[test]
    fn checked_write_refuses_concurrent_changes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("chapter.md");
        fs::write(&path, "base").unwrap();
        assert!(write_checked(&path, "mine", Some(&Disk::Text("base".into()))).is_ok());
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
        fs::write(&path, "agent").unwrap();
        assert_eq!(
            write_checked(&path, "again", Some(&Disk::Text("mine".into()))),
            Err(Disk::Text("agent".into()))
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "agent");
    }
}
