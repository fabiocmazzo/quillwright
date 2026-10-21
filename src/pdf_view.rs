//! The PDF viewer inside the Reference panel: pages stacked vertically, rendered on
//! demand (see `reference::pdf`), with page navigation and zoom.
use crate::{
    reference::pdf::{self, Key},
    theme,
};
use eframe::egui;
use egui::{
    pos2, vec2, Align, Button, Color32, CursorIcon, DragValue, FontId, Layout, Rect, RichText,
    TextureHandle, Ui,
};
use std::{
    collections::{HashMap, HashSet},
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

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub struct PdfView {
    doc: pdf::Document,
    /// Page index → (rendered width in pixels, texture).
    textures: HashMap<usize, (u32, TextureHandle)>,
    failed: HashSet<usize>,
    /// 1.0 fits the widest page to the panel.
    zoom: f32,
    current: usize,
    jump: Option<usize>,
    /// Each view scrolls on its own, so a new PDF does not open at the old one's offset.
    id: u64,
}

impl PdfView {
    /// A view of `doc` that starts at `page` (zero-based).
    pub fn new(doc: pdf::Document, page: usize) -> Self {
        let page = page.min(doc.page_sizes().len() - 1);
        PdfView {
            doc,
            textures: HashMap::new(),
            failed: HashSet::new(),
            zoom: 1.0,
            current: page,
            jump: (page > 0).then_some(page),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
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
        if let Some(page) = self.jump.take() {
            area = area.vertical_scroll_offset(tops[page.min(count - 1)]);
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
        let finished = self.doc.exchange(|done| {
            missing
                .into_iter()
                .filter(|key| !done.iter().any(|r| r.key.page == key.page))
                .collect()
        });
        if !finished.is_empty() {
            // This frame already painted placeholders; show the new pages right away.
            ctx.request_repaint();
        }
        for rendered in finished {
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
        let current = self.current;
        self.textures
            .retain(|&page, _| page + KEEP >= current && page <= current + KEEP);
        if self.doc.busy() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, p: &theme::Palette, count: usize) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Page").size(12.5).color(p.muted));
            let mut page = self.current + 1;
            if ui
                .add(DragValue::new(&mut page).range(1..=count).speed(0.1))
                .on_hover_text("Type a page number, or drag")
                .changed()
            {
                self.jump = Some(page.saturating_sub(1));
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
}

fn quiet(ui: &mut Ui, text: &str, p: &theme::Palette) -> egui::Response {
    ui.add(Button::new(RichText::new(text).size(13.0).color(p.muted)).frame(false))
        .on_hover_cursor(CursorIcon::PointingHand)
}
