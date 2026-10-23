//! Autosave, saving before leaving a file, and the Version History window.
use crate::i18n::t;
use crate::{assistant_ui, file_sync, history, manuscript, theme, MarkdownApp};
use eframe::egui;
use egui::{Context, RichText};
use std::time::SystemTime;

/// Seconds to wait before retrying an autosave that failed.
const RETRY_AFTER: f64 = 30.0;

#[derive(Default)]
pub struct SavingState {
    /// When the buffer last changed (egui time, in seconds).
    pub last_edit: f64,
    /// The window lost focus since the last frame: save now.
    pub focus_lost: bool,
    retry_at: f64,
    history: Option<HistoryView>,
}

struct HistoryView {
    snapshots: Vec<history::Snapshot>,
    words: Vec<Option<usize>>,
    selected: usize,
    /// Text of the selected copy and its diff against the editor, rebuilt when either changes.
    text: Option<String>,
    diff: Option<egui::text::LayoutJob>,
    diffed_against: String,
    error: Option<String>,
}

impl MarkdownApp {
    /// Autosave can write the open file without asking anything.
    pub(crate) fn can_save_silently(&self) -> bool {
        self.config.editor.autosave
            && self.file_path.is_some()
            && self.disk_conflict.is_none()
            && !self.base_missing
    }

    /// Called once per frame, after the buffer has been edited.
    pub(crate) fn run_autosave(&mut self, ctx: &Context) {
        let focus_lost = std::mem::take(&mut self.saving.focus_lost);
        if !self.modified || !self.can_save_silently() || self.editor.is_composing() {
            return;
        }
        let now = ctx.input(|i| i.time);
        if now < self.saving.retry_at {
            ctx.request_repaint_after(secs(self.saving.retry_at - now));
            return;
        }
        let delay = f64::from(self.config.editor.autosave_delay.clamp(0.5, 600.0));
        let waited = now - self.saving.last_edit;
        if !focus_lost && waited < delay {
            // egui only repaints on input; wake up when the save is due.
            ctx.request_repaint_after(secs(delay - waited));
            return;
        }
        if let Err(err) = self.save_in_place() {
            self.sync_notice = Some(format!(
                "⚠ {}",
                t!("autosave-failed", error = err.to_string())
            ));
            self.saving.retry_at = now + RETRY_AFTER;
        }
    }

    /// Save before closing the file, the project, or the app.
    /// Returns true when nothing unsaved is left.
    pub(crate) fn save_before_leaving(&mut self) -> bool {
        if self.modified && self.can_save_silently() {
            if let Err(err) = self.save_in_place() {
                self.show_tool_message(t!("save-failed", error = err.to_string()));
            }
        }
        !self.modified
    }

    /// Write the buffer to the open file. A conflicting change on disk is not an
    /// error: it opens the conflict dialog instead.
    pub(crate) fn save_in_place(&mut self) -> Result<(), String> {
        let Some(path) = self.file_path.clone() else {
            return Ok(());
        };
        let disk = file_sync::read(&path);
        let local = file_sync::Disk::Text(self.content.clone());
        if disk != self.expected_disk() && disk != local {
            // Someone else changed the file since we loaded it: never clobber silently.
            self.disk_conflict = Some(disk);
            return Ok(());
        }
        // Keep the version being replaced. A large cut is always kept.
        if let file_sync::Disk::Text(old) = &disk {
            let big_cut = self.content.len() < old.len() * 3 / 4;
            self.record_history(&old.clone(), big_cut);
        }
        match file_sync::write_checked(&path, &self.content, Some(&disk)) {
            Ok(()) => {
                self.mark_saved();
                Ok(())
            }
            Err(file_sync::Disk::Unreadable(err)) => Err(err),
            Err(changed) => {
                self.disk_conflict = Some(changed);
                Ok(())
            }
        }
    }

    /// Keep a copy of `text` in the open file's history.
    pub(crate) fn record_history(&mut self, text: &str, force: bool) {
        let (Some(root), Some(file)) = (self.history_root.as_ref(), self.file_path.as_ref()) else {
            return;
        };
        if let Err(err) = history::record(root, file, text, force, SystemTime::now()) {
            self.sync_notice = Some(format!(
                "⚠ {}",
                t!("history-copy-failed", error = err.to_string())
            ));
        }
    }

    pub(crate) fn open_history(&mut self) {
        let (Some(root), Some(file)) = (self.history_root.as_ref(), self.file_path.as_ref()) else {
            return;
        };
        let snapshots = history::list(root, file);
        let words = snapshots
            .iter()
            .map(|s| s.read().ok().map(|text| manuscript::word_count(&text)))
            .collect();
        self.saving.history = Some(HistoryView {
            snapshots,
            words,
            selected: 0,
            text: None,
            diff: None,
            diffed_against: String::new(),
            error: None,
        });
    }

    pub(crate) fn show_history_window(&mut self, ctx: &Context) {
        let title = t!("history-title", title = self.document_title());
        let Some(view) = &mut self.saving.history else {
            return;
        };
        let p = theme::current(ctx);
        let now = SystemTime::now();

        // Load the selected copy and diff it against the editor when needed.
        if let Some(snapshot) = view.snapshots.get(view.selected) {
            if view.text.is_none() {
                match snapshot.read() {
                    Ok(text) => view.text = Some(text),
                    Err(err) => {
                        view.error = Some(t!("history-read-failed", error = err.to_string()))
                    }
                }
                view.diff = None;
            }
            if let Some(text) = &view.text {
                if view.diff.is_none() || view.diffed_against != self.content {
                    view.diff = Some(assistant_ui::diff_job(&self.content, text, &p));
                    view.diffed_against = self.content.clone();
                }
            }
        }

        let mut open = true;
        let mut restore = None;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .default_size([760.0, 520.0])
            .show(ctx, |ui| {
                if view.snapshots.is_empty() {
                    ui.label(t!("history-empty"));
                    ui.label(RichText::new(t!("history-empty.when")).color(p.muted));
                    return;
                }
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        egui::ScrollArea::vertical()
                            .id_salt("history_list")
                            .show(ui, |ui| {
                                for (i, snapshot) in view.snapshots.iter().enumerate() {
                                    let mut label = history::time_label(snapshot.time, now);
                                    if let Some(words) = view.words[i] {
                                        label.push('\n');
                                        label.push_str(&crate::workspace::words(words));
                                    }
                                    let response = ui.add_sized(
                                        [ui.available_width(), 0.0],
                                        egui::Button::selectable(i == view.selected, label),
                                    );
                                    if response.clicked() && i != view.selected {
                                        view.selected = i;
                                        view.text = None;
                                        view.error = None;
                                    }
                                }
                            });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(t!("history-compared"))
                                .size(12.5)
                                .color(p.muted),
                        );
                        if let Some(error) = &view.error {
                            ui.colored_label(p.accent, error);
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt("history_diff")
                            .auto_shrink([false, false])
                            .max_height((ui.available_height() - 44.0).max(80.0))
                            .show(ui, |ui| {
                                if let Some(diff) = &view.diff {
                                    ui.label(diff.clone());
                                }
                            });
                        ui.separator();
                        let ready = view.text.is_some();
                        if ui
                            .add_enabled(ready, egui::Button::new(t!("history-restore")))
                            .on_hover_text(t!("history-restore.hint"))
                            .clicked()
                        {
                            restore = view.text.clone();
                        }
                    });
                });
            });

        if let Some(text) = restore {
            self.restore_version(text);
            open = false;
        }
        if !open {
            self.saving.history = None;
        }
    }

    fn restore_version(&mut self, text: String) {
        let current = self.content.clone();
        self.record_history(&current, true);
        let selection = self.editor.selection;
        let map = |index| file_sync::map_cursor(&current, &text, index);
        self.editor.selection = egui::text::CCursorRange::two(
            egui::text::CCursor::new(map(selection.secondary.index.0)),
            egui::text::CCursor::new(map(selection.primary.index.0)),
        );
        self.content = text;
        // Restored text is not counted as words typed this session.
        self.external_load = true;
        self.update_modified_state();
        self.sync_notice = Some(format!("↺ {}", t!("history-restored")));
    }
}

fn secs(s: f64) -> std::time::Duration {
    std::time::Duration::from_secs_f64(s.max(0.0) + 0.05)
}
