//! The Export Manuscript window.
use crate::{
    assistant::Job,
    export::{self, Format},
    manuscript, theme, MarkdownApp,
};
use eframe::egui;
use egui::{Context, RichText};
use rfd::FileDialog;
use std::io::Write;
use std::path::PathBuf;

#[derive(Default)]
pub struct ExportState {
    open: bool,
    title: String,
    author: String,
    format: Format,
    table_of_contents: bool,
    /// `pandoc --version`, or None if pandoc could not be run.
    pandoc: Option<String>,
    engines: Vec<String>,
    running: Option<Running>,
    outcome: Option<Outcome>,
}

struct Running {
    job: Job,
    output: PathBuf,
    /// Deleted when the export finishes.
    _filter: tempfile::NamedTempFile,
}

enum Outcome {
    Done { path: PathBuf, warnings: String },
    Failed { error: String, details: String },
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
        state.table_of_contents = config.table_of_contents;
        state.pandoc = export::pandoc_version(&config.pandoc);
        state.engines = export::installed_pdf_engines();
        if state.running.is_none() {
            state.outcome = None;
        }
    }

    fn pdf_engine(&self) -> Option<String> {
        self.config
            .export
            .pdf_engine
            .clone()
            .or_else(|| self.export.engines.first().cloned())
    }

    pub(crate) fn show_export_window(&mut self, ctx: &Context) {
        self.poll_export();
        if !self.export.open {
            return;
        }
        let p = theme::current(ctx);
        let chapters = self.index.chapters.len();
        let open_words = self
            .file_path
            .as_deref()
            .map(|path| (path, self.current_words));
        let words = self.index.manuscript_words(open_words);
        let pdf_engine = self.pdf_engine();
        let project = self.working_dir.clone();

        let mut open = true;
        let mut start = false;
        let mut cancel = false;
        let mut reveal = None;
        let state = &mut self.export;
        egui::Window::new("Export Manuscript")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .max_width(420.0)
            .show(ctx, |ui| {
                let Some(version) = &state.pandoc else {
                    ui.label("Exporting needs pandoc, a free document converter.");
                    ui.hyperlink_to("Install pandoc", "https://pandoc.org/installing.html");
                    ui.label(
                        RichText::new(
                            "Linux: install the pandoc package (pandoc-cli on Arch). \
                             macOS: brew install pandoc. Then open this window again. \
                             If pandoc is not on your PATH, set [export] pandoc in the config file.",
                        )
                        .size(12.5)
                        .color(p.muted),
                    );
                    return;
                };

                egui::Grid::new("export_fields")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Title");
                        ui.text_edit_singleline(&mut state.title);
                        ui.end_row();
                        ui.label("Author");
                        ui.text_edit_singleline(&mut state.author);
                        ui.end_row();
                    });
                ui.add_space(8.0);

                for format in Format::ALL {
                    let available = format != Format::Pdf || pdf_engine.is_some();
                    ui.add_enabled_ui(available, |ui| {
                        let response = ui
                            .radio(state.format == format, format.label())
                            .on_hover_text(format.hint())
                            .on_disabled_hover_text(
                                "PDF needs a PDF engine: install typst (small and fast) \
                                 or a LaTeX distribution.",
                            );
                        if response.clicked() {
                            state.format = format;
                        }
                    });
                }
                if state.format == Format::Pdf && pdf_engine.is_none() {
                    state.format = Format::Docx;
                }
                if state.format == Format::Pdf {
                    if let Some(engine) = &pdf_engine {
                        ui.label(
                            RichText::new(format!("Using {engine}"))
                                .size(12.5)
                                .color(p.muted),
                        );
                    }
                }
                ui.add_space(4.0);
                ui.checkbox(&mut state.table_of_contents, "Table of contents");
                ui.add_space(8.0);

                let mut notes = vec![format!(
                    "{chapters} chapter{} · {} words, in sidebar order",
                    if chapters == 1 { "" } else { "s" },
                    manuscript::group(words)
                )];
                if project.join(export::METADATA_FILE).is_file() {
                    notes.push(format!("Uses {}", export::METADATA_FILE));
                }
                let reference = match state.format {
                    Format::Docx => Some(export::REFERENCE_DOCX),
                    Format::Odt => Some(export::REFERENCE_ODT),
                    _ => None,
                };
                if let Some(name) = reference.filter(|n| project.join(n).is_file()) {
                    notes.push(format!("Styles from {name}"));
                }
                notes.push(version.clone());
                for note in notes {
                    ui.label(RichText::new(note).size(12.5).color(p.muted));
                }
                ui.add_space(8.0);

                if state.running.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Exporting…");
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                } else if ui
                    .add_enabled(chapters > 0, egui::Button::new("Export…"))
                    .on_disabled_hover_text("No chapters found in the chapters folder")
                    .clicked()
                {
                    start = true;
                }

                match &state.outcome {
                    Some(Outcome::Done { path, warnings }) => {
                        ui.separator();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        ui.label(RichText::new(format!("Exported {name}")).color(p.accent));
                        ui.horizontal(|ui| {
                            if ui.button("Open").clicked() {
                                reveal = Some(path.clone());
                            }
                            if ui.button("Show in Folder").clicked() {
                                reveal = path.parent().map(PathBuf::from);
                            }
                        });
                        if !warnings.trim().is_empty() {
                            ui.collapsing("Pandoc warnings", |ui| {
                                ui.label(RichText::new(warnings.trim()).size(12.0).color(p.muted));
                            });
                        }
                    }
                    Some(Outcome::Failed { error, details }) => {
                        ui.separator();
                        ui.colored_label(p.accent, error);
                        if !details.trim().is_empty() {
                            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                                ui.label(RichText::new(details.trim()).size(12.0).color(p.muted));
                            });
                        }
                    }
                    None => {}
                }
            });

        if !open {
            self.export.open = false;
        }
        if cancel {
            if let Some(running) = &self.export.running {
                running.job.cancel();
            }
        }
        if let Some(path) = reveal {
            if let Err(err) = export::open_with_system(&path) {
                self.export.outcome = Some(Outcome::Failed {
                    error: format!("Could not open {}: {err}", path.display()),
                    details: String::new(),
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
        config.table_of_contents = self.export.table_of_contents;
        config
            .titles
            .insert(project.display().to_string(), title.clone());
        let last_dir = config.last_directory.clone();
        self.save_config();

        let Some(mut output) = FileDialog::new()
            .set_directory(export::default_directory(&project, last_dir.as_ref()))
            .set_file_name(&export::default_file_name(&title, format))
            .add_filter(format.label(), &[format.extension()])
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
        let chapters: Vec<export::Chapter> = self
            .index
            .chapters
            .iter()
            .map(|entry| export::Chapter {
                title: entry.title.clone(),
                text: if self.file_path.as_ref() == Some(&entry.path) {
                    self.content.clone()
                } else {
                    std::fs::read_to_string(&entry.path).unwrap_or_default()
                },
            })
            .collect();
        let book = export::assemble(&chapters);

        let result = (|| -> std::io::Result<Running> {
            let mut filter = tempfile::Builder::new()
                .prefix("quillwright-export-")
                .suffix(".lua")
                .tempfile()?;
            filter.write_all(export::FILTER.as_bytes())?;
            filter.flush()?;
            let options = export::Options {
                format,
                title,
                author: self.config.export.author.clone(),
                table_of_contents: self.export.table_of_contents,
                pdf_engine: self.pdf_engine(),
            };
            let mut command = vec![self.config.export.pandoc.clone()];
            command.extend(export::arguments(
                &options,
                &output,
                filter.path(),
                &project,
            ));
            let job = Job::run(&command, &project, Some(book), ctx.clone())?;
            Ok(Running {
                job,
                output: output.clone(),
                _filter: filter,
            })
        })();
        match result {
            Ok(running) => {
                self.export.running = Some(running);
                self.export.outcome = None;
            }
            Err(err) => {
                self.export.outcome = Some(Outcome::Failed {
                    error: format!("Could not start pandoc: {err}"),
                    details: String::new(),
                })
            }
        }
    }

    fn poll_export(&mut self) {
        let Some(running) = &mut self.export.running else {
            return;
        };
        running.job.poll();
        let Some(result) = running.job.finished.clone() else {
            return;
        };
        let details = running.job.stderr.clone();
        self.export.outcome = Some(match result {
            Ok(()) => Outcome::Done {
                path: running.output.clone(),
                warnings: details,
            },
            Err(error) => Outcome::Failed {
                error: format!("Export failed: {error}"),
                details,
            },
        });
        self.export.running = None;
        // The export may have been written inside the project.
        self.index_dirty = true;
    }
}
