//! The Reference side panel: research files shown read only beside the page.
//! See docs/reference-material.md.
use crate::{
    fonts, live_preview, manuscript,
    pdf_view::PdfView,
    process,
    reference::{self, Content, Kind, Loaded, Stamp},
    theme, DiscardAction, MarkdownApp,
};
use eframe::egui;
use egui::{Align, Button, Context, CursorIcon, FontId, Layout, RichText, TextureHandle, Ui};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

pub struct ReferenceState {
    pub open: bool,
    path: Option<PathBuf>,
    stamp: Option<Stamp>,
    shown: Option<Shown>,
    loading: Option<mpsc::Receiver<Loaded>>,
    error: Option<String>,
    /// Images: fit the panel width (true) or show at 100 %.
    fit_width: bool,
    /// A search result to show once the PDF is open: (query, page, character index).
    pending_find: Option<(String, usize, usize)>,
}

impl Default for ReferenceState {
    fn default() -> Self {
        ReferenceState {
            open: false,
            path: None,
            stamp: None,
            shown: None,
            loading: None,
            error: None,
            fit_width: true,
            pending_find: None,
        }
    }
}

impl ReferenceState {
    /// The reference shown in the panel, if the panel is open.
    pub fn showing(&self) -> Option<&Path> {
        self.path.as_deref().filter(|_| self.open)
    }

    pub fn has_file(&self) -> bool {
        self.path.is_some()
    }
}

enum Shown {
    Text(String),
    Markdown(String),
    Image(TextureHandle),
    Pdf(Box<PdfView>),
    Unsupported(String),
}

enum PanelAction {
    Close,
    Edit(PathBuf),
    External(PathBuf),
    ToggleFit,
}

impl MarkdownApp {
    pub(crate) fn open_reference(&mut self, path: PathBuf) {
        let state = &mut self.reference;
        state.open = true;
        if state.path.as_ref() == Some(&path) && state.error.is_none() {
            return;
        }
        state.shown = None;
        state.error = None;
        state.fit_width = true;
        state.loading = Some(reference::load_in_background(path.clone()));
        state.path = Some(path);
    }

    /// Open a PDF at a search match (from project search).
    pub(crate) fn reveal_in_pdf(&mut self, path: PathBuf, query: String, page: usize, at: usize) {
        self.open_reference(path);
        self.reference.pending_find = Some((query, page, at));
    }

    pub(crate) fn toggle_reference(&mut self) {
        self.reference.open = !self.reference.open && self.reference.has_file();
    }

    pub(crate) fn poll_reference(&mut self, ctx: &Context) {
        let state = &mut self.reference;
        let Some(rx) = &state.loading else {
            return;
        };
        let loaded = match rx.try_recv() {
            Ok(loaded) => loaded,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(50));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.loading = None;
                state.error = Some("The file could not be read.".into());
                return;
            }
        };
        state.loading = None;
        if state.path.as_ref() != Some(&loaded.path) {
            return; // Another file was chosen meanwhile.
        }
        state.stamp = loaded.stamp;
        // A reloaded PDF opens at the page the author was reading.
        let page = match &state.shown {
            Some(Shown::Pdf(view)) => view.current_page(),
            _ => 0,
        };
        match loaded.content {
            Ok(content) => {
                state.error = None;
                state.shown = Some(match content {
                    Content::Pdf(doc) => Shown::Pdf(Box::new(PdfView::new(doc, page))),
                    Content::Text(text) => Shown::Text(text),
                    Content::Markdown(text) => Shown::Markdown(text),
                    Content::Image(image) => Shown::Image(ctx.load_texture(
                        format!("reference:{}", loaded.path.display()),
                        image,
                        egui::TextureOptions::LINEAR,
                    )),
                    Content::Unsupported(message) => Shown::Unsupported(message),
                });
            }
            Err(error) => {
                state.shown = None;
                state.error = Some(format!("This file could not be opened: {error}"));
            }
        }
    }

    /// After the project index is rebuilt: follow changes to the open reference.
    pub(crate) fn refresh_reference(&mut self) {
        let state = &mut self.reference;
        let Some(path) = state.path.clone() else {
            return;
        };
        if !path.starts_with(&self.working_dir) {
            *state = ReferenceState::default(); // Another project was opened.
            return;
        }
        if state.loading.is_some() {
            return;
        }
        if !path.is_file() {
            state.shown = None;
            state.error = Some("This file was moved or deleted.".into());
            return;
        }
        if reference::stamp(&path) != state.stamp {
            // Keep showing the old version until the new one is ready.
            state.loading = Some(reference::load_in_background(path));
        }
    }

    pub(crate) fn in_project_for_research(&self) -> bool {
        self.index.has_chapters_dir || self.index.has_research_dir
    }

    pub(crate) fn add_research_files(&mut self) {
        let Some(files) = rfd::FileDialog::new()
            .set_title("Add Research Files")
            .pick_files()
        else {
            return;
        };
        let dir = self.working_dir.join(manuscript::RESEARCH_DIR);
        match reference::add_files(&dir, &files) {
            Ok(added) => {
                self.index_dirty = true;
                if let Some(first) = added.into_iter().next() {
                    self.open_reference(first);
                }
            }
            Err(error) => {
                self.sync_notice = Some(format!("⚠ Could not add research files: {error}"));
            }
        }
    }

    pub(crate) fn show_reference_panel(&mut self, root: &mut Ui) {
        let ctx = &root.ctx().clone();
        let p = theme::current(ctx);
        let Some(path) = self.reference.path.clone() else {
            self.reference.open = false;
            return;
        };
        let kind = Kind::of(&path);
        let mut action = None;
        egui::Panel::right("reference")
            .resizable(true)
            .default_size(380.0)
            .size_range(260.0..=900.0)
            .frame(
                egui::Frame::NONE
                    .fill(p.desk)
                    .inner_margin(egui::Margin::symmetric(14, 12)),
            )
            .show(root, |ui| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if quiet(ui, "Close", &p).clicked() {
                            action = Some(PanelAction::Close);
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
                            .on_hover_text(path.display().to_string());
                        });
                    });
                });
                let relative = path.strip_prefix(&self.working_dir).unwrap_or(&path);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{} · read only", relative.display()))
                            .size(12.0)
                            .color(p.muted),
                    );
                });
                ui.horizontal(|ui| {
                    if kind.editable()
                        && quiet(ui, "Edit", &p)
                            .on_hover_text("Open this file in the editor")
                            .clicked()
                    {
                        action = Some(PanelAction::Edit(path.clone()));
                    }
                    if matches!(self.reference.shown, Some(Shown::Image(_))) {
                        let label = if self.reference.fit_width {
                            "Actual Size"
                        } else {
                            "Fit Width"
                        };
                        if quiet(ui, label, &p).clicked() {
                            action = Some(PanelAction::ToggleFit);
                        }
                    }
                    if quiet(ui, "Open with System Viewer", &p).clicked() {
                        action = Some(PanelAction::External(path.clone()));
                    }
                });
                ui.add_space(6.0);
                ui.separator();
                self.reference_body(ui, &p, &mut action);
            });
        match action {
            None => {}
            Some(PanelAction::Close) => self.reference.open = false,
            Some(PanelAction::ToggleFit) => self.reference.fit_width ^= true,
            Some(PanelAction::Edit(path)) => {
                self.reference.open = false;
                if self.file_path.as_ref() != Some(&path) {
                    self.run_or_prompt_discard(DiscardAction::OpenPath(path));
                }
            }
            Some(PanelAction::External(path)) => {
                if let Err(error) = open_with_system(&path) {
                    self.sync_notice = Some(format!("⚠ Could not open the file: {error}"));
                }
            }
        }
    }

    fn reference_body(
        &mut self,
        ui: &mut Ui,
        p: &theme::Palette,
        action: &mut Option<PanelAction>,
    ) {
        let state = &mut self.reference;
        let fit = state.fit_width;
        if let Some(error) = &state.error {
            ui.add_space(8.0);
            ui.label(RichText::new(error).color(p.muted));
            return;
        }
        let Some(shown) = &mut state.shown else {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Loading…").color(p.muted));
            });
            return;
        };
        match shown {
            Shown::Text(text) => {
                egui::ScrollArea::vertical()
                    .id_salt("reference_text")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(text.as_str())
                                    .font(FontId::new(15.0, fonts::family(false, false)))
                                    .color(p.text),
                            )
                            .selectable(true)
                            .wrap(),
                        );
                    });
            }
            Shown::Markdown(text) => {
                egui::ScrollArea::vertical()
                    .id_salt("reference_markdown")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        let projection = live_preview::project(ui, text, width, None);
                        ui.add(egui::Label::new(projection.job).selectable(true));
                    });
            }
            Shown::Image(texture) => {
                let size = texture.size_vec2();
                egui::ScrollArea::both()
                    .id_salt("reference_image")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let scale = if fit {
                            (ui.available_width() / size.x).min(1.0)
                        } else {
                            1.0
                        };
                        let image = egui::Image::new(egui::load::SizedTexture::new(
                            texture.id(),
                            size * scale,
                        ))
                        .sense(egui::Sense::click());
                        let hint = if fit {
                            "Click for actual size"
                        } else {
                            "Click to fit the panel"
                        };
                        let response = ui
                            .add(image)
                            .on_hover_text(format!("{} × {} px. {hint}", size.x, size.y))
                            .on_hover_cursor(CursorIcon::PointingHand);
                        if response.clicked() {
                            *action = Some(PanelAction::ToggleFit);
                        }
                    });
            }
            Shown::Pdf(view) => {
                if let Some((query, page, at)) = state.pending_find.take() {
                    view.find_at(query, page, at);
                }
                view.show(ui, p)
            }
            Shown::Unsupported(message) => {
                ui.add_space(8.0);
                ui.label(RichText::new(message.as_str()).color(p.muted));
            }
        }
    }
}

fn quiet(ui: &mut Ui, text: &str, p: &theme::Palette) -> egui::Response {
    ui.add(Button::new(RichText::new(text).size(12.5).color(p.muted)).frame(false))
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// Open a file or folder with the operating system's default program.
pub(crate) fn open_with_system(path: &Path) -> std::io::Result<()> {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    process::command(program).arg(path).spawn().map(|_| ())
}
