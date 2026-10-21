//! Spelling in the app: the editor's context menu, the View menu, and keeping the
//! dictionary in step with the project.
use crate::{spelling, MarkdownApp};
use eframe::egui::{self, RichText, Ui};

impl MarkdownApp {
    /// Pick the language for the open project, start loading it, and read the
    /// project's word list. Runs whenever the manuscript index is rebuilt.
    pub(crate) fn refresh_spelling(&mut self) {
        if !self.config.spelling.enabled {
            return;
        }
        let language =
            spelling::resolve_language(&self.config.spelling.language, &self.working_dir);
        self.spelling.set_language(&language);
        let names = self.index.sheets.iter().flat_map(|entry| {
            let stem = entry
                .path
                .file_stem()
                .map(|s| s.to_string_lossy().replace(['-', '_'], " "))
                .unwrap_or_default();
            [entry.title.clone(), stem]
        });
        let names: Vec<String> = names.collect();
        self.spelling.set_project(&self.working_dir, names);
    }

    pub(crate) fn poll_spelling(&mut self, ctx: &egui::Context) {
        if self.spelling.poll() {
            ctx.request_repaint();
        }
        if self.spelling.is_loading() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    /// Right-click menu on the page: suggestions for a misspelled word.
    pub(crate) fn spelling_context_menu(&mut self, ui: &mut Ui) {
        let target = self
            .editor
            .context_index
            .and_then(|index| spelling::word_at(&self.content, index))
            .filter(|(_, word)| {
                self.config.spelling.enabled
                    && self.spelling.is_ready()
                    && !self.spelling.is_correct(word)
            });
        let Some((range, word)) = target else {
            ui.label(RichText::new("No spelling suggestions").weak());
            return;
        };
        let suggestions = self.spelling.suggestions(&word);
        if suggestions.is_empty() {
            ui.label(RichText::new("No suggestions").weak());
        }
        for suggestion in suggestions {
            if ui.button(RichText::new(&suggestion).strong()).clicked() {
                self.editor
                    .replace_chars(&mut self.content, range.clone(), &suggestion);
                self.update_modified_state();
                ui.close();
            }
        }
        ui.separator();
        if ui
            .button("Add to Project Dictionary")
            .on_hover_text(format!(
                "Adds “{word}” to {} in the project folder",
                spelling::PROJECT_WORDS_FILE
            ))
            .clicked()
        {
            if let Err(err) = self.spelling.add_to_project(&word) {
                self.show_tool_message(format!(
                    "Could not update {}: {err}",
                    spelling::PROJECT_WORDS_FILE
                ));
            }
            ui.close();
        }
        if ui.button("Ignore for This Session").clicked() {
            self.spelling.ignore(&word);
            ui.close();
        }
    }

    /// "Check Spelling" and the language submenu, in the View menu.
    pub(crate) fn spelling_menu(&mut self, ui: &mut Ui) {
        let mut enabled = self.config.spelling.enabled;
        if ui
            .checkbox(&mut enabled, "Check Spelling")
            .on_hover_text("Underline misspelled words; right-click one for suggestions")
            .changed()
        {
            self.config.spelling.enabled = enabled;
            self.save_config();
            self.refresh_spelling();
        }
        ui.add_enabled_ui(enabled, |ui| {
            ui.menu_button("Spelling Language", |ui| self.spelling_language_menu(ui));
        });
    }

    /// True when the working folder is a book, so its language belongs in its
    /// `metadata.yaml` rather than in the app's settings.
    fn in_project(&self) -> bool {
        !crate::file_sync::is_broad(&self.working_dir)
            && (self.index.has_documents_dir || self.working_dir.join("metadata.yaml").is_file())
    }

    fn spelling_language_menu(&mut self, ui: &mut Ui) {
        let in_project = self.in_project();
        let current = spelling::resolve_language(&self.config.spelling.language, &self.working_dir);
        ui.label(
            RichText::new(if in_project {
                "This book (saved in metadata.yaml)"
            } else {
                "Default for books without a language"
            })
            .small()
            .weak(),
        );
        let mut choice = None;
        for language in spelling::available() {
            if ui.radio(current == language, &language).clicked() {
                choice = Some(language);
            }
        }
        if let Some(language) = choice {
            if in_project {
                if let Err(err) = spelling::set_project_language(&self.working_dir, &language) {
                    self.show_tool_message(err);
                }
            } else {
                self.config.spelling.language = language;
                self.save_config();
            }
            self.refresh_spelling();
            ui.close();
        }
        ui.separator();
        let status = if self.spelling.is_loading() {
            format!("Loading {}…", self.spelling.language)
        } else if let Some(error) = &self.spelling.error {
            error.clone()
        } else {
            format!("Checking {}", self.spelling.language)
        };
        ui.label(RichText::new(status).small().weak());
        if let Some(dir) = spelling::user_dictionary_dir() {
            if ui
                .button("Open Dictionaries Folder")
                .on_hover_text(format!(
                    "Add Hunspell .aff and .dic files here for more languages:\n{}",
                    dir.display()
                ))
                .clicked()
            {
                if let Err(err) = std::fs::create_dir_all(&dir)
                    .and_then(|_| crate::reference_ui::open_with_system(&dir))
                {
                    self.show_tool_message(format!("Could not open {}: {err}", dir.display()));
                }
                ui.close();
            }
        }
    }
}
