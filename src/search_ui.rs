//! The project search field at the top of the sidebar, and its results.
use crate::i18n::t;
use crate::{
    reference,
    search::{Hit, Location, Results, Searcher, Source, SourceKind},
    theme, DiscardAction, MarkdownApp,
};
use eframe::egui;
use egui::{
    text::{LayoutJob, TextFormat, TextWrapping},
    Color32, CursorIcon, FontId, RichText, Sense, TextEdit, Ui,
};
use std::{path::PathBuf, time::Duration};

/// Wait this long after the last keystroke before searching.
const DEBOUNCE: f64 = 0.25;

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    searcher: Option<Searcher>,
    results: Option<Results>,
    generation: u64,
    /// The query last sent, so edits can be told apart from no change.
    sent: Option<String>,
    edited_at: f64,
    pub focus: bool,
    /// Select this range once the file is open in the editor.
    reveal: Option<(PathBuf, usize, usize)>,
}

impl SearchState {
    /// True while the sidebar shows search results instead of the manuscript.
    pub fn active(&self) -> bool {
        !self.query.trim().is_empty()
    }

    /// Search again (the files changed).
    pub fn refresh(&mut self) {
        if self.active() {
            self.sent = None;
        }
    }
}

pub(crate) enum SearchAction {
    Open(PathBuf, usize, usize),
    /// A PDF, the query that found the match, its page and character index.
    Pdf(PathBuf, String, usize, usize),
}

impl MarkdownApp {
    pub(crate) fn open_project_search(&mut self) {
        if !self.config.view.sidebar {
            self.config.view.sidebar = true;
            self.save_config();
        }
        self.search.focus = true;
    }

    fn project_sources(&self) -> Vec<Source> {
        let open = self.file_path.as_ref();
        let source = |path: &PathBuf, title: &str, kind: SourceKind, pdf: bool| Source {
            path: path.clone(),
            title: title.to_owned(),
            kind,
            pdf,
            text: (open == Some(path)).then(|| self.content.clone()),
        };
        let index = &self.index;
        let mut sources = Vec::new();
        for entry in &index.documents {
            sources.push(source(
                &entry.path,
                &entry.title,
                SourceKind::Chapter,
                false,
            ));
        }
        for entry in &index.sheets {
            sources.push(source(
                &entry.path,
                &entry.title,
                SourceKind::Character,
                false,
            ));
        }
        for entry in &index.notes {
            sources.push(source(&entry.path, &entry.title, SourceKind::Note, false));
        }
        let research_dir = self.working_dir.join(&self.kind.research.folder);
        for path in &index.research {
            let pdf = match reference::Kind::of(path) {
                reference::Kind::Pdf => true,
                reference::Kind::Text | reference::Kind::Markdown => false,
                _ => continue,
            };
            let title = path
                .strip_prefix(&research_dir)
                .unwrap_or(path)
                .display()
                .to_string();
            sources.push(source(path, &title, SourceKind::Research, pdf));
        }
        sources
    }

    /// Send the query when typing pauses, and collect results.
    pub(crate) fn poll_project_search(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let query = self.search.query.trim().to_owned();
        if self.search.sent.as_deref() != Some(query.as_str()) {
            if now - self.search.edited_at >= DEBOUNCE {
                if query.is_empty() {
                    self.search.results = None;
                } else {
                    let sources = self.project_sources();
                    let searcher = self.search.searcher.get_or_insert_with(Searcher::new);
                    self.search.generation = searcher.search(&query, sources);
                }
                self.search.sent = Some(query);
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(DEBOUNCE));
            }
        }
        if let Some(results) = self.search.searcher.as_ref().and_then(|s| s.poll()) {
            if results.generation == self.search.generation {
                self.search.results = Some(results);
            }
        }
        let working = self.search.active()
            && self
                .search
                .results
                .as_ref()
                .is_none_or(|r| r.generation != self.search.generation || !r.done);
        if working {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        // Select the match once its file is open (after a save or a prompt, if needed).
        if let Some((path, start, end)) = self.search.reveal.clone() {
            if self.file_path.as_ref() == Some(&path) {
                self.pending_selection = Some((start, end));
                self.search.reveal = None;
            } else if self.pending_discard_action.is_none() {
                self.search.reveal = None; // The author cancelled, or it failed.
            }
        }
    }

    pub(crate) fn search_field(&mut self, ui: &mut Ui, p: &theme::Palette) {
        let response = ui.add(
            TextEdit::singleline(&mut self.search.query)
                .hint_text(format!("{} (Ctrl+Shift+H)", t!("search-hint")))
                .desired_width(f32::INFINITY),
        );
        if std::mem::take(&mut self.search.focus) {
            response.request_focus();
        }
        if response.changed() {
            self.search.edited_at = ui.input(|i| i.time);
        }
        if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.search.query.clear();
            self.search.edited_at = 0.0;
        }
        if self.search.active() {
            ui.label(
                RichText::new(self.search_status())
                    .size(12.0)
                    .color(p.muted),
            );
        }
    }

    fn search_status(&self) -> String {
        let Some(results) = self
            .search
            .results
            .as_ref()
            .filter(|r| r.generation == self.search.generation)
        else {
            return t!("search-searching");
        };
        let mut status = match (results.total(), results.files.len()) {
            (0, _) if results.done => t!("search-none"),
            (0, _) => t!("search-searching"),
            (n, files) => t!("search-results", count = n, files = files),
        };
        if let Some(title) = &results.reading {
            status.push_str(" · ");
            status.push_str(&t!("search-reading", title = title.as_str()));
        } else if !results.done && results.total() > 0 {
            status.push_str(" · ");
            status.push_str(&t!("search-more"));
        }
        if results.failed > 0 {
            status.push_str(" · ");
            status.push_str(&t!("search-failed", count = results.failed));
        }
        status
    }

    /// The results list. Returns what the author clicked.
    pub(crate) fn search_results(&self, ui: &mut Ui, p: &theme::Palette) -> Option<SearchAction> {
        let results = self
            .search
            .results
            .as_ref()
            .filter(|r| r.generation == self.search.generation)?;
        let mut action = None;
        for file in &results.files {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&file.source.title)
                        .size(13.5)
                        .strong()
                        .color(p.text),
                )
                .on_hover_text(file.source.path.display().to_string());
                ui.label(
                    RichText::new(format!(
                        "{} · {}",
                        self.source_label(file.source.kind),
                        file.total
                    ))
                    .size(11.5)
                    .color(p.muted),
                );
            });
            for hit in &file.hits {
                if hit_row(ui, hit, p).clicked() {
                    let path = file.source.path.clone();
                    action = Some(match hit.location {
                        Location::Page(page) => {
                            SearchAction::Pdf(path, results.query.clone(), page, hit.range.start)
                        }
                        Location::Line(_) => {
                            SearchAction::Open(path, hit.range.start, hit.range.end)
                        }
                    });
                }
            }
            if file.total > file.hits.len() {
                ui.label(
                    RichText::new(t!("search-and-more", count = file.total - file.hits.len()))
                        .size(12.0)
                        .italics()
                        .color(p.muted),
                );
            }
        }
        action
    }

    /// What kind of file a result is in: "Chapter", "Note".
    fn source_label(&self, kind: SourceKind) -> String {
        let noun = |section: &crate::kinds::Section| crate::kinds::capitalize(&section.noun().0);
        match kind {
            SourceKind::Chapter => noun(&self.kind.documents),
            SourceKind::Character => noun(&self.kind.sheets),
            SourceKind::Note => t!("search-note"),
            SourceKind::Research => self.kind.research.label(),
        }
    }

    pub(crate) fn apply_search_action(&mut self, action: Option<SearchAction>) {
        match action {
            None => {}
            Some(SearchAction::Open(path, start, end)) => {
                if self.file_path.as_ref() == Some(&path) {
                    self.pending_selection = Some((start, end));
                } else {
                    self.search.reveal = Some((path.clone(), start, end));
                    self.run_or_prompt_discard(DiscardAction::OpenPath(path));
                }
            }
            Some(SearchAction::Pdf(path, query, page, start)) => {
                self.reveal_in_pdf(path, query, page, start);
            }
        }
    }
}

/// One result: where it is, and the match in its context.
fn hit_row(ui: &mut Ui, hit: &Hit, p: &theme::Palette) -> egui::Response {
    let background = ui.painter().add(egui::Shape::Noop);
    let width = ui.available_width();
    let plain = |color: Color32| TextFormat {
        font_id: FontId::proportional(12.5),
        color,
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    let place = match hit.location {
        Location::Line(line) => format!("{line}  "),
        Location::Page(page) => format!("{}  ", t!("search-page", page = page + 1)),
    };
    job.append(&place, 0.0, plain(p.muted));
    job.append(&hit.before, 0.0, plain(theme::mix(p.text, p.muted, 0.45)));
    job.append(
        &hit.matched,
        0.0,
        TextFormat {
            background: theme::mix(p.desk, p.accent, 0.35),
            ..plain(p.text)
        },
    );
    job.append(&hit.after, 0.0, plain(theme::mix(p.text, p.muted, 0.45)));
    job.wrap = TextWrapping {
        max_width: width - 12.0,
        max_rows: 2,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, galley.size().y + 8.0), Sense::click());
    if response.hovered() {
        ui.painter().set(
            background,
            egui::Shape::rect_filled(rect, 6.0, theme::mix(p.desk, p.text, 0.07)),
        );
    }
    ui.painter()
        .galley(rect.min + egui::vec2(6.0, 4.0), galley, p.text);
    response.on_hover_cursor(CursorIcon::PointingHand)
}
