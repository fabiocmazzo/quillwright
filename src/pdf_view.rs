//! The PDF viewer inside the Reference panel: pages stacked vertically, rendered on
//! demand (see `reference::pdf`), with page navigation, zoom, and search.
use crate::{
    reference::{
        pdf::{self, Key},
        text::{self, Folded, PageText},
    },
    theme,
};
use eframe::egui;
use egui::{
    pos2, vec2, Align, Button, Color32, CursorIcon, DragValue, FontId, Layout, Rect, RichText,
    TextEdit, TextureHandle, Ui,
};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

/// Space between pages, in points.
const GAP: f32 = 12.0;
/// Textures are kept for pages this close to the current one.
const KEEP: usize = 6;
/// Rendered widths are rounded up to a multiple of this, so resizing the panel by a few
/// pixels does not render every page again.
const WIDTH_STEP: u32 = 128;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 4.0;
/// Searching stops collecting matches after this many.
const MAX_MATCHES: usize = 5000;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Match {
    page: usize,
    range: Range<usize>,
}

pub struct PdfView {
    doc: pdf::Document,
    /// Page index → (rendered width in pixels, texture).
    textures: HashMap<usize, (u32, TextureHandle)>,
    failed: HashSet<usize>,
    /// 1.0 fits the widest page to the panel.
    zoom: f32,
    current: usize,
    /// Scroll to this page, and this far down it (in PDF points).
    jump: Option<(usize, f32)>,
    /// Each view scrolls on its own, so a new PDF does not open at the old one's offset.
    id: u64,
    texts: Vec<Option<PageText>>,
    folded: Vec<Option<Folded>>,
    query: String,
    folded_query: Vec<char>,
    /// Sorted by page, then position.
    matches: Vec<Match>,
    selected: Option<usize>,
    focus_search: bool,
}

impl PdfView {
    /// A view of `doc` that starts at `page` (zero-based).
    pub fn new(doc: pdf::Document, page: usize) -> Self {
        let count = doc.page_sizes().len();
        let page = page.min(count - 1);
        PdfView {
            doc,
            textures: HashMap::new(),
            failed: HashSet::new(),
            zoom: 1.0,
            current: page,
            jump: (page > 0).then_some((page, 0.0)),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            texts: vec![None; count],
            folded: vec![None; count],
            query: String::new(),
            folded_query: Vec::new(),
            matches: Vec::new(),
            selected: None,
            focus_search: false,
        }
    }

    pub fn current_page(&self) -> usize {
        self.current
    }

    pub fn show(&mut self, ui: &mut Ui, p: &theme::Palette) {
        let ctx = ui.ctx().clone();
        let sizes = self.doc.page_sizes().to_vec();
        let count = sizes.len();
        self.toolbar(ui, p, count);
        self.search_bar(ui, p, count);
        ui.add_space(4.0);

        let widest = sizes.iter().map(|s| s[0]).fold(1.0, f32::max);
        let scrollbar = ui.spacing().scroll.allocated_width() + 4.0;
        let available = (ui.available_width() - scrollbar).max(60.0);
        let scale = available * self.zoom / widest;
        let mut tops = Vec::with_capacity(count);
        let mut y = 0.0;
        for size in &sizes {
            tops.push(y);
            y += size[1] * scale + GAP;
        }
        let total = y - GAP;
        let content_width = (widest * scale).max(available);

        let mut area = egui::ScrollArea::both()
            .id_salt(("reference_pdf", self.id))
            .auto_shrink([false; 2]);
        if let Some((page, within)) = self.jump.take() {
            // Put the target a third of the way down the view.
            let lead = if within > 0.0 {
                ui.available_height() / 3.0
            } else {
                0.0
            };
            let offset = tops[page.min(count - 1)] + within * scale - lead;
            area = area.vertical_scroll_offset(offset.max(0.0));
        }
        let pixels_per_point = ctx.pixels_per_point();
        let mut wanted = Vec::new();
        area.show_viewport(ui, |ui, viewport| {
            let origin = ui.max_rect().min;
            ui.set_min_size(vec2(content_width, total));
            let first = tops
                .partition_point(|&top| top <= viewport.min.y)
                .saturating_sub(1);
            let mut end = first;
            while end < count && tops[end] < viewport.max.y {
                end += 1;
            }
            let reading_line = viewport.min.y + viewport.height() / 3.0;
            self.current = tops
                .partition_point(|&top| top <= reading_line)
                .saturating_sub(1);

            let painter = ui.painter();
            for (page, size) in sizes.iter().enumerate().take(end).skip(first) {
                let size = vec2(size[0] * scale, size[1] * scale);
                let min = pos2(
                    origin.x + (content_width - size.x) / 2.0,
                    origin.y + tops[page],
                );
                let rect = Rect::from_min_size(min, size);
                painter.rect_filled(rect.expand(1.0), 0.0, theme::mix(p.desk, p.text, 0.15));
                painter.rect_filled(rect, 0.0, Color32::WHITE);
                if let Some((_, texture)) = self.textures.get(&page) {
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    painter.image(texture.id(), rect, uv, Color32::WHITE);
                } else {
                    let text = if self.failed.contains(&page) {
                        "This page could not be shown.".to_string()
                    } else {
                        format!("Page {}", page + 1)
                    };
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        FontId::proportional(13.0),
                        Color32::from_gray(140),
                    );
                }
                self.paint_matches(painter, page, min, scale);
            }

            // Visible pages first, then one on each side, so scrolling finds them ready.
            let neighbors = [first.checked_sub(1), (end < count).then_some(end)];
            for page in (first..end).chain(neighbors.into_iter().flatten()) {
                let pixels = (sizes[page][0] * scale * pixels_per_point).ceil() as u32;
                let width = pixels.div_ceil(WIDTH_STEP) * WIDTH_STEP;
                wanted.push(Key {
                    page,
                    width: width.clamp(WIDTH_STEP, pdf::MAX_SIDE),
                });
            }
        });

        let textures = &self.textures;
        let failed = &self.failed;
        let missing: Vec<Key> = wanted
            .into_iter()
            .filter(|key| {
                !failed.contains(&key.page)
                    && textures.get(&key.page).map(|(width, _)| *width) != Some(key.width)
            })
            .collect();
        let update = self.doc.exchange(|done| {
            missing
                .into_iter()
                .filter(|key| !done.iter().any(|r| r.key.page == key.page))
                .collect()
        });
        if !update.rendered.is_empty() || (!update.texts.is_empty() && !self.query.is_empty()) {
            // This frame already painted; show the new pages and matches right away.
            ctx.request_repaint();
        }
        for rendered in update.rendered {
            let page = rendered.key.page;
            match rendered.image {
                Some(image) => {
                    let texture = ctx.load_texture(
                        format!("reference-pdf-page-{page}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.textures.insert(page, (rendered.key.width, texture));
                }
                None => {
                    self.failed.insert(page);
                }
            }
        }
        self.receive_texts(update.texts);
        let current = self.current;
        self.textures
            .retain(|&page, _| page + KEEP >= current && page <= current + KEEP);
        if self.doc.busy() {
            ctx.request_repaint_after(Duration::from_millis(30));
        } else if !self.query.is_empty() && self.doc.text_pages() < count {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }

    fn paint_matches(&self, painter: &egui::Painter, page: usize, min: egui::Pos2, scale: f32) {
        let Some(page_text) = &self.texts[page] else {
            return;
        };
        let start = self.matches.partition_point(|m| m.page < page);
        for (i, m) in self.matches.iter().enumerate().skip(start) {
            if m.page != page {
                break;
            }
            let color = if self.selected == Some(i) {
                Color32::from_rgba_unmultiplied(255, 120, 0, 120)
            } else {
                Color32::from_rgba_unmultiplied(255, 205, 0, 100)
            };
            for r in text::highlight(page_text, m.range.clone()) {
                let rect = Rect::from_min_max(
                    min + vec2(r[0] * scale, r[1] * scale),
                    min + vec2(r[2] * scale, r[3] * scale),
                );
                painter.rect_filled(rect.expand(1.0), 2.0, color);
            }
        }
    }

    // ----- Search --------------------------------------------------------

    fn receive_texts(&mut self, texts: Vec<(usize, PageText)>) {
        for (page, page_text) in texts {
            let Some(slot) = self.texts.get_mut(page) else {
                continue;
            };
            let folded = text::fold(&page_text.text);
            *slot = Some(page_text);
            if !self.folded_query.is_empty() && self.matches.len() < MAX_MATCHES {
                let at = self.matches.partition_point(|m| m.page < page);
                let found: Vec<Match> = text::find(&folded, &self.folded_query)
                    .into_iter()
                    .map(|range| Match { page, range })
                    .collect();
                let added = found.len();
                self.matches.splice(at..at, found);
                match self.selected {
                    Some(selected) if selected >= at => self.selected = Some(selected + added),
                    None if added > 0 => {
                        // The first matches have arrived while the author waits.
                        self.selected = Some(at);
                        self.jump_to_selected();
                    }
                    _ => {}
                }
            }
            self.folded[page] = Some(folded);
        }
    }

    fn run_search(&mut self) {
        self.folded_query = text::fold_query(&self.query);
        self.matches.clear();
        self.selected = None;
        if self.folded_query.is_empty() {
            return;
        }
        for (page, folded) in self.folded.iter().enumerate() {
            let Some(folded) = folded else {
                continue;
            };
            for range in text::find(folded, &self.folded_query) {
                self.matches.push(Match { page, range });
            }
            if self.matches.len() >= MAX_MATCHES {
                break;
            }
        }
        // Start from the page being read, not from the beginning of the book.
        if !self.matches.is_empty() {
            let from_here = self.matches.partition_point(|m| m.page < self.current);
            self.selected = Some(from_here % self.matches.len());
            self.jump_to_selected();
        }
    }

    fn step(&mut self, forward: bool) {
        let count = self.matches.len();
        if count == 0 {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) if forward => (i + 1) % count,
            Some(i) => (i + count - 1) % count,
        });
        self.jump_to_selected();
    }

    fn jump_to_selected(&mut self) {
        let Some(m) = self.selected.and_then(|i| self.matches.get(i)) else {
            return;
        };
        let top = self.texts[m.page]
            .as_ref()
            .and_then(|t| text::highlight(t, m.range.clone()).first().map(|r| r[1]))
            .unwrap_or(0.0);
        self.jump = Some((m.page, top.max(0.01)));
    }

    fn search_status(&self, count: usize) -> String {
        let ready = self.doc.text_pages();
        let reading = ready < count;
        if self.folded_query.is_empty() {
            return String::new();
        }
        let searched_text = self.texts.iter().flatten().any(|t| !t.is_empty());
        if !reading && !searched_text {
            return "This PDF has no text to search.".into();
        }
        let more = if reading {
            format!(" · reading {}%", ready * 100 / count)
        } else {
            String::new()
        };
        match (self.selected, self.matches.len()) {
            (_, 0) => format!("No matches{more}"),
            (Some(i), n) => {
                let plus = if n >= MAX_MATCHES { "+" } else { "" };
                format!("{} of {n}{plus}{more}", i + 1)
            }
            (None, n) => format!("{n} matches{more}"),
        }
    }

    // ----- Toolbars ------------------------------------------------------

    fn toolbar(&mut self, ui: &mut Ui, p: &theme::Palette, count: usize) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Page").size(12.5).color(p.muted));
            let mut page = self.current + 1;
            if ui
                .add(DragValue::new(&mut page).range(1..=count).speed(0.1))
                .on_hover_text("Type a page number, or drag")
                .changed()
            {
                self.jump = Some((page.saturating_sub(1), 0.0));
            }
            ui.label(
                RichText::new(format!("of {count}"))
                    .size(12.5)
                    .color(p.muted),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if quiet(ui, "+", p).on_hover_text("Zoom in").clicked() {
                    self.zoom = (self.zoom * 1.25).min(MAX_ZOOM);
                }
                let percent = format!("{:.0}%", self.zoom * 100.0);
                if quiet(ui, &percent, p)
                    .on_hover_text("Fit the page to the panel")
                    .clicked()
                {
                    self.zoom = 1.0;
                }
                if quiet(ui, "−", p).on_hover_text("Zoom out").clicked() {
                    self.zoom = (self.zoom / 1.25).max(MIN_ZOOM);
                }
            });
        });
    }

    fn search_bar(&mut self, ui: &mut Ui, p: &theme::Palette, count: usize) {
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 64.0).max(80.0);
            let response = ui.add(
                TextEdit::singleline(&mut self.query)
                    .hint_text("Find in this PDF")
                    .desired_width(width),
            );
            if std::mem::take(&mut self.focus_search) {
                response.request_focus();
            }
            if response.changed() {
                self.run_search();
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let back = ui.input(|i| i.modifiers.shift);
                self.step(!back);
                self.focus_search = true;
            }
            let any = !self.matches.is_empty();
            if ui
                .add_enabled(any, Button::new("‹").frame(false))
                .on_hover_text("Previous match (Shift+Enter)")
                .clicked()
            {
                self.step(false);
            }
            if ui
                .add_enabled(any, Button::new("›").frame(false))
                .on_hover_text("Next match (Enter)")
                .clicked()
            {
                self.step(true);
            }
        });
        let status = self.search_status(count);
        if !status.is_empty() {
            ui.label(RichText::new(status).size(12.0).color(p.muted));
        }
    }
}

fn quiet(ui: &mut Ui, text: &str, p: &theme::Palette) -> egui::Response {
    ui.add(Button::new(RichText::new(text).size(13.0).color(p.muted)).frame(false))
        .on_hover_cursor(CursorIcon::PointingHand)
}
