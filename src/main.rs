use directories::ProjectDirs;
use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use egui::{Align2, CentralPanel, Color32, Context, TextEdit, TextStyle, TopBottomPanel, Visuals};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_extras::StripBuilder;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{Builder, NamedTempFile};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
    let app = if args.len() > 1 {
        let cli_path = PathBuf::from(&args[1]);
        match fs::read_to_string(&cli_path) {
            Ok(content) => MarkdownApp {
                content: content.clone(),
                original_content: content,
                file_path: Some(fs::canonicalize(&cli_path).unwrap_or_else(|_| cli_path.clone())),
                ..Default::default()
            },
            Err(e) => {
                eprintln!("Error reading file '{}': {}", cli_path.display(), e);
                MarkdownApp::default()
            }
        }
    } else {
        MarkdownApp::default()
    };
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "md-echo - edit/preview",
        options,
        Box::new(move |_cc| Box::new(app)),
    )
}

struct MarkdownApp {
    content: String,
    original_content: String,
    file_path: Option<PathBuf>,
    working_dir: PathBuf,
    config_path: Option<PathBuf>,
    config: AppConfig,
    cache: CommonMarkCache,
    modified: bool,
    show_exit_confirm: bool,
    pending_new: bool,
    pending_open: bool,
    pending_save: bool,
    pending_save_as: bool,
    pending_exit: bool,
    pending_lint: bool,
    pending_format: bool,
    tool_output: Option<String>,
    show_tool_output: bool,
    theme_applied: bool,
    scroll_left: f32,
    scroll_right: f32,
    current_line: usize,
    cursor_fraction: f32,
    preview_max_scroll: f32,
    editor_has_focus: bool,
    current_char_index: usize,
    selection_start: usize,
    selection_end: usize,
    pending_selection: Option<(usize, usize)>,
    show_help_overlay: bool,
    show_find_replace: bool,
    find_query: String,
    replace_query: String,
    find_status: Option<String>,
    focus_find_field: bool,
    focus_replace_field: bool,
    pending_find_next: bool,
    pending_find_previous: bool,
    pending_replace: bool,
    pending_replace_all: bool,
    pending_delete_line: bool,
    pending_page_up: bool,
    pending_page_down: bool,
    pending_save_all: bool,
    pending_editor_scroll: Option<f32>,
    visible_editor_lines: usize,
    editor_line_height: f32,
    pending_discard_action: Option<DiscardAction>,
}

#[derive(Clone)]
enum DiscardAction {
    NewDocument,
    OpenDialog,
    OpenPath(PathBuf),
}

impl DiscardAction {
    fn description(&self) -> String {
        match self {
            DiscardAction::NewDocument => "starting a new document".to_string(),
            DiscardAction::OpenDialog => "opening another file".to_string(),
            DiscardAction::OpenPath(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                format!("opening {name}")
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct AppConfig {
    working_dir: Option<PathBuf>,
    theme: ThemeConfig,
    tools: ToolsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            working_dir: None,
            theme: ThemeConfig::default(),
            tools: ToolsConfig::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct ThemeConfig {
    base: String,
    background: Option<String>,
    panel: Option<String>,
    text: Option<String>,
    accent: Option<String>,
    hyperlink: Option<String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            base: "dark".to_string(),
            background: None,
            panel: None,
            text: None,
            accent: None,
            hyperlink: None,
        }
    }
}

impl ThemeConfig {
    fn to_visuals(&self) -> Visuals {
        let mut visuals = match self.base.to_lowercase().as_str() {
            "light" => Visuals::light(),
            _ => Visuals::dark(),
        };

        if let Some(color) = self
            .background
            .as_deref()
            .and_then(|value| parse_color(value))
        {
            visuals.window_fill = color;
            visuals.panel_fill = color;
            visuals.extreme_bg_color = color;
        }

        if let Some(color) = self.panel.as_deref().and_then(|value| parse_color(value)) {
            visuals.panel_fill = color;
        }

        if let Some(color) = self.text.as_deref().and_then(|value| parse_color(value)) {
            visuals.override_text_color = Some(color);
        }

        if let Some(color) = self.accent.as_deref().and_then(|value| parse_color(value)) {
            visuals.selection.bg_fill = color;
            visuals.widgets.active.bg_fill = color;
            visuals.widgets.hovered.bg_fill = color;
            visuals.hyperlink_color = color;
        }

        if let Some(color) = self
            .hyperlink
            .as_deref()
            .and_then(|value| parse_color(value))
        {
            visuals.hyperlink_color = color;
        }

        visuals
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct ToolsConfig {
    lint: Option<Vec<String>>,
    lint_use_open_file: bool,
    format: Option<Vec<String>>,
    format_use_open_file: bool,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            lint: default_lint_command(),
            lint_use_open_file: false,
            format: default_format_command(),
            format_use_open_file: false,
        }
    }
}

fn default_lint_command() -> Option<Vec<String>> {
    Some(vec!["rumdl".to_string(), "check".to_string()])
}

fn default_format_command() -> Option<Vec<String>> {
    Some(vec!["rumdl".to_string(), "fmt".to_string()])
}

fn parse_color(value: &str) -> Option<Color32> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else if hex.len() == 8 {
        let a = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let r = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let g = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let b = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some(Color32::from_rgba_unmultiplied(r, g, b, a))
    } else {
        None
    }
}

impl Default for MarkdownApp {
    fn default() -> Self {
        let (mut config, config_path) = MarkdownApp::load_config();
        let working_dir = MarkdownApp::initial_working_directory(&mut config);
        let app = Self {
            content: String::new(),
            original_content: String::new(),
            file_path: None,
            working_dir,
            config_path,
            config,
            cache: CommonMarkCache::default(),
            modified: false,
            show_exit_confirm: false,
            pending_new: false,
            pending_open: false,
            pending_save: false,
            pending_save_as: false,
            pending_exit: false,
            pending_lint: false,
            pending_format: false,
            tool_output: None,
            show_tool_output: false,
            theme_applied: false,
            scroll_left: 0.0,
            scroll_right: 0.0,
            current_line: 0,
            cursor_fraction: 0.0,
            preview_max_scroll: 0.0,
            editor_has_focus: false,
            current_char_index: 0,
            selection_start: 0,
            selection_end: 0,
            pending_selection: None,
            show_help_overlay: false,
            show_find_replace: false,
            find_query: String::new(),
            replace_query: String::new(),
            find_status: None,
            focus_find_field: false,
            focus_replace_field: false,
            pending_find_next: false,
            pending_find_previous: false,
            pending_replace: false,
            pending_replace_all: false,
            pending_delete_line: false,
            pending_page_up: false,
            pending_page_down: false,
            pending_save_all: false,
            pending_editor_scroll: None,
            visible_editor_lines: 40,
            editor_line_height: 16.0,
            pending_discard_action: None,
        };

        if app
            .config_path
            .as_ref()
            .map(|path| !path.exists())
            .unwrap_or(false)
        {
            app.save_config();
        }

        app
    }
}

impl eframe::App for MarkdownApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.ensure_theme(ctx);
        // Handle hotkeys
        ctx.input(|i| {
            if i.key_pressed(egui::Key::H) && i.modifiers.ctrl {
                self.show_help_overlay = !self.show_help_overlay;
            }
            if i.key_pressed(egui::Key::Q) && i.modifiers.ctrl {
                if self.modified {
                    self.show_exit_confirm = true;
                } else {
                    self.pending_exit = true;
                }
            }
            if i.key_pressed(egui::Key::N) && i.modifiers.ctrl {
                self.pending_new = true;
            }
            if i.key_pressed(egui::Key::O) && i.modifiers.ctrl {
                self.pending_open = true;
            }
            if i.key_pressed(egui::Key::S) && i.modifiers.alt {
                self.pending_save_as = true;
            }
            if i.key_pressed(egui::Key::A) && i.modifiers.alt {
                self.pending_save_all = true;
            }
            if i.key_pressed(egui::Key::S) && i.modifiers.ctrl {
                if i.modifiers.shift {
                    self.pending_save_as = true;
                } else {
                    self.pending_save = true;
                }
            }
            if i.key_pressed(egui::Key::F) && i.modifiers.ctrl && !i.modifiers.shift {
                self.show_find_replace = true;
                self.focus_find_field = true;
                if let Some(selected) = self.selected_text() {
                    if !selected.is_empty() {
                        self.find_query = selected;
                    }
                }
            }
            if i.key_pressed(egui::Key::R) && i.modifiers.ctrl {
                self.show_find_replace = true;
                self.focus_find_field = self.find_query.is_empty();
                self.focus_replace_field = true;
            }
            if i.key_pressed(egui::Key::D) && i.modifiers.ctrl {
                self.pending_delete_line = true;
            }
            if i.key_pressed(egui::Key::PageUp) {
                self.pending_page_up = true;
            }
            if i.key_pressed(egui::Key::PageDown) {
                self.pending_page_down = true;
            }
            if i.key_pressed(egui::Key::L) && i.modifiers.ctrl && i.modifiers.shift {
                self.pending_lint = true;
            }
            if i.key_pressed(egui::Key::F) && i.modifiers.ctrl && i.modifiers.shift {
                self.pending_format = true;
            }
        });

        // ==== MENU BAR ====
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.run_or_prompt_discard(DiscardAction::NewDocument);
                        ui.close_menu();
                    }

                    if ui.button("Open").clicked() {
                        self.run_or_prompt_discard(DiscardAction::OpenDialog);
                        ui.close_menu();
                    }

                    if ui.button("Save").clicked() {
                        self.save_file(false);
                        ui.close_menu();
                    }

                    if ui.button("Save As...").clicked() {
                        self.save_file(true);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        if self.modified {
                            self.show_exit_confirm = true;
                        } else {
                            std::process::exit(0);
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Lint Markdown	Ctrl+Shift+L").clicked() {
                        self.pending_lint = true;
                        ui.close_menu();
                    }
                    if ui.button("Format Markdown	Ctrl+Shift+F").clicked() {
                        self.pending_format = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // ==== CENTRAL PANEL ====
        CentralPanel::default().show(ctx, |ui| {
            // Handle pending actions from hotkeys
            if self.pending_exit {
                self.pending_exit = false;
                std::process::exit(0);
            }
            if self.pending_new {
                self.pending_new = false;
                self.run_or_prompt_discard(DiscardAction::NewDocument);
            }
            if self.pending_open {
                self.pending_open = false;
                self.run_or_prompt_discard(DiscardAction::OpenDialog);
            }
            if self.pending_save {
                self.pending_save = false;
                self.save_file(false);
            }
            if self.pending_save_as {
                self.pending_save_as = false;
                self.save_file(true);
            }
            if self.pending_save_all {
                self.pending_save_all = false;
                self.save_file(false);
            }
            if self.pending_lint {
                self.pending_lint = false;
                self.run_lint_tool();
            }
            if self.pending_format {
                self.pending_format = false;
                self.run_format_tool();
            }
            if self.pending_delete_line {
                self.pending_delete_line = false;
                self.delete_current_line();
            }
            if self.pending_find_next {
                self.pending_find_next = false;
                self.queue_find_next(true);
            }
            if self.pending_find_previous {
                self.pending_find_previous = false;
                self.queue_find_previous(true);
            }
            if self.pending_replace {
                self.pending_replace = false;
                let replaced = self.replace_current_selection();
                if replaced {
                    self.queue_find_next(true);
                }
            }
            if self.pending_replace_all {
                self.pending_replace_all = false;
                self.replace_all_matches();
            }
            if self.pending_page_up {
                self.pending_page_up = false;
                let lines = self.visible_editor_lines.max(1);
                self.jump_lines(-(lines as isize));
            }
            if self.pending_page_down {
                self.pending_page_down = false;
                let lines = self.visible_editor_lines.max(1);
                self.jump_lines(lines as isize);
            }

            StripBuilder::new(ui)
                .size(egui_extras::Size::relative(0.25).at_least(160.0))
                .size(egui_extras::Size::relative(0.35))
                .size(egui_extras::Size::relative(0.40))
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        ui.vertical(|ui| {
                            self.show_file_tree(ui);
                        });
                    });

                    // MIDDLE: Editor inside ScrollArea
                    strip.cell(|ui| {
                        self.editor_line_height =
                            ui.text_style_height(&TextStyle::Monospace).max(1.0);
                        let mut editor_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);
                        let mut requested_offset = None;
                        if let Some(offset) = self.pending_editor_scroll.take() {
                            requested_offset = Some(offset);
                            editor_area = editor_area.vertical_scroll_offset(offset);
                        }
                        let scroll = editor_area.show(ui, |ui| {
                            let mut editor_output = TextEdit::multiline(&mut self.content)
                                .desired_width(f32::INFINITY)
                                .code_editor()
                                .show(ui);

                            if let Some((start, end)) = self.pending_selection.take() {
                                let ccursor_range =
                                    CCursorRange::two(CCursor::new(start), CCursor::new(end));
                                editor_output
                                    .state
                                    .cursor
                                    .set_char_range(Some(ccursor_range));
                                ui.memory_mut(|mem| mem.request_focus(editor_output.response.id));
                            }

                            let cursor_range =
                                editor_output.state.cursor.range(&editor_output.galley);
                            editor_output
                                .state
                                .store(ui.ctx(), editor_output.response.id);

                            let response = editor_output.response.clone();

                            self.editor_has_focus = response.has_focus();

                            if let Some(cursor_range) = cursor_range {
                                let sorted = cursor_range.sorted_cursors();
                                let char_count = self.content.chars().count();
                                let total_chars = char_count.max(1);
                                let cursor_index =
                                    cursor_range.primary.ccursor.index.min(char_count);
                                let fraction = cursor_index as f32 / total_chars as f32;
                                self.cursor_fraction = fraction.clamp(0.0, 1.0);
                                self.current_char_index = cursor_range.primary.ccursor.index;
                                self.current_line = cursor_range.primary.rcursor.row;
                                self.selection_start = sorted[0].ccursor.index.min(char_count);
                                self.selection_end = sorted[1].ccursor.index.min(char_count);
                            } else {
                                self.selection_start = 0;
                                self.selection_end = 0;
                                self.current_char_index = 0;
                            }

                            if response.changed() {
                                self.update_modified_state();
                            }

                            let line_height = editor_output
                                .galley
                                .rows
                                .first()
                                .map(|row| row.rect.height())
                                .unwrap_or_else(|| ui.text_style_height(&TextStyle::Monospace));
                            self.editor_line_height = line_height.max(1.0);
                        });
                        self.scroll_left = scroll.state.offset.y;
                        if let Some(target) = requested_offset {
                            let max_offset =
                                (scroll.content_size.y - scroll.inner_rect.height()).max(0.0);
                            let clamped = target.clamp(0.0, max_offset);
                            if (scroll.state.offset.y - clamped).abs() > 1.0 {
                                self.pending_editor_scroll = Some(clamped);
                            }
                        }
                        let visible_height = scroll.inner_rect.height();
                        if visible_height > 0.0 {
                            let lines = (visible_height / self.editor_line_height.max(1.0)).max(1.0)
                                as usize;
                            self.visible_editor_lines = lines.max(1);
                        }
                    });

                    // RIGHT: Preview
                    strip.cell(|ui| {
                        let mut preview_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

                        if self.editor_has_focus {
                            let max_scroll = self.preview_max_scroll.max(0.0);
                            let target_offset =
                                (self.cursor_fraction * max_scroll).clamp(0.0, max_scroll);
                            preview_area = preview_area.vertical_scroll_offset(target_offset);
                        }

                        let scroll = preview_area.show(ui, |ui| {
                            CommonMarkViewer::new("full_markdown").show(
                                ui,
                                &mut self.cache,
                                &self.content,
                            );
                        });
                        let visible_height = scroll.inner_rect.height();
                        let content_height = scroll.content_size.y;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        self.preview_max_scroll = max_scroll;
                        self.scroll_right = scroll.state.offset.y;
                    });
                });
        });

        // ==== STATUS BAR ====
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let name = self
                    .file_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Untitled".to_string());
                ui.label(format!(
                    "{}{}",
                    if self.modified { "🟡 " } else { "🟢 " },
                    name
                ));
                ui.separator();
                ui.label(format!("✍️ {} chars", self.content.len()));
                if let Some(status) = &self.find_status {
                    ui.separator();
                    ui.label(status);
                }
            });
        });

        if self.show_tool_output {
            let mut open = true;
            egui::Window::new("Tool Output")
                .open(&mut open)
                .resizable(true)
                .vscroll(true)
                .show(ctx, |ui| {
                    if let Some(output) = &self.tool_output {
                        ui.monospace(output);
                    } else {
                        ui.label("No output available.");
                    }
                });
            if !open {
                self.show_tool_output = false;
            }
        }

        if self.show_help_overlay {
            egui::Window::new("Key Bindings")
                .anchor(Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    egui::Grid::new("key_bindings_grid")
                        .striped(true)
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Ctrl+H");
                            ui.label("Toggle help");
                            ui.end_row();
                            ui.label("Ctrl+N");
                            ui.label("New file");
                            ui.end_row();
                            ui.label("Ctrl+O");
                            ui.label("Open file");
                            ui.end_row();
                            ui.label("Ctrl+Q");
                            ui.label("Quit");
                            ui.end_row();
                            ui.label("Ctrl+S");
                            ui.label("Save");
                            ui.end_row();
                            ui.label("Alt+S");
                            ui.label("Save as");
                            ui.end_row();
                            ui.label("Alt+A");
                            ui.label("Save all");
                            ui.end_row();
                            ui.label("Ctrl+Z / Ctrl+Y");
                            ui.label("Undo / Redo");
                            ui.end_row();
                            ui.label("Ctrl+F");
                            ui.label("Find");
                            ui.end_row();
                            ui.label("Ctrl+R");
                            ui.label("Replace");
                            ui.end_row();
                            ui.label("Ctrl+D");
                            ui.label("Delete line");
                            ui.end_row();
                            ui.label("Page Up");
                            ui.label("Move up one page");
                            ui.end_row();
                            ui.label("Page Down");
                            ui.label("Move down one page");
                            ui.end_row();
                            ui.label("Ctrl+Shift+L");
                            ui.label("Lint Markdown");
                            ui.end_row();
                            ui.label("Ctrl+Shift+F");
                            ui.label("Format Markdown");
                            ui.end_row();
                        });
                });
        }

        if self.show_find_replace {
            let mut open = true;
            egui::Window::new("Find / Replace")
                .anchor(Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
                .resizable(false)
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        let find_response = ui.text_edit_singleline(&mut self.find_query);
                        if self.focus_find_field {
                            find_response.request_focus();
                            self.focus_find_field = false;
                        }
                        let replace_response = ui.text_edit_singleline(&mut self.replace_query);
                        if self.focus_replace_field {
                            replace_response.request_focus();
                            self.focus_replace_field = false;
                        }

                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if enter_pressed {
                            if find_response.has_focus() {
                                self.pending_find_next = true;
                            } else if replace_response.has_focus() {
                                self.pending_replace = true;
                            }
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Find Next").clicked() {
                                self.pending_find_next = true;
                            }
                            if ui.button("Find Prev").clicked() {
                                self.pending_find_previous = true;
                            }
                            if ui.button("Replace").clicked() {
                                self.pending_replace = true;
                            }
                            if ui.button("Replace All").clicked() {
                                self.pending_replace_all = true;
                            }
                            if ui.button("Close").clicked() {
                                self.show_find_replace = false;
                            }
                        });

                        if let Some(status) = &self.find_status {
                            ui.label(status);
                        }
                    });
                });
            if !open {
                self.show_find_replace = false;
            }
        }

        // ==== EXIT CONFIRMATION ====
        if self.show_exit_confirm {
            egui::Window::new("Unsaved Changes")
                .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("You have unsaved changes. Save before exiting?");
                    ui.horizontal(|ui| {
                        if ui.button("Save and Exit").clicked() {
                            self.save_file(false);
                            std::process::exit(0);
                        }
                        if ui.button("Discard and Exit").clicked() {
                            std::process::exit(0);
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_exit_confirm = false;
                        }
                    });
                });
        }

        if self.pending_discard_action.is_some() {
            let description = self
                .pending_discard_action
                .as_ref()
                .map(|action| action.description())
                .unwrap_or_default();
            egui::Window::new("Unsaved Changes")
                .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "You have unsaved changes. Save before {}?",
                        description
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.save_file(false);
                            if !self.modified {
                                if let Some(action) = self.pending_discard_action.take() {
                                    self.perform_discard_action(action);
                                }
                            }
                        }
                        if ui.button("Discard").clicked() {
                            if let Some(action) = self.pending_discard_action.take() {
                                self.perform_discard_action(action);
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending_discard_action = None;
                        }
                    });
                });
        }
    }
}

impl MarkdownApp {
    fn load_config() -> (AppConfig, Option<PathBuf>) {
        let config_path = MarkdownApp::config_file_path();
        let mut config = AppConfig::default();

        if let Some(path) = &config_path {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                        Ok(parsed) => config = parsed,
                        Err(err) => eprintln!("Config parse error ({}): {}", path.display(), err),
                    },
                    Err(err) => eprintln!("Config read error ({}): {}", path.display(), err),
                }
            }
        }

        if config.working_dir.is_none() {
            if let Some(legacy_path) = MarkdownApp::legacy_settings_path() {
                if let Ok(contents) = fs::read_to_string(&legacy_path) {
                    let candidate = PathBuf::from(contents.trim());
                    if candidate.is_dir() {
                        config.working_dir = Some(candidate);
                    }
                }
            }
        }

        (config, config_path)
    }

    fn initial_working_directory(config: &mut AppConfig) -> PathBuf {
        if let Some(dir) = &config.working_dir {
            if dir.is_dir() {
                return dir.clone();
            }
        }

        let default_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        config.working_dir = Some(default_dir.clone());
        default_dir
    }

    fn config_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "fibnas", "md-echo")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    fn legacy_settings_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "fibnas", "md-echo")
            .map(|dirs| dirs.config_dir().join("settings.txt"))
    }

    fn save_config(&self) {
        if let Some(config_path) = &self.config_path {
            if let Some(parent) = config_path.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    eprintln!("Config directory error: {}", err);
                    return;
                }
            }

            match toml::to_string_pretty(&self.config) {
                Ok(serialized) => {
                    if let Err(err) = fs::write(config_path, serialized) {
                        eprintln!("Config write error: {}", err);
                    }
                }
                Err(err) => eprintln!("Config serialize error: {}", err),
            }
        }
    }

    fn ensure_theme(&mut self, ctx: &Context) {
        if self.theme_applied {
            return;
        }
        let visuals = self.config.theme.to_visuals();
        ctx.set_visuals(visuals);
        self.theme_applied = true;
    }

    fn set_working_directory(&mut self, new_dir: PathBuf) {
        if new_dir.is_dir() {
            self.working_dir = new_dir.clone();
            self.config.working_dir = Some(new_dir);
            self.save_config();
        } else {
            eprintln!("Invalid working directory: {}", new_dir.display());
        }
    }

    fn show_file_tree(&mut self, ui: &mut egui::Ui) {
        ui.label("Working Directory");
        ui.monospace(self.working_dir.display().to_string());

        if ui.button("Change...").clicked() {
            let mut dialog = FileDialog::new();
            if self.working_dir.is_dir() {
                dialog = dialog.set_directory(&self.working_dir);
            }
            if let Some(path) = dialog.pick_folder() {
                self.set_working_directory(path);
            }
        }

        ui.separator();

        if !self.working_dir.is_dir() {
            ui.label("Working directory is unavailable.");
            return;
        }

        let root_dir = self.working_dir.clone();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.render_directory(ui, &root_dir, true);
            });
    }

    fn render_directory(&mut self, ui: &mut egui::Ui, path: &Path, is_root: bool) {
        let name = if is_root {
            path.display().to_string()
        } else {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string())
        };

        let header = egui::CollapsingHeader::new(name)
            .id_source(path.display().to_string())
            .default_open(is_root);

        header.show(ui, |ui| match fs::read_dir(path) {
            Ok(entries) => {
                let mut directories = Vec::new();
                let mut files = Vec::new();

                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        directories.push(entry_path);
                    } else {
                        files.push(entry_path);
                    }
                }

                directories.sort();
                files.sort();

                for dir in directories {
                    self.render_directory(ui, &dir, false);
                }

                for file in files {
                    let file_name = file
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| file.display().to_string());

                    let file_path_string = file.display().to_string();
                    let is_selected = self
                        .file_path
                        .as_ref()
                        .map(|current| current == &file)
                        .unwrap_or(false);

                    let response = ui
                        .selectable_label(is_selected, file_name)
                        .on_hover_text(file_path_string);
                    if response.clicked() {
                        self.run_or_prompt_discard(DiscardAction::OpenPath(file.clone()));
                    }
                }
            }
            Err(err) => {
                ui.label(format!("Cannot read {}: {}", path.display(), err));
            }
        });
    }

    fn run_lint_tool(&mut self) {
        match self.config.tools.lint.clone() {
            Some(command) => {
                self.run_external_tool(&command, false, self.config.tools.lint_use_open_file)
            }
            None => self.show_tool_message(
                "No lint command configured. Add a [tools] lint entry to config.toml.",
            ),
        }
    }

    fn run_format_tool(&mut self) {
        match self.config.tools.format.clone() {
            Some(command) => {
                self.run_external_tool(&command, true, self.config.tools.format_use_open_file)
            }
            None => self.show_tool_message(
                "No format command configured. Add a [tools] format entry to config.toml.",
            ),
        }
    }

    fn run_external_tool(
        &mut self,
        command: &[String],
        modifies_content: bool,
        use_current_file: bool,
    ) {
        if command.is_empty() {
            self.show_tool_message("Configured tool command is empty.");
            return;
        }

        let mut temp_file: Option<NamedTempFile> = None;
        let target_path = if use_current_file {
            if self.modified {
                self.show_tool_message(
                    "Save before running this tool on the current file, or disable *_use_open_file.",
                );
                return;
            }
            match self.file_path.as_ref() {
                Some(path) => {
                    if path.is_file() {
                        path.clone()
                    } else {
                        self.show_tool_message(format!(
                            "Current file path '{}' is not a file.",
                            path.display()
                        ));
                        return;
                    }
                }
                None => {
                    self.show_tool_message(
                        "No file is currently open. Save the document first or disable *_use_open_file.",
                    );
                    return;
                }
            }
        } else {
            let file = match self.create_temp_markdown() {
                Ok(file) => file,
                Err(err) => {
                    self.show_tool_message(format!("Failed to prepare temp file: {}", err));
                    return;
                }
            };
            let path = file.path().to_path_buf();
            temp_file = Some(file);
            path
        };

        let mut cmd = Command::new(&command[0]);
        for arg in &command[1..] {
            cmd.arg(arg);
        }
        if self.working_dir.is_dir() {
            cmd.current_dir(&self.working_dir);
        }
        cmd.arg(&target_path);

        let output = match cmd.output() {
            Ok(output) => output,
            Err(err) => {
                self.show_tool_message(format!("Failed to run '{}': {}", command[0], err));
                return;
            }
        };

        if let Some(file) = temp_file.as_mut() {
            if let Err(err) = file.flush() {
                eprintln!("Temp file flush error: {}", err);
            }
        }

        let mut message = String::new();
        message.push_str(&format!(
            "$ {}\n",
            Self::format_command_for_display(command, &target_path)
        ));
        message.push_str(&format!("Status: {:?}\n", output.status));

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            message.push_str("\nstdout:\n");
            message.push_str(stdout.trim_end());
            message.push('\n');
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            message.push_str("\nstderr:\n");
            message.push_str(stderr.trim_end());
            message.push('\n');
        }

        if modifies_content && output.status.success() {
            match fs::read_to_string(&target_path) {
                Ok(new_content) => {
                    if new_content != self.content {
                        self.content = new_content;
                        self.modified = self.content != self.original_content;
                    }
                }
                Err(err) => {
                    message.push_str(&format!(
                        "\nFormat note: failed to read formatter output ({}): {}\n",
                        target_path.display(),
                        err
                    ));
                }
            }
        }

        self.show_tool_message(message);
    }

    fn format_command_for_display(command: &[String], path: &Path) -> String {
        let mut parts = command.to_vec();
        parts.push(path.display().to_string());
        parts.join(" ")
    }

    fn canonicalize_path(path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn create_temp_markdown(&self) -> std::io::Result<NamedTempFile> {
        let mut file = Builder::new().suffix(".md").tempfile()?;
        file.write_all(self.content.as_bytes())?;
        file.flush()?;
        Ok(file)
    }

    fn reset_view_state(&mut self) {
        self.current_line = 0;
        self.cursor_fraction = 0.0;
        self.scroll_left = 0.0;
        self.scroll_right = 0.0;
        self.preview_max_scroll = 0.0;
        self.editor_has_focus = false;
        self.current_char_index = 0;
        self.selection_start = 0;
        self.selection_end = 0;
        self.pending_selection = None;
        self.find_status = None;
        self.pending_editor_scroll = None;
        self.visible_editor_lines = 40;
        self.editor_line_height = 16.0;
    }

    fn update_modified_state(&mut self) {
        self.modified = self.content != self.original_content;
    }

    fn char_to_byte_index(text: &str, char_index: usize) -> usize {
        if char_index == 0 {
            return 0;
        }
        let mut count = 0usize;
        for (byte_idx, _) in text.char_indices() {
            if count == char_index {
                return byte_idx;
            }
            count += 1;
        }
        text.len()
    }

    fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
        text[..byte_index.min(text.len())].chars().count()
    }

    fn char_range_to_byte_range(text: &str, start: usize, end: usize) -> (usize, usize) {
        let start_byte = Self::char_to_byte_index(text, start);
        let end_byte = Self::char_to_byte_index(text, end);
        (start_byte, end_byte)
    }

    fn selected_text(&self) -> Option<String> {
        if self.selection_end > self.selection_start {
            let (start, end) = Self::char_range_to_byte_range(
                &self.content,
                self.selection_start,
                self.selection_end,
            );
            return Some(self.content[start..end].to_string());
        }
        None
    }

    fn find_next_match(&self, start_char: usize, wrap: bool) -> Option<(usize, usize)> {
        if self.find_query.is_empty() || self.content.is_empty() {
            return None;
        }
        let start_char = start_char.min(self.content.chars().count());
        let pattern = self.find_query.as_str();
        let start_byte = Self::char_to_byte_index(&self.content, start_char);
        let pattern_char_len = self.find_query.chars().count();

        let search_from_start = |byte_offset: usize| -> Option<(usize, usize)> {
            let haystack = &self.content[byte_offset..];
            haystack.find(pattern).map(|idx| {
                let match_start_byte = byte_offset + idx;
                let match_start_char = Self::byte_to_char_index(&self.content, match_start_byte);
                (match_start_char, match_start_char + pattern_char_len)
            })
        };

        if let Some(result) = search_from_start(start_byte) {
            return Some(result);
        }

        if wrap {
            if let Some(result) = search_from_start(0) {
                return Some(result);
            }
        }

        None
    }

    fn find_previous_match(&self, start_char: usize, wrap: bool) -> Option<(usize, usize)> {
        if self.find_query.is_empty() || self.content.is_empty() {
            return None;
        }
        let pattern = self.find_query.as_str();
        let pattern_char_len = self.find_query.chars().count();
        let start_byte =
            Self::char_to_byte_index(&self.content, start_char.min(self.content.chars().count()));

        let before = &self.content[..start_byte];
        let mut last_idx = None;
        for (idx, _) in before.match_indices(pattern) {
            last_idx = Some(idx);
        }
        if let Some(idx) = last_idx {
            let match_start_char = Self::byte_to_char_index(&self.content, idx);
            return Some((match_start_char, match_start_char + pattern_char_len));
        }

        if wrap {
            let mut wrap_last = None;
            for (idx, _) in self.content.match_indices(pattern) {
                wrap_last = Some(idx);
            }
            if let Some(idx) = wrap_last {
                let match_start_char = Self::byte_to_char_index(&self.content, idx);
                return Some((match_start_char, match_start_char + pattern_char_len));
            }
        }

        None
    }

    fn queue_find_next(&mut self, wrap: bool) {
        let start_from = if self.selection_end > self.selection_start {
            self.selection_end
        } else {
            self.current_char_index
        };
        if let Some((start, end)) = self.find_next_match(start_from, wrap) {
            self.pending_selection = Some((start, end));
            self.current_char_index = end;
            self.find_status = Some("Match found".to_string());
            self.selection_start = start;
            self.selection_end = end;
        } else {
            self.find_status = Some("No match found".to_string());
        }
    }

    fn queue_find_previous(&mut self, wrap: bool) {
        let start_from = if self.selection_start < self.selection_end {
            self.selection_start
        } else {
            self.current_char_index
        };
        if let Some((start, end)) = self.find_previous_match(start_from, wrap) {
            self.pending_selection = Some((start, end));
            self.current_char_index = start;
            self.find_status = Some("Match found".to_string());
            self.selection_start = start;
            self.selection_end = end;
        } else {
            self.find_status = Some("No previous match".to_string());
        }
    }

    fn replace_current_selection(&mut self) -> bool {
        if self.find_query.is_empty() {
            self.find_status = Some("Enter text to find".to_string());
            return false;
        }
        if self.selection_end <= self.selection_start {
            self.find_status = Some("Select a match or use Find".to_string());
            return false;
        }
        let (start_byte, end_byte) =
            Self::char_range_to_byte_range(&self.content, self.selection_start, self.selection_end);
        if &self.content[start_byte..end_byte] != self.find_query {
            self.find_status = Some("Selection does not match search text".to_string());
            return false;
        }
        let replacement = self.replace_query.clone();
        self.content
            .replace_range(start_byte..end_byte, replacement.as_str());
        let replacement_chars = replacement.chars().count();
        let new_selection_end = self.selection_start + replacement_chars;
        self.pending_selection = Some((self.selection_start, new_selection_end));
        self.current_char_index = new_selection_end;
        self.selection_end = new_selection_end;
        self.update_modified_state();
        self.find_status = Some("Replaced selection".to_string());
        true
    }

    fn replace_all_matches(&mut self) {
        if self.find_query.is_empty() {
            self.find_status = Some("Enter text to find".to_string());
            return;
        }
        let count = self.content.matches(&self.find_query).count();
        if count == 0 {
            self.find_status = Some("No matches to replace".to_string());
            return;
        }
        let replacement = self.replace_query.clone();
        self.content = self.content.replace(&self.find_query, &replacement);
        self.update_modified_state();
        self.pending_selection = Some((0, 0));
        self.selection_start = 0;
        self.selection_end = 0;
        self.current_char_index = 0;
        self.find_status = Some(format!("Replaced {} occurrence(s)", count));
    }

    fn delete_current_line(&mut self) {
        if self.content.is_empty() {
            return;
        }

        let mut byte_offset = 0usize;
        let mut line_start = 0usize;
        let mut line_end = self.content.len();
        let mut line_index = 0usize;

        for line in self.content.split_inclusive('\n') {
            let len = line.len();
            line_start = byte_offset;
            line_end = byte_offset + len;
            if line_index == self.current_line {
                break;
            }
            byte_offset += len;
            line_index += 1;
        }

        if self.current_line >= line_index
            && !self.content.is_empty()
            && line_end == self.content.len()
        {
            if let Some(last_line) = self.content.split_inclusive('\n').last() {
                let len = last_line.len();
                line_start = self.content.len().saturating_sub(len);
                line_end = self.content.len();
            }
        }

        if line_start >= line_end {
            return;
        }

        let remove_range = if line_end < self.content.len() {
            line_start..line_end
        } else if line_start > 0 && self.content.as_bytes()[line_start - 1] == b'\n' {
            (line_start - 1)..line_end
        } else {
            line_start..line_end
        };

        self.content.replace_range(remove_range.clone(), "");
        self.update_modified_state();

        let new_char_index = Self::byte_to_char_index(&self.content, remove_range.start);
        self.pending_selection = Some((new_char_index, new_char_index));
        self.current_char_index = new_char_index;
        self.selection_start = new_char_index;
        self.selection_end = new_char_index;
        self.find_status = Some("Line deleted".to_string());
    }

    fn total_line_count(&self) -> usize {
        if self.content.is_empty() {
            1
        } else {
            self.content.split_inclusive('\n').count().max(1)
        }
    }

    fn line_bounds_char_indices(&self, line_idx: usize) -> Option<(usize, usize)> {
        if self.content.is_empty() {
            return if line_idx == 0 { Some((0, 0)) } else { None };
        }

        let mut char_offset = 0usize;
        for (idx, segment) in self.content.split_inclusive('\n').enumerate() {
            let seg_chars = segment.chars().count();
            let line_chars = if segment.ends_with('\n') {
                seg_chars.saturating_sub(1)
            } else {
                seg_chars
            };
            let start = char_offset;
            let end = start + line_chars;
            if idx == line_idx {
                return Some((start, end));
            }
            char_offset += seg_chars;
        }

        if line_idx == self.total_line_count().saturating_sub(1) {
            let count = self.content.chars().count();
            return Some((count, count));
        }

        None
    }

    fn jump_lines(&mut self, delta: isize) {
        let total_lines = self.total_line_count();
        if total_lines == 0 {
            return;
        }

        let max_index = total_lines.saturating_sub(1) as isize;
        let current_line = self.current_line.min(max_index as usize) as isize;
        let target_line = (current_line + delta).clamp(0, max_index) as usize;

        let current_bounds = self
            .line_bounds_char_indices(current_line as usize)
            .unwrap_or((0, 0));
        let column = self.current_char_index.saturating_sub(current_bounds.0);

        if let Some((target_start, target_end)) = self.line_bounds_char_indices(target_line) {
            let line_len = target_end.saturating_sub(target_start);
            let target_char = target_start + column.min(line_len);
            self.pending_selection = Some((target_char, target_char));
            self.current_char_index = target_char;
            self.selection_start = target_char;
            self.selection_end = target_char;
            self.current_line = target_line;
            let total_chars = self.content.chars().count().max(1);
            self.cursor_fraction =
                (self.current_char_index as f32 / total_chars as f32).clamp(0.0, 1.0);
            let offset = (self.current_line as f32) * self.editor_line_height.max(1.0);
            self.pending_editor_scroll = Some(offset.max(0.0));
        }
    }

    fn show_tool_message<S: Into<String>>(&mut self, message: S) {
        self.tool_output = Some(message.into());
        self.show_tool_output = true;
    }

    fn open_file_from_path(&mut self, path: &Path) {
        match fs::read_to_string(path) {
            Ok(data) => {
                self.content = data.clone();
                self.original_content = data;
                self.file_path = Some(Self::canonicalize_path(path));
                self.modified = false;
                self.reset_view_state();
            }
            Err(err) => {
                eprintln!("Error reading file '{}': {}", path.display(), err);
            }
        }
    }

    fn save_file(&mut self, save_as: bool) {
        if save_as || self.file_path.is_none() {
            if let Some(path) = FileDialog::new().save_file() {
                if let Err(err) = fs::write(&path, &self.content) {
                    eprintln!("Save error: {}", err);
                } else {
                    self.file_path = Some(Self::canonicalize_path(&path));
                    self.original_content = self.content.clone();
                    self.modified = false;
                }
            }
        } else if let Some(path) = &self.file_path {
            if let Err(err) = fs::write(path, &self.content) {
                eprintln!("Save error: {}", err);
            } else {
                self.original_content = self.content.clone();
                self.modified = false;
            }
        }
    }

    fn run_or_prompt_discard(&mut self, action: DiscardAction) {
        if self.modified {
            self.pending_discard_action = Some(action);
        } else {
            self.perform_discard_action(action);
        }
    }

    fn perform_discard_action(&mut self, action: DiscardAction) {
        match action {
            DiscardAction::NewDocument => {
                self.content.clear();
                self.original_content.clear();
                self.file_path = None;
                self.modified = false;
                self.reset_view_state();
            }
            DiscardAction::OpenDialog => {
                if let Some(path) = FileDialog::new().pick_file() {
                    self.open_file_from_path(&path);
                }
            }
            DiscardAction::OpenPath(path) => {
                self.open_file_from_path(&path);
            }
        }
    }
}
