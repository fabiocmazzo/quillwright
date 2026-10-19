//! A source-backed editor with a separate, mapped Markdown display projection.
use crate::live_preview::{self, Projection};
use egui::{
    text::{CCursor, CCursorRange, CursorRange},
    text_selection::{visuals, TextCursorState},
    Event, Key, TextStyle, Ui,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub line_numbers: bool,
    pub max_width: f32,
    pub centered: bool,
    /// Keep the line being written vertically centered, like a typewriter.
    pub typewriter: bool,
    /// Fade every paragraph except the one with the cursor.
    pub dim_paragraphs: bool,
    /// Save the open file automatically after a pause in typing.
    pub autosave: bool,
    /// Seconds without typing before autosave writes the file.
    pub autosave_delay: f32,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_family: "Literata".into(),
            font_size: 18.0,
            line_height: 1.4,
            line_numbers: false,
            max_width: 760.0,
            centered: true,
            typewriter: false,
            dim_paragraphs: false,
            autosave: true,
            autosave_delay: 2.0,
        }
    }
}

/// Space above the first line, so the page does not start against the menu bar.
const TOP_MARGIN: f32 = 40.0;

// Width applies only to text. Reserve space outside it for padding and gutter.
fn column_geometry(available: f32, gutter: f32, config: &EditorConfig) -> (f32, f32) {
    let padding = 16.0_f32.min(available / 4.0);
    let gutter = gutter.min(available / 4.0);
    let reserve = if config.centered {
        2.0 * gutter
    } else {
        gutter
    };
    let limit = if config.max_width.is_finite() && config.max_width > 0.0 {
        config.max_width
    } else {
        760.0
    };
    let width = (available - 2.0 * padding - reserve).max(1.0).min(limit);
    let left = if config.centered {
        (available - width) / 2.0
    } else {
        padding + gutter
    };
    (left, width)
}

fn source_line_rows(source: &str, projection: &Projection, galley: &egui::Galley) -> Vec<usize> {
    std::iter::once(0)
        .chain(
            source
                .chars()
                .enumerate()
                .filter_map(|(i, ch)| (ch == '\n').then_some(i + 1)),
        )
        .map(|start| {
            let mut cursor = CCursor::new(projection.to_display(start));
            cursor.prefer_next_row = true;
            galley.from_ccursor(cursor).rcursor.row
        })
        .collect()
}

#[derive(Default)]
pub struct Editor {
    pub selection: CCursorRange,
    undo: egui::util::undoer::Undoer<(CCursorRange, String)>,
    composition: Option<CCursorRange>,
    preferred_x: Option<f32>,
}

fn byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(i, _)| i)
}

impl Editor {
    /// True while an IME composition is in progress; external reloads must wait.
    pub fn is_composing(&self) -> bool {
        self.composition.is_some()
    }

    fn replace(&mut self, source: &mut String, replacement: &str) {
        let [start, end] = self.selection.sorted();
        source.replace_range(
            byte(source, start.index)..byte(source, end.index),
            replacement,
        );
        self.selection = CCursorRange::one(CCursor::new(start.index + replacement.chars().count()));
        self.preferred_x = None;
    }

    fn displayed(&self, projection: &Projection, galley: &egui::Galley) -> CursorRange {
        CursorRange {
            primary: galley.from_ccursor(CCursor::new(
                projection.to_display(self.selection.primary.index),
            )),
            secondary: galley.from_ccursor(CCursor::new(
                projection.to_display(self.selection.secondary.index),
            )),
        }
    }

    fn set_from_display(&mut self, range: CursorRange, projection: &Projection) {
        self.selection = CCursorRange {
            primary: CCursor::new(projection.to_source(range.primary.ccursor.index)),
            secondary: CCursor::new(projection.to_source(range.secondary.ccursor.index)),
        };
    }

    fn projection(&self, ui: &Ui, source: &str, width: f32, focused: bool) -> Projection {
        live_preview::project(
            ui,
            source,
            width,
            focused.then_some((self.selection.primary.index, self.selection.secondary.index)),
        )
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        source: &mut String,
        pending: Option<(usize, usize)>,
        config: &EditorConfig,
        viewport_height: f32,
    ) -> egui::Response {
        let id = ui.make_persistent_id("live_markdown_editor");
        if let Some((start, end)) = pending {
            self.selection = CCursorRange::two(CCursor::new(start), CCursor::new(end));
            self.preferred_x = None;
            ui.memory_mut(|mem| mem.request_focus(id));
        }
        let count = source.chars().count();
        self.selection.primary.index = self.selection.primary.index.min(count);
        self.selection.secondary.index = self.selection.secondary.index.min(count);
        let available_width = ui.available_width().max(1.0);
        let gutter_font = egui::FontId::monospace(12.0);
        let gutter = if config.line_numbers {
            let count = source.bytes().filter(|&b| b == b'\n').count() + 1;
            ui.fonts(|fonts| {
                fonts
                    .layout_no_wrap(
                        count.to_string(),
                        gutter_font.clone(),
                        ui.visuals().weak_text_color(),
                    )
                    .size()
                    .x
            }) + 20.0
        } else {
            0.0
        };
        let (left, width) = column_geometry(available_width, gutter, config);
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("editor_dim"), config.dim_paragraphs));
        // Typewriter mode needs room to scroll the last line up to the middle.
        let tail = if config.typewriter {
            viewport_height * 0.5
        } else {
            TOP_MARGIN
        };
        let focused = ui.memory(|mem| mem.has_focus(id));
        let projection = self.projection(ui, source, width, focused);
        let galley = ui.fonts(|fonts| fonts.layout_job(projection.job.clone()));
        let height = galley.size().y.max(ui.text_style_height(&TextStyle::Body));
        let (_, rect) = ui.allocate_space(egui::vec2(
            available_width,
            (height + TOP_MARGIN + tail).max(viewport_height),
        ));
        let origin = rect.min + egui::vec2(left, TOP_MARGIN);
        let mut response = ui.interact(rect, id, egui::Sense::click_and_drag());
        let old_selection = self.selection;
        let old_source = source.clone();
        let time = ui.input(|i| i.time);
        self.undo
            .feed_state(time, &(self.selection, source.clone()));
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            ui.output_mut(|out| out.mutable_text_under_cursor = true);
        }
        let pointer_event =
            ui.input(|i| i.pointer.any_pressed() || i.pointer.delta() != egui::Vec2::ZERO);
        if pointer_event || response.double_clicked() || response.triple_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let cursor = galley.cursor_from_pos(pos - origin);
                let mut state = TextCursorState::from(self.displayed(&projection, &galley));
                if state.pointer_interaction(ui, &response, cursor, &galley, response.dragged()) {
                    self.set_from_display(state.range(&galley).unwrap(), &projection);
                    self.preferred_x = None;
                    self.composition = None;
                    response.request_focus();
                }
            }
        }
        if response.clicked_elsewhere() {
            response.surrender_focus();
        }
        let focused = response.has_focus();
        let filter = egui::EventFilter {
            tab: true,
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: false,
        };
        let mut scroll_to_cursor = pending.is_some();
        if focused {
            ui.memory_mut(|mem| mem.set_focus_lock_filter(id, filter));
            for event in ui.input(|i| i.filtered_events(&filter)) {
                let before = (self.selection, source.clone());
                if matches!(
                    &event,
                    Event::Cut | Event::Paste(_) | Event::CompositionStart
                ) {
                    // Keep discrete edits separate from a preceding typing burst.
                    self.undo.add_undo(&before);
                }
                match event {
                    Event::Copy | Event::Cut => {
                        if self.selection.primary.index == self.selection.secondary.index {
                            // Match TextEdit's copy/cut-all behavior with no selection.
                            self.selection = CCursorRange::two(
                                CCursor::new(0),
                                CCursor::new(source.chars().count()),
                            );
                        }
                        let [a, b] = self.selection.sorted();
                        ui.output_mut(|out| {
                            out.copied_text =
                                source[byte(source, a.index)..byte(source, b.index)].to_owned()
                        });
                        if matches!(event, Event::Cut) {
                            self.replace(source, "");
                        } else {
                            self.selection = before.0;
                        }
                    }
                    Event::Text(text)
                        if self.composition.is_none()
                            && !text.is_empty()
                            && text != "\n"
                            && text != "\r" =>
                    {
                        self.replace(source, &text);
                    }
                    Event::Paste(text) if self.composition.is_none() && !text.is_empty() => {
                        self.replace(source, &text.replace("\r\n", "\n").replace('\r', "\n"));
                    }
                    Event::CompositionStart => self.composition = Some(self.selection),
                    Event::CompositionUpdate(text) => {
                        if let Some(range) = self.composition {
                            self.selection = range;
                            let start = range.sorted()[0];
                            self.replace(source, &text);
                            self.composition =
                                Some(CCursorRange::two(start, self.selection.primary));
                        }
                    }
                    Event::CompositionEnd(text) => {
                        if let Some(range) = self.composition.take() {
                            self.selection = range;
                            self.replace(source, &text);
                        }
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } if self.composition.is_none() => {
                        if modifiers.command && (key == Key::Z || key == Key::Y) {
                            let snapshot = (self.selection, source.clone());
                            let restored = if key == Key::Y || modifiers.shift {
                                self.undo.redo(&snapshot)
                            } else {
                                self.undo.undo(&snapshot)
                            }
                            .cloned();
                            if let Some((selection, text)) = restored {
                                self.selection = selection;
                                *source = text;
                            }
                        } else if key == Key::Enter && !modifiers.command {
                            self.replace(source, "\n");
                        } else if key == Key::Tab && !modifiers.command {
                            if modifiers.shift {
                                let cursor = self.selection.sorted()[0].index;
                                let start_byte = source[..byte(source, cursor)]
                                    .rfind('\n')
                                    .map_or(0, |i| i + 1);
                                let line = &source[start_byte..];
                                let remove = if line.starts_with('\t') {
                                    1
                                } else {
                                    line.chars().take(4).take_while(|&ch| ch == ' ').count()
                                };
                                if remove > 0 {
                                    let start = source[..start_byte].chars().count();
                                    self.selection = CCursorRange::two(
                                        CCursor::new(start),
                                        CCursor::new(start + remove),
                                    );
                                    self.replace(source, "");
                                    self.selection = CCursorRange::one(CCursor::new(
                                        cursor.saturating_sub(remove).max(start),
                                    ));
                                }
                            } else {
                                self.replace(source, "\t");
                            }
                        } else if key == Key::Backspace || key == Key::Delete {
                            if self.selection.primary.index == self.selection.secondary.index {
                                let p = self.selection.primary;
                                let other = if key == Key::Backspace {
                                    if modifiers.ctrl || modifiers.alt {
                                        egui::text_selection::text_cursor_state::ccursor_previous_word(source, p)
                                    } else {
                                        CCursor::new(p.index.saturating_sub(1))
                                    }
                                } else if modifiers.ctrl || modifiers.alt {
                                    egui::text_selection::text_cursor_state::ccursor_next_word(
                                        source, p,
                                    )
                                } else {
                                    CCursor::new((p.index + 1).min(source.chars().count()))
                                };
                                self.selection.secondary = other;
                            }
                            self.replace(source, "");
                        } else {
                            self.navigate(ui, source, width, key, modifiers);
                        }
                    }
                    _ => {}
                }
                if before != (self.selection, source.clone()) {
                    scroll_to_cursor = true;
                }
            }
        }
        self.undo
            .feed_state(time, &(self.selection, source.clone()));
        // Rebuild after input: cursor and painted text always use the same mapping.
        let projection = self.projection(ui, source, width, focused);
        let galley = ui.fonts(|fonts| fonts.layout_job(projection.job.clone()));
        ui.painter()
            .rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        if config.line_numbers {
            let color = ui.visuals().weak_text_color().gamma_multiply(0.65);
            let separator_x = origin.x - 8.0;
            ui.painter().line_segment(
                [
                    egui::pos2(separator_x, rect.top()),
                    egui::pos2(separator_x, rect.bottom()),
                ],
                egui::Stroke::new(1.0_f32, color.gamma_multiply(0.3)),
            );
            for (line, row) in source_line_rows(source, &projection, &galley)
                .into_iter()
                .enumerate()
            {
                let position =
                    egui::pos2(origin.x - 14.0, origin.y + galley.rows[row].rect.center().y);
                ui.painter().text(
                    position,
                    egui::Align2::RIGHT_CENTER,
                    (line + 1).to_string(),
                    gutter_font.clone(),
                    color,
                );
            }
        }
        ui.painter()
            .galley(origin, galley.clone(), ui.visuals().text_color());
        if focused {
            let range = self.displayed(&projection, &galley);
            visuals::paint_text_selection(
                ui.painter(),
                ui.visuals(),
                origin,
                &galley,
                &range,
                None,
            );
            let cursor_rect = galley
                .pos_from_cursor(&range.primary)
                .translate(origin.to_vec2());
            visuals::paint_cursor(ui.painter(), ui.visuals(), cursor_rect);
            ui.output_mut(|out| out.ime = Some(egui::output::IMEOutput { rect, cursor_rect }));
            if config.typewriter && scroll_to_cursor && !response.dragged() {
                ui.scroll_to_rect(cursor_rect, Some(egui::Align::Center));
            } else if scroll_to_cursor || response.dragged() {
                ui.scroll_to_rect(cursor_rect, None);
            }
        }
        if old_source != *source {
            response.mark_changed();
        }
        if old_source != *source
            || old_selection != self.selection
            || response.gained_focus()
            || response.lost_focus()
        {
            ui.ctx().request_repaint();
        }
        response
    }

    fn navigate(
        &mut self,
        ui: &Ui,
        source: &str,
        width: f32,
        key: Key,
        modifiers: egui::Modifiers,
    ) {
        if matches!(key, Key::ArrowUp | Key::ArrowDown) && !modifiers.command {
            let projection = self.projection(ui, source, width, true);
            let galley = ui.fonts(|fonts| fonts.layout_job(projection.job.clone()));
            let mut range = self.displayed(&projection, &galley);
            let caret = galley.pos_from_cursor(&range.primary);
            let x = *self.preferred_x.get_or_insert(caret.left());
            let row = range.primary.rcursor.row;
            let target = if key == Key::ArrowUp {
                row.saturating_sub(1)
            } else {
                (row + 1).min(galley.rows.len() - 1)
            };
            range.primary = if target == row {
                if key == Key::ArrowUp {
                    galley.begin()
                } else {
                    galley.end()
                }
            } else {
                galley.cursor_from_pos(egui::vec2(x, galley.rows[target].rect.center().y))
            };
            if !modifiers.shift {
                range.secondary = range.primary;
            }
            self.set_from_display(range, &projection);
        } else {
            self.preferred_x = None;
            // Horizontal/word movement and deletion operate on the source. The
            // caret line is revealed, so there are no invisible cursor stops.
            let job =
                live_preview::project(ui, source, width, Some((0, source.chars().count()))).job;
            let galley = ui.fonts(|fonts| fonts.layout_job(job));
            let mut range = CursorRange {
                primary: galley.from_ccursor(self.selection.primary),
                secondary: galley.from_ccursor(self.selection.secondary),
            };
            if range.on_key_press(ui.ctx().os(), &galley, &modifiers, key) {
                self.selection = range.as_ccursor_range();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Harness {
        ctx: egui::Context,
        editor: Editor,
        source: String,
        time: f64,
        config: EditorConfig,
    }

    impl Harness {
        fn new(source: &str) -> Self {
            let ctx = egui::Context::default();
            crate::fonts::install_test_fonts(&ctx);
            Self {
                ctx,
                editor: Editor::default(),
                source: source.into(),
                time: 0.0,
                config: EditorConfig::default(),
            }
        }
        fn frame(
            &mut self,
            events: Vec<Event>,
            pending: Option<(usize, usize)>,
        ) -> egui::FullOutput {
            self.time += 0.1;
            self.ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 700.0),
                    )),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        self.editor
                            .show(ui, &mut self.source, pending, &self.config, 0.0);
                    });
                },
            )
        }
        fn key(&mut self, key: Key, modifiers: egui::Modifiers) {
            self.frame(
                vec![Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                None,
            );
        }
    }

    #[test]
    fn keyboard_editing_unicode_selection_clipboard_and_undo_keep_markdown() {
        let original = "# Title\n\né **bold** and *italic* [link](url)";
        let mut h = Harness::new(original);
        let end = original.chars().count();
        h.frame(vec![], Some((end, end)));
        h.frame(vec![Event::Paste("\nnew **line**".into())], None);
        assert_eq!(h.source, format!("{original}\nnew **line**"));
        h.key(Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(h.source, original);
        h.key(Key::Y, egui::Modifiers::COMMAND);
        assert_eq!(h.source, format!("{original}\nnew **line**"));
        h.key(Key::A, egui::Modifiers::COMMAND);
        let copied = h.frame(vec![Event::Copy], None);
        assert_eq!(copied.platform_output.copied_text, h.source);
        h.frame(vec![Event::Text("café".into())], None);
        assert_eq!(h.source, "café");
        h.key(Key::Backspace, egui::Modifiers::NONE);
        assert_eq!(h.source, "caf");
        h.key(Key::Enter, egui::Modifiers::NONE);
        h.frame(vec![Event::Text("next".into())], None);
        assert_eq!(h.source, "caf\nnext");
        h.frame(vec![], Some((3, 4)));
        h.key(Key::Delete, egui::Modifiers::NONE);
        assert_eq!(h.source, "cafnext");
    }

    #[test]
    fn navigation_reveals_lines_and_shift_selection_keeps_source_indices() {
        let mut h = Harness::new("# One\n**two**\n*three*");
        h.frame(vec![], Some((0, 0)));
        h.key(Key::ArrowDown, egui::Modifiers::NONE);
        assert_eq!(h.editor.selection.primary.index, 8); // beginning of rendered 'two'
        h.key(Key::Home, egui::Modifiers::NONE);
        assert_eq!(h.editor.selection.primary.index, 6); // now revealed source line
        h.key(Key::ArrowRight, egui::Modifiers::SHIFT);
        assert_eq!(h.editor.selection.sorted().map(|c| c.index), [6, 7]);
        let copied = h.frame(vec![Event::Copy], None);
        assert_eq!(copied.platform_output.copied_text, "*");
        h.key(Key::End, egui::Modifiers::COMMAND | egui::Modifiers::CTRL);
        assert_eq!(h.editor.selection.primary.index, h.source.chars().count());
        h.key(Key::Home, egui::Modifiers::COMMAND | egui::Modifiers::CTRL);
        assert_eq!(h.editor.selection.primary.index, 0);
        h.key(Key::ArrowDown, egui::Modifiers::SHIFT);
        assert_eq!(h.editor.selection.secondary.index, 0);
        assert!(h.editor.selection.primary.index >= 6);
        let copied = h.frame(vec![Event::Copy], None);
        assert!(copied.platform_output.copied_text.starts_with("# One\n"));
    }

    #[test]
    fn pointer_click_on_collapsed_heading_maps_to_source() {
        let mut h = Harness::new("# Title\n\n**bold**");
        h.frame(vec![], None);
        let pos = egui::pos2(8.0, 15.0);
        h.frame(
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            None,
        );
        h.frame(
            vec![Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            None,
        );
        assert_eq!(h.editor.selection.primary.index, 2);
        h.frame(vec![Event::Text("X".into())], None);
        assert_eq!(h.source, "# XTitle\n\n**bold**");
    }

    #[test]
    fn ime_replaces_preedit_without_duplicate_commits() {
        let mut h = Harness::new("**text**");
        h.frame(vec![], Some((2, 6)));
        h.frame(
            vec![
                Event::CompositionStart,
                Event::CompositionUpdate("に".into()),
            ],
            None,
        );
        h.frame(
            vec![
                Event::CompositionUpdate("日本".into()),
                Event::CompositionEnd("日本語".into()),
            ],
            None,
        );
        assert_eq!(h.source, "**日本語**");
    }
    #[test]
    fn empty_document_focus_loss_and_reversed_cross_line_selection() {
        let mut h = Harness::new("");
        h.frame(vec![], Some((0, 0)));
        h.key(Key::ArrowUp, egui::Modifiers::NONE);
        h.key(Key::ArrowDown, egui::Modifiers::NONE);
        h.frame(vec![Event::Paste("# Head\n\n**é**".into())], None);
        let original = h.source.clone();
        let end = original.chars().count();
        h.frame(vec![], Some((end, 0)));
        let copied = h.frame(vec![Event::Copy], None);
        assert_eq!(copied.platform_output.copied_text, original);
        h.frame(vec![Event::Cut], None);
        assert_eq!(h.source, "");
        h.key(Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(h.source, original);
        // Clicking outside the editor must not mutate the Markdown buffer.
        let pos = egui::pos2(450.0, 600.0);
        h.frame(
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            None,
        );
        h.frame(
            vec![Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            None,
        );
        assert_eq!(h.source, original);
        h.frame(vec![Event::Text("must not insert".into())], None);
        assert_eq!(h.source, original);
    }

    #[test]
    fn dragging_across_lines_copies_source_and_supports_replacement() {
        let mut h = Harness::new("# Head\n\n**bold**");
        h.frame(vec![], None);
        let start = egui::pos2(8.0, 15.0);
        h.frame(
            vec![
                Event::PointerMoved(start),
                Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            None,
        );
        // Below all text, at the right edge: selection ends at document end.
        let end = egui::pos2(400.0, 90.0 + TOP_MARGIN);
        h.frame(vec![Event::PointerMoved(end)], None);
        h.frame(
            vec![Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            None,
        );
        assert_eq!(h.editor.selection.secondary.index, 2);
        assert_eq!(h.editor.selection.primary.index, h.source.chars().count());
        let copied = h.frame(vec![Event::Copy], None);
        assert_eq!(copied.platform_output.copied_text, "Head\n\n**bold**");
        h.frame(vec![Event::Text("New".into())], None);
        assert_eq!(h.source, "# New");
    }
    #[test]
    fn editor_config_defaults_partial_tables_and_round_trip() {
        let old: crate::AppConfig = toml::from_str("[theme]\nbase = 'light'").unwrap();
        assert!(!old.editor.line_numbers);
        assert_eq!(old.editor.font_family, "Literata");
        assert_eq!(old.editor.font_size, 18.0);
        assert_eq!(old.editor.line_height, 1.4);
        assert_eq!(old.editor.max_width, 760.0);
        assert!(old.editor.centered);
        let partial: crate::AppConfig = toml::from_str("[editor]\nline_numbers = true").unwrap();
        assert_eq!(partial.editor.max_width, 760.0);
        let configured: crate::AppConfig =
            toml::from_str("[editor]\nline_numbers = true\nmax_width = 640\ncentered = false\nfont_family = 'Atkinson Hyperlegible'\nfont_size = 22.0\nline_height = 1.6")
                .unwrap();
        let saved = toml::to_string(&configured).unwrap();
        let restored: crate::AppConfig = toml::from_str(&saved).unwrap();
        assert!(restored.editor.line_numbers);
        assert_eq!(restored.editor.max_width, 640.0);
        assert!(!restored.editor.centered);
        assert_eq!(restored.editor.font_family, "Atkinson Hyperlegible");
        assert_eq!(restored.editor.font_size, 22.0);
        assert_eq!(restored.editor.line_height, 1.6);
    }

    #[test]
    fn column_width_and_alignment_are_independent_of_canvas() {
        let mut config = EditorConfig::default();
        assert_eq!(column_geometry(1200.0, 40.0, &config), (220.0, 760.0));
        config.centered = false;
        assert_eq!(column_geometry(1200.0, 40.0, &config), (56.0, 760.0));
        for available in [20.0, 200.0, 800.0] {
            let (left, width) = column_geometry(available, 40.0, &config);
            assert!(width > 0.0 && left + width <= available);
        }
        config.max_width = f32::NAN;
        assert_eq!(column_geometry(1200.0, 0.0, &config).1, 760.0);
    }

    #[test]
    fn gutter_tracks_source_lines_not_wrapped_rows_and_canvas_fills_viewport() {
        let ctx = egui::Context::default();
        crate::fonts::install_test_fonts(&ctx);
        let mut editor = Editor::default();
        let mut source = "# Heading\n\nA **long bold sentence** that wraps across several rows with a [link](url).\n```\ncode\n```\n".to_string();
        let original = source.clone();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for active in [None, Some((0, source.chars().count()))] {
                    let projection = live_preview::project(ui, &source, 100.0, active);
                    let galley = ui.fonts(|fonts| fonts.layout_job(projection.job.clone()));
                    let rows = source_line_rows(&source, &projection, &galley);
                    assert_eq!(rows.len(), 7); // includes empty and trailing source lines
                    assert!(rows.windows(2).all(|pair| pair[0] < pair[1]));
                    assert!(rows[3] > rows[2] + 1); // wrapping never adds a line number
                }
                let config = EditorConfig {
                    line_numbers: true,
                    max_width: 200.0,
                    centered: true,
                    ..Default::default()
                };
                let width = ui.available_width();
                let response = editor.show(ui, &mut source, None, &config, 600.0);
                assert_eq!(response.rect.width(), width);
                assert!(response.rect.height() >= 600.0);
                assert_eq!(source, original);
            });
        });
    }
    #[test]
    fn centered_column_with_gutter_preserves_pointer_and_keyboard_offsets() {
        let mut h = Harness::new("# Title\n**bold**");
        h.config = EditorConfig {
            line_numbers: true,
            max_width: 200.0,
            centered: true,
            ..Default::default()
        };
        h.frame(vec![], None);
        // 500px screen, 8px panel margins: centered 200px text starts at x=150.
        let pos = egui::pos2(150.0, 30.0);
        h.frame(
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            None,
        );
        h.frame(
            vec![Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            None,
        );
        assert_eq!(h.editor.selection.primary.index, 2);
        h.frame(vec![Event::Text("é".into())], None);
        assert_eq!(h.source, "# éTitle\n**bold**");
        h.key(Key::Home, egui::Modifiers::NONE);
        assert_eq!(h.editor.selection.primary.index, 0);
        h.key(Key::ArrowDown, egui::Modifiers::NONE);
        assert_eq!(h.editor.selection.primary.index, 11);
        h.key(Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(h.source, "# Title\n**bold**");
    }
}
