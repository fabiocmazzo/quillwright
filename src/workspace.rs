//! The writing workspace: menus, manuscript navigator, status bar, home screen,
//! the page, and focus mode.
use crate::i18n::{self, t};
use crate::{fonts, manuscript, project, reference, theme, DiscardAction, MarkdownApp};
use eframe::egui;
use egui::{
    text::{LayoutJob, TextWrapping},
    Align, Align2, Button, Color32, Context, CursorIcon, FontId, Layout, RichText, TextStyle, Ui,
    ViewportCommand,
};
use std::path::{Path, PathBuf};

/// Shortcuts shown in the help overlay, kept next to the code that handles them: the
/// keys, and the message that says what they do.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("F11", "shortcut-focus"),
    ("Ctrl+B", "shortcut-sidebar"),
    ("Ctrl+K", "shortcut-assistant"),
    ("Ctrl+Shift+E", "shortcut-export"),
    ("Ctrl+Enter", "shortcut-send"),
    ("Ctrl+Shift+T", "shortcut-typewriter"),
    ("Ctrl+Shift+D", "shortcut-dim"),
    ("Ctrl+] / Ctrl+[", "shortcut-text-size"),
    ("Ctrl+Shift+N", "shortcut-new-document"),
    ("Ctrl+N", "shortcut-blank"),
    ("Ctrl+O", "shortcut-open"),
    ("Ctrl+S", "shortcut-save"),
    ("Ctrl+Shift+S / Alt+S", "shortcut-save-as"),
    ("Ctrl+Z / Ctrl+Y", "shortcut-undo"),
    ("Ctrl+F / Ctrl+R", "shortcut-find"),
    ("Ctrl+Shift+H", "shortcut-search"),
    ("Ctrl+D", "shortcut-delete-line"),
    ("Page Up / Page Down", "shortcut-page"),
    ("Ctrl+Shift+L / Ctrl+Shift+F", "shortcut-lint"),
    ("Ctrl+H", "shortcut-help"),
    ("Ctrl+Q", "shortcut-quit"),
];

/// A menu item's label with its keyboard shortcut.
fn keys(label: String, keys: &str) -> String {
    format!("{label}\t{keys}")
}

/// Things the sidebar or home screen asked for; applied after drawing.
enum NavAction {
    Open(PathBuf),
    Jump(usize),
    NewChapter,
    NewCharacter,
    PickProject,
    OpenProject(PathBuf),
    NewProject,
    BlankPage,
    Home,
    Reference(PathBuf),
    AddResearch,
}

impl MarkdownApp {
    // ----- Menus -----------------------------------------------------------

    pub(crate) fn show_menu_bar(&mut self, root: &mut Ui) {
        let palette = theme::current(root.ctx());
        egui::Panel::top("menu_bar")
            .frame(
                egui::Frame::NONE
                    .fill(palette.desk)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button(t!("menu-file"), |ui| self.file_menu(ui));
                    ui.menu_button(t!("menu-view"), |ui| self.view_menu(ui));
                    ui.menu_button(t!("menu-assistant"), |ui| self.assistant_menu(ui));
                    ui.menu_button(t!("menu-tools"), |ui| {
                        if ui.button(keys(t!("menu-lint"), "Ctrl+Shift+L")).clicked() {
                            self.pending_lint = true;
                            ui.close();
                        }
                        if ui.button(keys(t!("menu-format"), "Ctrl+Shift+F")).clicked() {
                            self.pending_format = true;
                            ui.close();
                        }
                    });
                    ui.menu_button(t!("menu-help"), |ui| {
                        if ui.button(keys(t!("menu-shortcuts"), "Ctrl+H")).clicked() {
                            self.show_help_overlay = true;
                            ui.close();
                        }
                    });
                });
            });
    }

    fn file_menu(&mut self, ui: &mut Ui) {
        let documents = keys(self.kind.documents.new_menu_label(), "Ctrl+Shift+N");
        if ui.button(documents).clicked() {
            self.create_chapter();
            ui.close();
        }
        if ui.button(self.kind.sheets.new_menu_label()).clicked() {
            self.create_character();
            ui.close();
        }
        if ui
            .add_enabled(
                self.in_project_for_research(),
                Button::new(t!("menu-add-research")),
            )
            .on_hover_text(t!("menu-add-research.hint"))
            .clicked()
        {
            self.add_research_files();
            ui.close();
        }
        ui.separator();
        if ui.button(t!("menu-new-project")).clicked() {
            self.open_new_project_dialog();
            ui.close();
        }
        if ui.button(t!("menu-open-project")).clicked() {
            self.pick_project();
            ui.close();
        }
        if ui.button(t!("menu-project-home")).clicked() {
            self.go_home();
            ui.close();
        }
        ui.separator();
        if ui.button(keys(t!("menu-new-blank"), "Ctrl+N")).clicked() {
            self.run_or_prompt_discard(DiscardAction::NewDocument);
            ui.close();
        }
        if ui.button(keys(t!("menu-open-file"), "Ctrl+O")).clicked() {
            self.run_or_prompt_discard(DiscardAction::OpenDialog);
            ui.close();
        }
        ui.separator();
        if ui.button(keys(t!("menu-save"), "Ctrl+S")).clicked() {
            self.save_file(false);
            ui.close();
        }
        if ui
            .button(keys(t!("menu-save-as"), "Ctrl+Shift+S"))
            .clicked()
        {
            self.save_file(true);
            ui.close();
        }
        let mut autosave = self.config.editor.autosave;
        if ui
            .checkbox(&mut autosave, t!("menu-autosave"))
            .on_hover_text(t!("menu-autosave.hint"))
            .changed()
        {
            self.config.editor.autosave = autosave;
            self.save_config();
        }
        let has_history = self.file_path.is_some() && self.history_root.is_some();
        if ui
            .add_enabled(has_history, Button::new(t!("menu-history")))
            .on_hover_text(t!("menu-history.hint"))
            .clicked()
        {
            self.open_history();
            ui.close();
        }
        if ui.button(keys(t!("menu-export"), "Ctrl+Shift+E")).clicked() {
            self.open_export();
            ui.close();
        }
        ui.separator();
        if ui.button(keys(t!("menu-exit"), "Ctrl+Q")).clicked() {
            self.pending_exit = true;
            ui.close();
        }
    }

    fn assistant_menu(&mut self, ui: &mut Ui) {
        if ui
            .button(keys(t!("menu-assistant-panel"), "Ctrl+K"))
            .clicked()
        {
            self.toggle_assistant();
            ui.close();
        }
        ui.separator();
        let has_selection = self.selection_start != self.selection_end;
        let running = self.assistant.running();
        for action in &self.kind.assistant.actions {
            let enabled = !running && (!action.needs_selection || has_selection);
            if ui
                .add_enabled(enabled, Button::new(action.label()))
                .clicked()
            {
                let ctx = ui.ctx().clone();
                let action = crate::assistant::Action::Kind(action.clone());
                self.send_assistant(&ctx, action);
                ui.close();
            }
        }
        ui.separator();
        if ui.button(t!("menu-assistant-settings")).clicked() {
            self.open_assistant_settings();
            ui.close();
        }
    }

    fn view_menu(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(t!("menu-theme")).small().weak());
        let follows = theme::follows_system(&self.config.theme.base);
        if ui.radio(follows, t!("menu-theme-system")).clicked() {
            self.set_theme(theme::SYSTEM);
            ui.close();
        }
        let current = theme::Preset::from_name(&self.config.theme.base);
        for preset in theme::Preset::ALL {
            if ui
                .radio(!follows && current == preset, preset.label())
                .clicked()
            {
                self.set_theme(preset.name());
                ui.close();
            }
        }
        ui.separator();
        if ui.button(keys(t!("menu-focus"), "F11")).clicked() {
            self.pending_focus_toggle = true;
            ui.close();
        }
        let mut sidebar = self.config.view.sidebar;
        if ui
            .checkbox(&mut sidebar, keys(t!("menu-sidebar"), "Ctrl+B"))
            .changed()
        {
            self.config.view.sidebar = sidebar;
            self.save_config();
        }
        if ui.button(keys(t!("menu-search"), "Ctrl+Shift+H")).clicked() {
            self.open_project_search();
            ui.close();
        }
        let mut reference = self.reference.open;
        if ui
            .add_enabled(
                self.reference.has_file(),
                egui::Checkbox::new(&mut reference, t!("menu-reference")),
            )
            .on_hover_text(t!("menu-reference.hint"))
            .changed()
        {
            self.toggle_reference();
        }
        let mut typewriter = self.config.editor.typewriter;
        if ui
            .checkbox(&mut typewriter, keys(t!("menu-typewriter"), "Ctrl+Shift+T"))
            .on_hover_text(t!("menu-typewriter.hint"))
            .changed()
        {
            self.config.editor.typewriter = typewriter;
            self.save_config();
        }
        let mut dim = self.config.editor.dim_paragraphs;
        if ui
            .checkbox(&mut dim, keys(t!("menu-dim"), "Ctrl+Shift+D"))
            .on_hover_text(t!("menu-dim.hint"))
            .changed()
        {
            self.config.editor.dim_paragraphs = dim;
            self.save_config();
        }
        let mut numbers = self.config.editor.line_numbers;
        if ui.checkbox(&mut numbers, t!("menu-line-numbers")).changed() {
            self.config.editor.line_numbers = numbers;
            self.save_config();
        }
        self.spelling_menu(ui);
        ui.menu_button(t!("menu-language"), |ui| {
            let mut choice = None;
            let chosen = &self.config.view.language;
            if ui
                .radio(chosen.is_empty(), t!("menu-language-system"))
                .clicked()
            {
                choice = Some(String::new());
            }
            for language in i18n::LANGUAGES {
                if ui.radio(chosen == language.id, language.name).clicked() {
                    choice = Some(language.id.to_string());
                }
            }
            if let Some(id) = choice {
                self.set_language(&id);
                ui.close();
            }
        });
        ui.separator();
        if ui.button(keys(t!("menu-larger"), "Ctrl+]")).clicked() {
            self.change_text_size(ui.ctx(), 1.0);
        }
        if ui.button(keys(t!("menu-smaller"), "Ctrl+[")).clicked() {
            self.change_text_size(ui.ctx(), -1.0);
        }
        if ui.button(t!("menu-default-size")).clicked() {
            let delta =
                crate::editor::EditorConfig::default().font_size - fonts::typography(ui.ctx()).size;
            self.change_text_size(ui.ctx(), delta);
            ui.close();
        }
        ui.separator();
        ui.label(RichText::new(t!("menu-session-goal")).small().weak());
        let mut goal = self.config.view.session_goal;
        if ui
            .add(egui::DragValue::new(&mut goal).speed(50).range(0..=100_000))
            .on_hover_text(t!("menu-session-goal.hint"))
            .changed()
        {
            self.config.view.session_goal = goal;
            self.save_config();
        }
    }

    pub(crate) fn open_new_project_dialog(&mut self) {
        self.new_project = Some(project::NewProject {
            name: String::new(),
            parent: self
                .working_dir
                .parent()
                .filter(|_| self.index.has_documents_dir)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.working_dir.clone()),
            kind: crate::kinds::default_kind(),
            error: None,
        });
    }

    /// Sets the interface language: an id from [`i18n::LANGUAGES`], or "" for the system's.
    pub(crate) fn set_language(&mut self, id: &str) {
        self.config.view.language = id.to_string();
        i18n::set_language(id);
        self.save_config();
        // Texts kept between frames are rebuilt in the new language.
        self.window_title.clear();
        self.index_dirty = true;
    }

    /// Sets the theme: a preset name, or [`theme::SYSTEM`].
    pub(crate) fn set_theme(&mut self, name: &str) {
        self.config.theme.base = name.to_string();
        self.theme_applied = false;
        self.save_config();
    }

    pub(crate) fn change_text_size(&mut self, ctx: &Context, delta: f32) {
        let size = fonts::set_size(ctx, fonts::typography(ctx).size + delta);
        self.config.editor.font_size = size;
        self.save_config();
    }

    pub(crate) fn set_focus_mode(&mut self, ctx: &Context, on: bool) {
        self.focus_mode = on;
        self.focus_since = ctx.input(|i| i.time);
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(on));
    }

    pub(crate) fn go_home(&mut self) {
        if self.file_path.is_none() && !self.modified {
            self.show_home = true;
        } else {
            self.run_or_prompt_discard(DiscardAction::Home);
        }
    }

    // ----- New files -------------------------------------------------------

    pub(crate) fn create_chapter(&mut self) {
        let section = &self.kind.documents;
        match manuscript::new_document(&self.working_dir, section) {
            Ok(path) => {
                self.index_dirty = true;
                self.run_or_prompt_discard(DiscardAction::OpenPath(path));
            }
            Err(err) => self.show_tool_message(t!("create-file-failed", error = err.to_string())),
        }
    }

    pub(crate) fn create_character(&mut self) {
        match manuscript::new_sheet(&self.working_dir, self.kind) {
            Ok(path) => {
                self.index_dirty = true;
                self.run_or_prompt_discard(DiscardAction::OpenPath(path));
            }
            Err(err) => self.show_tool_message(t!("create-file-failed", error = err.to_string())),
        }
    }

    // ----- Sidebar ---------------------------------------------------------

    pub(crate) fn show_sidebar(&mut self, root: &mut Ui) {
        let ctx = &root.ctx().clone();
        let p = theme::current(ctx);
        let index = self.index.clone();
        let kind = self.kind;
        let current = self.file_path.clone();
        let open_words = current.as_deref().map(|path| (path, self.current_words));
        let total = index.manuscript_words(open_words);
        let outline = if self.file_path.is_some() || !self.content.is_empty() {
            manuscript::outline(&self.content)
        } else {
            Vec::new()
        };
        let cursor = self.current_char_index;
        let mut action = None;
        let mut search_action = None;

        egui::Panel::left("navigator")
            .resizable(true)
            .default_size(270.0)
            .size_range(200.0..=460.0)
            .frame(
                egui::Frame::NONE
                    .fill(p.desk)
                    .inner_margin(egui::Margin::symmetric(10, 12)),
            )
            .show(root, |ui| {
                let name = folder_name(&self.working_dir);
                ui.horizontal(|ui| {
                    // The buttons take their space first; a long name is cut short with
                    // "…" in what is left instead of running under them.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if quiet_button(ui, &t!("sidebar-home"), &p)
                            .on_hover_text(t!("sidebar-home.hint"))
                            .clicked()
                        {
                            action = Some(NavAction::Home);
                        }
                        if quiet_button(ui, &t!("sidebar-open"), &p)
                            .on_hover_text(t!("sidebar-open.hint"))
                            .clicked()
                        {
                            action = Some(NavAction::PickProject);
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&name)
                                        .font(FontId::new(19.0, fonts::family(true, false)))
                                        .color(p.text),
                                )
                                .truncate(),
                            )
                            .on_hover_text(self.working_dir.display().to_string());
                        });
                    });
                });
                if index.has_documents_dir {
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            words(total),
                            kind.documents.count(index.documents.len())
                        ))
                        .size(12.0)
                        .color(p.muted),
                    );
                }
                ui.add_space(6.0);
                self.search_field(ui, &p);
                ui.add_space(4.0);

                // A solid scroll bar takes its own space; a floating one would cover the
                // "+" buttons and word counts at the right edge.
                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if self.search.active() {
                            search_action = self.search_results(ui, &p);
                            return;
                        }
                        if index.has_documents_dir {
                            let documents = &kind.documents;
                            let new = documents.new_label();
                            if section(ui, &documents.label(), Some(&new), &p) {
                                action = Some(NavAction::NewChapter);
                            }
                            if index.documents.is_empty() {
                                hint(ui, &documents.empty_hint(), &p);
                            }
                            for (i, entry) in index.documents.iter().enumerate() {
                                let selected = current.as_ref() == Some(&entry.path);
                                let words = open_words
                                    .filter(|(path, _)| *path == entry.path)
                                    .map_or(entry.words, |(_, words)| words);
                                let label = format!("{}  {}", i + 1, entry.title);
                                if nav_row(ui, selected, 0.0, &label, Some(short_count(words)), &p)
                                    .on_hover_text(file_label(&entry.path))
                                    .clicked()
                                {
                                    action = Some(NavAction::Open(entry.path.clone()));
                                }
                            }
                        }

                        if outline.len() > 1 || outline.first().is_some_and(|h| h.level > 1) {
                            section(ui, &t!("sidebar-outline"), None, &p);
                            let active = outline
                                .iter()
                                .rposition(|heading| heading.char_index <= cursor);
                            let min_level = outline.iter().map(|h| h.level).min().unwrap_or(1);
                            for (i, heading) in outline.iter().enumerate() {
                                let indent = (heading.level - min_level) as f32 * 12.0;
                                if nav_row(ui, active == Some(i), indent, &heading.text, None, &p)
                                    .clicked()
                                {
                                    action = Some(NavAction::Jump(heading.char_index));
                                }
                            }
                        }

                        if index.has_sheets_dir {
                            let new = kind.sheets.new_label();
                            if section(ui, &kind.sheets.label(), Some(&new), &p) {
                                action = Some(NavAction::NewCharacter);
                            }
                            for entry in &index.sheets {
                                let selected = current.as_ref() == Some(&entry.path);
                                if nav_row(ui, selected, 0.0, &entry.title, None, &p)
                                    .on_hover_text(file_label(&entry.path))
                                    .clicked()
                                {
                                    action = Some(NavAction::Open(entry.path.clone()));
                                }
                            }
                        }

                        if !index.notes.is_empty() {
                            section(ui, &t!("sidebar-notes"), None, &p);
                            for entry in &index.notes {
                                let selected = current.as_ref() == Some(&entry.path);
                                if nav_row(ui, selected, 0.0, &entry.title, None, &p)
                                    .on_hover_text(file_label(&entry.path))
                                    .clicked()
                                {
                                    action = Some(NavAction::Open(entry.path.clone()));
                                }
                            }
                        }

                        if self.in_project_for_research() {
                            let research = &kind.research;
                            let add = t!("sidebar-add-research");
                            if section(ui, &research.label(), Some(&add), &p) {
                                action = Some(NavAction::AddResearch);
                            }
                            if index.research.is_empty() {
                                hint(ui, &research.hint(), &p);
                            }
                            let research_dir = self.working_dir.join(&research.folder);
                            let showing = self.reference.showing();
                            for path in &index.research {
                                let label = path
                                    .strip_prefix(&research_dir)
                                    .unwrap_or(path)
                                    .display()
                                    .to_string();
                                let kind = reference::Kind::of(path);
                                let selected = showing == Some(path.as_path());
                                if nav_row(ui, selected, 0.0, &label, Some(kind.label()), &p)
                                    .on_hover_text(file_label(path))
                                    .clicked()
                                {
                                    action = Some(NavAction::Reference(path.clone()));
                                }
                            }
                        }

                        ui.add_space(10.0);
                        let structured = index.has_documents_dir || index.has_sheets_dir;
                        egui::CollapsingHeader::new(
                            RichText::new(t!("sidebar-all-files").to_uppercase())
                                .size(11.0)
                                .color(p.muted)
                                .strong(),
                        )
                        .id_salt("all_files")
                        .default_open(!structured)
                        .show(ui, |ui| {
                            if self.working_dir.is_dir() {
                                let root = self.working_dir.clone();
                                self.render_entries(ui, &root);
                            } else {
                                hint(ui, &t!("sidebar-folder-unavailable"), &p);
                            }
                        });
                    });
            });
        self.apply_nav_action(action);
        self.apply_search_action(search_action);
    }

    fn apply_nav_action(&mut self, action: Option<NavAction>) {
        match action {
            None => {}
            Some(NavAction::Open(path)) => {
                if self.file_path.as_ref() != Some(&path) {
                    self.run_or_prompt_discard(DiscardAction::OpenPath(path));
                }
            }
            Some(NavAction::Jump(char_index)) => {
                self.pending_selection = Some((char_index, char_index));
            }
            Some(NavAction::NewChapter) => self.create_chapter(),
            Some(NavAction::NewCharacter) => self.create_character(),
            Some(NavAction::PickProject) => self.pick_project(),
            Some(NavAction::OpenProject(path)) => {
                self.run_or_prompt_discard(DiscardAction::OpenProject(path))
            }
            Some(NavAction::NewProject) => self.open_new_project_dialog(),
            Some(NavAction::BlankPage) => self.run_or_prompt_discard(DiscardAction::NewDocument),
            Some(NavAction::Home) => self.go_home(),
            Some(NavAction::Reference(path)) => self.open_reference(path),
            Some(NavAction::AddResearch) => self.add_research_files(),
        }
    }

    // ----- Status bar ------------------------------------------------------

    pub(crate) fn show_status_bar(&mut self, root: &mut Ui) {
        let ctx = &root.ctx().clone();
        let p = theme::current(ctx);
        let open_words = self
            .file_path
            .as_deref()
            .map(|path| (path, self.current_words));
        let total = self.index.manuscript_words(open_words);
        let is_chapter = self
            .file_path
            .as_ref()
            .is_some_and(|path| self.index.documents.iter().any(|e| &e.path == path));
        let selected_words = (self.selection_start != self.selection_end)
            .then(|| self.selected_text())
            .flatten()
            .map(|text| manuscript::word_count(&text))
            .filter(|&n| n > 0);

        egui::Panel::bottom("status_bar")
            .frame(
                egui::Frame::NONE
                    .fill(p.desk)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let small = |text: String| RichText::new(text).size(12.5).color(p.muted);
                    if self.modified && self.can_save_silently() {
                        ui.label(RichText::new(t!("status-edited")).size(12.5).color(p.muted))
                            .on_hover_text(t!("status-edited.autosave"));
                    } else if self.modified {
                        ui.label(
                            RichText::new(format!("• {}", t!("status-edited")))
                                .size(12.5)
                                .color(p.accent),
                        )
                        .on_hover_text(t!("status-edited.unsaved"));
                    } else {
                        ui.label(RichText::new(t!("status-saved")).size(12.5).color(p.muted));
                    }
                    ui.label(
                        RichText::new(self.document_title())
                            .size(13.0)
                            .color(p.text),
                    )
                    .on_hover_text(
                        self.file_path
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| t!("status-not-saved")),
                    );
                    if let Some(notice) = &self.sync_notice {
                        ui.label(small(notice.clone()));
                    }
                    if let Some(status) = &self.find_status {
                        ui.label(small(status.clone()));
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let goal = self.config.view.session_goal;
                        let session = self.session_words;
                        if goal > 0 {
                            let done = session.max(0) as f32 / goal as f32;
                            ui.add(
                                egui::ProgressBar::new(done.min(1.0))
                                    .desired_width(90.0)
                                    .desired_height(6.0)
                                    .corner_radius(3)
                                    .fill(p.accent),
                            )
                            .on_hover_text(t!(
                                "status-goal",
                                words = i18n::group(session.max(0) as usize),
                                goal = i18n::group(goal),
                                count = goal
                            ));
                        }
                        // Right to left: the first part added sits at the far right.
                        let sign = if session >= 0 { "+" } else { "−" };
                        let session_words = session.unsigned_abs() as usize;
                        let mut parts = vec![t!(
                            "status-session",
                            words = format!("{sign}{}", i18n::group(session_words)),
                            count = session_words
                        )];
                        if self.index.has_documents_dir {
                            parts.push(t!(
                                "status-manuscript",
                                words = i18n::group(total),
                                count = total
                            ));
                        }
                        if !self.show_home || self.file_path.is_some() {
                            let count = self.current_words;
                            let words = if is_chapter {
                                let (document, gender) = self.kind.documents.noun();
                                t!(
                                    "status-words-in",
                                    words = i18n::group(count),
                                    count = count,
                                    document = document,
                                    gender = gender
                                )
                            } else {
                                words(count)
                            };
                            let minutes = manuscript::reading_minutes(count);
                            parts.push(format!(
                                "{words} · {}",
                                t!("status-reading", minutes = minutes)
                            ));
                            if let Some(words) = selected_words {
                                parts.push(t!(
                                    "status-selected",
                                    words = i18n::group(words),
                                    count = words
                                ));
                            }
                            parts.push(t!("status-line", line = self.current_line + 1));
                        }
                        parts.reverse();
                        ui.label(small(parts.join("  ·  ")))
                            .on_hover_text(t!("status-session.hint"));
                    });
                });
            });
    }

    pub(crate) fn document_title(&self) -> String {
        match &self.file_path {
            Some(path) => manuscript::title(&self.content, path),
            None if self.show_home && !self.modified => t!("title-home"),
            None => t!("title-untitled"),
        }
    }

    // ----- Home screen -----------------------------------------------------

    pub(crate) fn show_home_screen(&mut self, ui: &mut Ui) {
        let p = theme::current(ui.ctx());
        let index = self.index.clone();
        let kind = self.kind;
        let screen_height = ui.ctx().content_rect().height();
        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let width = (ui.available_width() - 48.0).clamp(240.0, 500.0);
                let side = ((ui.available_width() - width) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(side);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        ui.add_space((screen_height * 0.1).max(32.0));
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Quillwright")
                                    .font(FontId::new(46.0, fonts::family(true, false)))
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new(t!("home-tagline"))
                                    .font(FontId::new(17.0, fonts::family(false, true)))
                                    .color(p.muted),
                            );
                        });
                        ui.add_space(32.0);

                        let has_project = index.has_documents_dir || index.has_sheets_dir;
                        if has_project {
                            card(ui, &p, |ui| {
                                ui.label(
                                    RichText::new(t!("home-current-project").to_uppercase())
                                        .size(11.0)
                                        .color(p.muted),
                                );
                                ui.label(
                                    RichText::new(folder_name(&self.working_dir))
                                        .font(FontId::new(24.0, fonts::family(true, false)))
                                        .color(p.text),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {} · {}",
                                        words(index.manuscript_words(None)),
                                        kind.documents.count(index.documents.len()),
                                        kind.sheets.count(index.sheets.len())
                                    ))
                                    .size(13.0)
                                    .color(p.muted),
                                );
                                ui.add_space(14.0);
                                let (label, target) = match index.documents.last() {
                                    Some(last) => (
                                        t!("home-continue", title = last.title.as_str()),
                                        Some(NavAction::Open(last.path.clone())),
                                    ),
                                    None => {
                                        (kind.documents.first_action(), Some(NavAction::NewChapter))
                                    }
                                };
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 38.0],
                                        Button::new(RichText::new(label).size(15.0).color(p.page))
                                            .fill(p.accent),
                                    )
                                    .clicked()
                                {
                                    action = target;
                                }
                            });
                            ui.add_space(16.0);
                        }

                        ui.columns(3, |cols| {
                            let big = |key: &str| RichText::new(t!(key)).size(14.0);
                            if cols[0]
                                .add_sized(
                                    [cols[0].available_width(), 34.0],
                                    Button::new(big("home-new-project")),
                                )
                                .clicked()
                            {
                                action = Some(NavAction::NewProject);
                            }
                            if cols[1]
                                .add_sized(
                                    [cols[1].available_width(), 34.0],
                                    Button::new(big("home-open-project")),
                                )
                                .clicked()
                            {
                                action = Some(NavAction::PickProject);
                            }
                            if cols[2]
                                .add_sized(
                                    [cols[2].available_width(), 34.0],
                                    Button::new(big("home-blank-page")),
                                )
                                .clicked()
                            {
                                action = Some(NavAction::BlankPage);
                            }
                        });

                        let recent: Vec<_> = self
                            .config
                            .recent_projects
                            .iter()
                            .filter(|path| **path != self.working_dir && path.is_dir())
                            .cloned()
                            .collect();
                        if !recent.is_empty() {
                            ui.add_space(12.0);
                            section(ui, &t!("home-recent"), None, &p);
                            for path in recent {
                                let parent = path
                                    .parent()
                                    .map(|parent| parent.display().to_string())
                                    .unwrap_or_default();
                                if nav_row(ui, false, 0.0, &folder_name(&path), Some(parent), &p)
                                    .clicked()
                                {
                                    action = Some(NavAction::OpenProject(path));
                                }
                            }
                        }

                        ui.add_space(28.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(t!("home-keys")).size(12.0).color(p.muted));
                        });
                        ui.add_space(24.0);
                    });
                });
            });
        self.apply_nav_action(action);
    }

    // ----- The page --------------------------------------------------------

    pub(crate) fn show_editor(&mut self, ui: &mut Ui, ctx: &Context) {
        let viewport_height = ui.available_height();
        // Paint the entire viewport, including scrollbar and empty document space.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().extreme_bg_color);
        self.editor_line_height = ui.text_style_height(&TextStyle::Monospace).max(1.0);
        let mut editor_area = egui::ScrollArea::vertical()
            .id_salt("page")
            .auto_shrink([false; 2]);
        let mut requested_offset = None;
        if let Some(offset) = self.pending_editor_scroll.take() {
            requested_offset = Some(offset);
            editor_area = editor_area.vertical_scroll_offset(offset);
        }
        let scroll = editor_area.show(ui, |ui| {
            let response = self.editor.show(
                ui,
                &mut self.content,
                self.pending_selection.take(),
                &self.config.editor,
                viewport_height,
                self.config.spelling.enabled.then_some(&self.spelling),
            );
            response.context_menu(|ui| self.spelling_context_menu(ui));
            self.editor_has_focus = response.has_focus();
            let range = self.editor.selection;
            let sorted = range.sorted_cursors();
            self.current_char_index = range.primary.index.0;
            self.current_line = self
                .content
                .chars()
                .take(self.current_char_index)
                .filter(|&ch| ch == '\n')
                .count();
            self.selection_start = sorted[0].index.0;
            self.selection_end = sorted[1].index.0;
            if response.changed() {
                self.update_modified_state();
            }
            let typography = fonts::typography(ctx);
            self.editor_line_height = typography.size * typography.line_height;
        });
        if let Some(target) = requested_offset {
            let max_offset = (scroll.content_size.y - scroll.inner_rect.height()).max(0.0);
            let clamped = target.clamp(0.0, max_offset);
            if (scroll.state.clamped_offset().y - clamped).abs() > 1.0 {
                self.pending_editor_scroll = Some(clamped);
            }
        }
        let visible_height = scroll.inner_rect.height();
        if visible_height > 0.0 {
            let lines = (visible_height / self.editor_line_height.max(1.0)).max(1.0) as usize;
            self.visible_editor_lines = lines.max(1);
        }
    }

    // ----- Focus mode ------------------------------------------------------

    pub(crate) fn show_focus_overlay(&mut self, ctx: &Context) {
        let p = theme::current(ctx);
        let faint = p.muted.gamma_multiply(0.75);
        egui::Area::new(egui::Id::unique("focus_stats"))
            .anchor(Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -14.0))
            .interactable(false)
            .show(ctx, |ui| {
                let mut text = words(self.current_words);
                let goal = self.config.view.session_goal;
                if goal > 0 {
                    text.push_str("  ·  ");
                    text.push_str(&t!(
                        "focus-goal",
                        words = i18n::group(self.session_words.max(0) as usize),
                        goal = i18n::group(goal)
                    ));
                }
                if self.modified {
                    text.push_str("  ·  ");
                    text.push_str(&t!("focus-unsaved"));
                }
                ui.label(RichText::new(text).size(12.0).color(faint));
            });
        // A short hint on how to leave, fading out after a few seconds.
        let elapsed = ctx.input(|i| i.time) - self.focus_since;
        if elapsed < 4.0 {
            let alpha = (1.0 - (elapsed as f32 - 2.5).max(0.0) / 1.5).clamp(0.0, 1.0);
            egui::Area::new(egui::Id::unique("focus_hint"))
                .anchor(Align2::CENTER_TOP, egui::vec2(0.0, 16.0))
                .interactable(false)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(t!("focus-hint"))
                            .size(12.5)
                            .color(faint.gamma_multiply(alpha)),
                    );
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    // ----- Bookkeeping after each frame ------------------------------------

    pub(crate) fn after_frame(&mut self, ctx: &Context) {
        if self.content != self.counted_text {
            self.saving.last_edit = ctx.input(|i| i.time);
            let words = manuscript::word_count(&self.content);
            if !self.external_load {
                self.session_words += words as i64 - self.current_words as i64;
            }
            self.current_words = words;
            self.counted_text = self.content.clone();
        }
        self.external_load = false;

        let title = format!(
            "{}{} — {} — Quillwright",
            if self.modified { "• " } else { "" },
            self.document_title(),
            folder_name(&self.working_dir)
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    pub(crate) fn remember_project(&mut self, root: &Path) {
        let recent = &mut self.config.recent_projects;
        recent.retain(|path| path != root);
        recent.insert(0, root.to_path_buf());
        recent.truncate(8);
    }
}

// ----- Small widgets ---------------------------------------------------------

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// "1 word", "12,345 words".
pub(crate) fn words(n: usize) -> String {
    t!("words", words = i18n::group(n), count = n)
}

fn short_count(words: usize) -> String {
    if words >= 10_000 {
        t!("thousands", count = format!("{:.1}", words as f32 / 1000.0))
    } else {
        i18n::group(words)
    }
}

fn quiet_button(ui: &mut Ui, text: &str, p: &theme::Palette) -> egui::Response {
    ui.add(Button::new(RichText::new(text).size(12.5).color(p.muted)).frame(false))
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// An uppercase section label with an optional "+" action. Returns true when clicked.
fn section(ui: &mut Ui, label: &str, action: Option<&str>, p: &theme::Palette) -> bool {
    ui.add_space(12.0);
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label.to_uppercase())
                .size(11.0)
                .strong()
                .color(p.muted),
        );
        if let Some(tip) = action {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = ui
                    .add(Button::new(RichText::new("+").size(16.0).color(p.muted)).frame(false))
                    .on_hover_text(tip)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked();
            });
        }
    });
    clicked
}

fn hint(ui: &mut Ui, text: &str, p: &theme::Palette) {
    ui.label(RichText::new(text).size(12.5).italics().color(p.muted));
}

fn card(ui: &mut Ui, p: &theme::Palette, add: impl FnOnce(&mut Ui)) {
    egui::Frame::NONE
        .fill(theme::mix(p.page, p.desk, 0.6))
        .stroke(egui::Stroke::new(1.0_f32, theme::mix(p.desk, p.text, 0.1)))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// A full-width navigation row: title on the left, muted detail on the right.
fn nav_row(
    ui: &mut Ui,
    selected: bool,
    indent: f32,
    title: &str,
    trailing: Option<String>,
    p: &theme::Palette,
) -> egui::Response {
    let height = 26.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let background = if selected {
            Some(theme::mix(p.desk, p.accent, 0.2))
        } else if response.hovered() {
            Some(theme::mix(p.desk, p.text, 0.07))
        } else {
            None
        };
        if let Some(color) = background {
            ui.painter().rect_filled(rect, 6.0, color);
        }
        if selected {
            let bar = egui::Rect::from_min_size(
                rect.min + egui::vec2(0.0, 6.0),
                egui::vec2(3.0, height - 12.0),
            );
            ui.painter().rect_filled(bar, 1.5, p.accent);
        }
        let mut right = rect.right() - 8.0;
        if let Some(text) = trailing {
            let mut job = LayoutJob::simple_singleline(text, FontId::proportional(12.0), p.muted);
            job.wrap = TextWrapping {
                max_width: (rect.width() * 0.45).max(20.0),
                max_rows: 1,
                break_anywhere: true,
                overflow_character: Some('…'),
            };
            let galley = ui.painter().layout_job(job);
            right -= galley.size().x;
            let pos = egui::pos2(right, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(pos, galley, p.muted);
            right -= 10.0;
        }
        let left = rect.left() + 10.0 + indent;
        let mut job = LayoutJob::simple_singleline(
            title.to_owned(),
            FontId::proportional(14.0),
            if selected {
                p.text
            } else {
                theme::mix(p.text, p.muted, 0.25)
            },
        );
        job.wrap = TextWrapping {
            max_width: (right - left).max(10.0),
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        };
        let galley = ui.painter().layout_job(job);
        let pos = egui::pos2(left, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, Color32::PLACEHOLDER);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}
