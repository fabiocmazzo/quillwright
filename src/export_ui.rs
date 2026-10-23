//! The Export Manuscript window.
use crate::i18n::t;
use crate::{
    export::{self, Format, PageSize},
    theme, MarkdownApp,
};
use eframe::egui;
use egui::{Context, RichText};
use rfd::FileDialog;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

#[derive(Default)]
pub struct ExportState {
    open: bool,
    title: String,
    author: String,
    format: Format,
    language: String,
    page_size: PageSize,
    table_of_contents: bool,
    /// The export running on a background thread.
    running: Option<mpsc::Receiver<Outcome>>,
    outcome: Option<Outcome>,
}

enum Outcome {
    Done {
        path: PathBuf,
        warnings: Vec<String>,
    },
    Failed {
        error: String,
    },
}

/// Write the file next to its destination, then move it into place, so a
/// failed export never leaves half a file behind.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::Builder::new()
        .prefix(".quillwright-export-")
        .tempfile_in(dir)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

impl MarkdownApp {
    pub(crate) fn open_export(&mut self) {
        let config = &self.config.export;
        let key = self.working_dir.display().to_string();
        let state = &mut self.export;
        state.open = true;
        state.title = config
            .titles
            .get(&key)
            .cloned()
            .unwrap_or_else(|| export::title_from_folder(&self.working_dir));
        state.author = config.author.clone();
        state.format = config.format;
        state.language = config.language.clone();
        state.page_size = config.page_size;
        state.table_of_contents = config.table_of_contents;
        if state.running.is_none() {
            state.outcome = None;
        }
    }

    pub(crate) fn show_export_window(&mut self, ctx: &Context) {
        self.poll_export();
        if !self.export.open {
            return;
        }
        let p = theme::current(ctx);
        let chapters = self.index.documents.len();
        let kind = self.kind;
        let open_words = self
            .file_path
            .as_deref()
            .map(|path| (path, self.current_words));
        let words = self.index.manuscript_words(open_words);
        let project = self.working_dir.clone();

        let mut open = true;
        let mut start = false;
        let mut reveal = None;
        let state = &mut self.export;
        egui::Window::new(t!("export-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .max_width(420.0)
            .show(ctx, |ui| {
                egui::Grid::new("export_fields")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(t!("export-book-title"));
                        ui.text_edit_singleline(&mut state.title);
                        ui.end_row();
                        ui.label(t!("export-author"));
                        ui.text_edit_singleline(&mut state.author);
                        ui.end_row();
                        ui.label(t!("export-language"));
                        ui.add(
                            egui::TextEdit::singleline(&mut state.language)
                                .hint_text("en, pt-BR, es…"),
                        )
                        .on_hover_text(t!("export-language.hint"));
                        ui.end_row();
                    });
                ui.add_space(8.0);

                for format in Format::ALL {
                    if ui
                        .radio(state.format == format, format.label())
                        .on_hover_text(format.hint())
                        .clicked()
                    {
                        state.format = format;
                    }
                }
                if state.format.has_pages() {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(t!("export-page-size"));
                        egui::ComboBox::from_id_salt("export_page_size")
                            .selected_text(state.page_size.label())
                            .show_ui(ui, |ui| {
                                for size in PageSize::ALL {
                                    ui.selectable_value(&mut state.page_size, size, size.label());
                                }
                            });
                    });
                }
                ui.add_space(4.0);
                ui.checkbox(&mut state.table_of_contents, t!("export-toc"));
                ui.add_space(8.0);

                let mut notes = vec![t!(
                    "export-summary",
                    documents = kind.documents.count(chapters),
                    words = crate::workspace::words(words)
                )];
                if project.join(export::METADATA_FILE).is_file() {
                    notes.push(t!("export-uses", file = export::METADATA_FILE));
                }
                let reference = match state.format {
                    Format::Docx => Some(export::REFERENCE_DOCX),
                    Format::Odt => Some(export::REFERENCE_ODT),
                    _ => None,
                };
                if let Some(name) = reference.filter(|n| project.join(n).is_file()) {
                    notes.push(t!("export-styles", file = name));
                }
                for note in notes {
                    ui.label(RichText::new(note).size(12.5).color(p.muted));
                }
                ui.add_space(8.0);

                if state.running.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(t!("export-running"));
                    });
                } else if ui
                    .add_enabled(chapters > 0, egui::Button::new(t!("export-start")))
                    .on_disabled_hover_text(t!("export-nothing"))
                    .clicked()
                {
                    start = true;
                }

                match &state.outcome {
                    Some(Outcome::Done { path, warnings }) => {
                        ui.separator();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        ui.label(
                            RichText::new(t!("export-done", file = name.as_ref())).color(p.accent),
                        );
                        ui.horizontal(|ui| {
                            if ui.button(t!("export-open")).clicked() {
                                reveal = Some(path.clone());
                            }
                            if ui.button(t!("export-show")).clicked() {
                                reveal = path.parent().map(PathBuf::from);
                            }
                        });
                        if !warnings.is_empty() {
                            egui::ScrollArea::vertical()
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    for warning in warnings {
                                        ui.label(RichText::new(warning).size(12.0).color(p.muted));
                                    }
                                });
                        }
                    }
                    Some(Outcome::Failed { error }) => {
                        ui.separator();
                        ui.colored_label(p.accent, error);
                    }
                    None => {}
                }
            });

        if !open {
            self.export.open = false;
        }
        if let Some(path) = reveal {
            if let Err(err) = export::open_with_system(&path) {
                self.export.outcome = Some(Outcome::Failed {
                    error: t!(
                        "open-failed",
                        path = path.display().to_string(),
                        error = err.to_string()
                    ),
                });
            }
        }
        if start {
            self.start_export(ctx);
        }
    }

    fn start_export(&mut self, ctx: &Context) {
        let format = self.export.format;
        let title = self.export.title.trim().to_string();
        let project = self.working_dir.clone();

        // Remember the choices for next time.
        let config = &mut self.config.export;
        config.format = format;
        config.author = self.export.author.trim().to_string();
        config.language = self.export.language.trim().to_string();
        config.page_size = self.export.page_size;
        config.table_of_contents = self.export.table_of_contents;
        config
            .titles
            .insert(project.display().to_string(), title.clone());
        let last_dir = config.last_directory.clone();
        self.save_config();

        let Some(mut output) = FileDialog::new()
            .set_directory(export::default_directory(&project, last_dir.as_ref()))
            .set_file_name(&export::default_file_name(&title, format))
            .add_filter(&format.label(), &[format.extension()])
            .save_file()
        else {
            return;
        };
        if output.extension().is_none() {
            output.set_extension(format.extension());
        }
        self.config.export.last_directory = output.parent().map(PathBuf::from);
        self.save_config();

        // The open chapter is exported as it is in the editor, saved or not.
        let sources: Vec<export::Source> = self
            .index
            .documents
            .iter()
            .map(|entry| export::Source {
                title: entry.title.clone(),
                text: if self.file_path.as_ref() == Some(&entry.path) {
                    self.content.clone()
                } else {
                    std::fs::read_to_string(&entry.path).unwrap_or_default()
                },
            })
            .collect();
        let options = export::Options {
            format,
            title,
            author: self.config.export.author.clone(),
            language: self.config.export.language.clone(),
            page_size: self.export.page_size,
            table_of_contents: self.export.table_of_contents,
        };

        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = match export::export(&options, &sources, &project) {
                Ok(out) => match write_atomically(&output, &out.bytes) {
                    Ok(()) => Outcome::Done {
                        path: output,
                        warnings: out.warnings,
                    },
                    Err(err) => Outcome::Failed {
                        error: t!(
                            "export-save-failed",
                            path = output.display().to_string(),
                            error = err.to_string()
                        ),
                    },
                },
                Err(error) => Outcome::Failed { error },
            };
            let _ = send.send(outcome);
            ctx.request_repaint();
        });
        self.export.running = Some(receive);
        self.export.outcome = None;
    }

    fn poll_export(&mut self) {
        let Some(running) = &self.export.running else {
            return;
        };
        let outcome = match running.try_recv() {
            Ok(outcome) => outcome,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Outcome::Failed {
                error: t!("export-stopped"),
            },
        };
        self.export.outcome = Some(outcome);
        self.export.running = None;
        // The export may have been written inside the project.
        self.index_dirty = true;
    }
}
