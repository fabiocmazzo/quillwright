// Release builds on Windows are GUI apps: no console window next to the editor.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use directories::ProjectDirs;
use eframe::egui;
use egui::{Align2, CentralPanel, Color32, Context};
mod assistant;
mod assistant_ui;
mod editor;
mod export;
mod export_ui;
mod file_sync;
mod fonts;
mod history;
mod live_preview;
mod manuscript;
mod pdf_view;
mod process;
mod project;
mod reference;
mod reference_ui;
mod saving;
mod spelling;
mod spelling_ui;
mod theme;
mod workspace;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
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
                show_home: false,
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
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Quillwright")
        // Matches the .desktop file, so Linux docks show the right icon.
        .with_app_id("quillwright");
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/quillwright-128.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Quillwright",
        options,
        Box::new(move |cc| {
            fonts::install(&cc.egui_ctx, &app.config.editor);
            Ok(Box::new(app))
        }),
    )
}

struct MarkdownApp {
    content: String,
    editor: editor::Editor,
    original_content: String,
    file_path: Option<PathBuf>,
    working_dir: PathBuf,
    config_path: Option<PathBuf>,
    config: AppConfig,
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
    current_line: usize,
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
    new_project: Option<project::NewProject>,
    /// Background watcher that reports external changes (for example, from agents).
    monitor: Option<file_sync::Monitor>,
    monitor_target: Option<file_sync::Target>,
    /// The open file no longer exists on disk; saving recreates it.
    base_missing: bool,
    /// Disk contents that conflict with unsaved edits in the editor.
    disk_conflict: Option<file_sync::Disk>,
    sync_notice: Option<String>,
    window_focused: bool,
    // Writing workspace state.
    index: manuscript::Index,
    index_dirty: bool,
    show_home: bool,
    focus_mode: bool,
    focus_since: f64,
    pending_focus_toggle: bool,
    /// Net words typed in the editor since launch (external reloads excluded).
    session_words: i64,
    current_words: usize,
    counted_text: String,
    /// The buffer was replaced by loading a file, not by typing.
    external_load: bool,
    window_title: String,
    assistant: assistant_ui::AssistantState,
    saving: saving::SavingState,
    export: export_ui::ExportState,
    spelling: spelling::Speller,
    reference: reference_ui::ReferenceState,
    /// Where version history is kept; None if no data directory is available.
    history_root: Option<PathBuf>,
}

#[derive(Clone)]
enum DiscardAction {
    NewDocument,
    OpenDialog,
    OpenPath(PathBuf),
    CreateProject(PathBuf),
    OpenProject(PathBuf),
    Home,
}

impl DiscardAction {
    fn description(&self) -> String {
        match self {
            DiscardAction::NewDocument => "starting a new document".to_string(),
            DiscardAction::Home => "going to the project home".to_string(),
            DiscardAction::OpenDialog => "opening another file".to_string(),
            DiscardAction::CreateProject(path) => format!("creating project {}", path.display()),
            DiscardAction::OpenProject(path) => format!("opening project {}", path.display()),
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
#[derive(Default)]
struct AppConfig {
    working_dir: Option<PathBuf>,
    theme: ThemeConfig,
    tools: ToolsConfig,
    editor: editor::EditorConfig,
    view: ViewConfig,
    assistant: assistant::AssistantConfig,
    export: export::ExportConfig,
    spelling: spelling::SpellingConfig,
    /// Most recently opened projects, newest first.
    recent_projects: Vec<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct ViewConfig {
    sidebar: bool,
    /// Words to write per session; 0 hides the goal.
    session_goal: usize,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            sidebar: true,
            session_goal: 0,
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
            base: "night".to_string(),
            background: None,
            panel: None,
            text: None,
            accent: None,
            hyperlink: None,
        }
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
            editor: editor::Editor::default(),
            original_content: String::new(),
            file_path: None,
            working_dir,
            config_path,
            config,
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
            current_line: 0,
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
            new_project: None,
            monitor: None,
            monitor_target: None,
            base_missing: false,
            disk_conflict: None,
            sync_notice: None,
            window_focused: true,
            index: manuscript::Index::default(),
            index_dirty: true,
            show_home: true,
            focus_mode: false,
            focus_since: 0.0,
            pending_focus_toggle: false,
            session_words: 0,
            current_words: 0,
            counted_text: String::new(),
            external_load: true,
            window_title: String::new(),
            assistant: assistant_ui::AssistantState::default(),
            saving: saving::SavingState::default(),
            export: export_ui::ExportState::default(),
            spelling: spelling::Speller::default(),
            reference: reference_ui::ReferenceState::default(),
            history_root: history::default_root(),
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        self.ensure_theme(ctx);
        self.sync_with_disk(ctx);
        self.poll_assistant();
        self.poll_spelling(ctx);
        self.poll_reference(ctx);
        // Closing the window: save first, or ask if that is not possible.
        if ctx.input(|i| i.viewport().close_requested()) && !self.save_before_leaving() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.disk_conflict.is_none() {
                self.show_exit_confirm = true;
            }
        }
        // Handle hotkeys. Actions that need the context run after the input lock is released.
        let (mut new_chapter, mut save, mut text_delta) = (false, false, 0.0);
        let mut open_export = false;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::H) && i.modifiers.ctrl {
                self.show_help_overlay = !self.show_help_overlay;
            }
            if i.key_pressed(egui::Key::Q) && i.modifiers.ctrl {
                self.pending_exit = true;
            }
            if i.key_pressed(egui::Key::N) && i.modifiers.ctrl {
                if i.modifiers.shift {
                    new_chapter = true;
                } else {
                    self.pending_new = true;
                }
            }
            if i.key_pressed(egui::Key::F11)
                || (self.focus_mode && i.key_pressed(egui::Key::Escape))
            {
                self.pending_focus_toggle = true;
            }
            if i.key_pressed(egui::Key::E) && i.modifiers.ctrl && i.modifiers.shift {
                open_export = true;
            }
            if i.key_pressed(egui::Key::K) && i.modifiers.ctrl {
                self.toggle_assistant();
            }
            if i.key_pressed(egui::Key::B) && i.modifiers.ctrl {
                self.config.view.sidebar = !self.config.view.sidebar;
                save = true;
            }
            if i.key_pressed(egui::Key::T) && i.modifiers.ctrl && i.modifiers.shift {
                self.config.editor.typewriter = !self.config.editor.typewriter;
                save = true;
            }
            if i.key_pressed(egui::Key::CloseBracket) && i.modifiers.ctrl {
                text_delta += 1.0;
            }
            if i.key_pressed(egui::Key::OpenBracket) && i.modifiers.ctrl {
                text_delta -= 1.0;
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
                if i.modifiers.shift {
                    self.config.editor.dim_paragraphs = !self.config.editor.dim_paragraphs;
                    save = true;
                } else {
                    self.pending_delete_line = true;
                }
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

        if new_chapter {
            self.create_chapter();
        }
        if save {
            self.save_config();
        }
        if text_delta != 0.0 {
            self.change_text_size(ctx, text_delta);
        }
        if open_export {
            self.open_export();
        }
        if self.pending_focus_toggle {
            self.pending_focus_toggle = false;
            self.set_focus_mode(ctx, !self.focus_mode);
        }

        self.run_pending_actions();
        if self.index_dirty {
            self.index = manuscript::Index::build(&self.working_dir);
            self.index_dirty = false;
            self.refresh_spelling();
            self.refresh_reference();
        }

        if !self.focus_mode {
            self.show_menu_bar(ui);
            self.show_status_bar(ui);
            if self.config.view.sidebar {
                self.show_sidebar(ui);
            }
            if self.assistant.open {
                self.show_assistant_panel(ui);
            }
            if self.reference.open {
                self.show_reference_panel(ui);
            }
        }

        // ==== PAGE ====
        let palette = theme::current(ctx);
        CentralPanel::default()
            .frame(egui::Frame::NONE.fill(palette.page))
            .show(ui, |ui| {
                if self.show_home && self.file_path.is_none() && !self.modified {
                    self.show_home_screen(ui);
                } else {
                    self.show_editor(ui, ctx);
                }
            });

        if self.focus_mode {
            self.show_focus_overlay(ctx);
        }

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
                        .striped(false)
                        .spacing([24.0, 6.0])
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (keys, action) in workspace::SHORTCUTS {
                                ui.label(egui::RichText::new(*keys).monospace());
                                ui.label(*action);
                                ui.end_row();
                            }
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

        self.show_new_project_dialog(ctx);
        self.show_disk_conflict_dialog(ctx);

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
                            // Stay open if the save was cancelled or hit a conflict.
                            if !self.modified {
                                std::process::exit(0);
                            }
                            self.show_exit_confirm = false;
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

        self.show_history_window(ctx);
        self.show_export_window(ctx);
        self.after_frame(ctx);
        self.run_autosave(ctx);
    }
}

impl MarkdownApp {
    fn run_pending_actions(&mut self) {
        if self.pending_exit {
            self.pending_exit = false;
            if self.save_before_leaving() {
                std::process::exit(0);
            } else if self.disk_conflict.is_none() {
                self.show_exit_confirm = true;
            }
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
    }

    fn load_config() -> (AppConfig, Option<PathBuf>) {
        let config_path = MarkdownApp::config_file_path();
        let mut config = AppConfig::default();

        let read_path = config_path
            .clone()
            .filter(|path| path.exists())
            .or_else(|| MarkdownApp::legacy_config_path().filter(|path| path.exists()));
        if let Some(path) = &read_path {
            {
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

        // Apps opened from a file manager, Finder, or the Start menu start in "/",
        // the program's folder, or a system folder: use the home folder instead.
        let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        let program_dir = env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        let default_dir = env::current_dir()
            .ok()
            .filter(|dir| dir.parent().is_some() && Some(dir) != program_dir.as_ref())
            .filter(|dir| {
                // C:\Windows\System32 and friends.
                !env::var_os("SystemRoot").is_some_and(|system| dir.starts_with(system))
            })
            .or(home)
            .unwrap_or_else(|| PathBuf::from("."));
        config.working_dir = Some(default_dir.clone());
        default_dir
    }

    fn config_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "quillwright", "quillwright")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Config written by md-echo, the project Quillwright was forked from.
    fn legacy_config_path() -> Option<PathBuf> {
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
        let t = &self.config.theme;
        let color = |value: &Option<String>| value.as_deref().and_then(parse_color);
        let overrides = theme::Overrides {
            background: color(&t.background),
            panel: color(&t.panel),
            text: color(&t.text),
            accent: color(&t.accent),
        };
        let palette = theme::resolve(theme::Preset::from_name(&t.base), &overrides);
        theme::apply(ctx, &palette, color(&t.hyperlink));
        self.theme_applied = true;
    }

    fn pick_project(&mut self) {
        if let Some(path) = FileDialog::new()
            .set_title("Open Project")
            .set_directory(&self.working_dir)
            .pick_folder()
        {
            self.run_or_prompt_discard(DiscardAction::OpenProject(path));
        }
    }

    fn activate_project(&mut self, root: PathBuf) {
        self.remember_project(&root);
        self.set_working_directory(root);
        self.index_dirty = true;
        self.show_home = true;
        self.external_load = true;
        self.content.clear();
        self.original_content.clear();
        self.file_path = None;
        self.modified = false;
        self.reset_view_state();
    }

    fn show_new_project_dialog(&mut self, ctx: &Context) {
        let Some(dialog) = &mut self.new_project else {
            return;
        };
        let mut open = true;
        let mut destination = None;
        let mut cancelled = false;
        egui::Window::new("New Project")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Project folder name");
                ui.text_edit_singleline(&mut dialog.name);
                ui.label("Create inside");
                ui.monospace(dialog.parent.display().to_string());
                if ui.button("Choose Folder...").clicked() {
                    if let Some(parent) = FileDialog::new()
                        .set_title("Choose Parent Folder")
                        .set_directory(&dialog.parent)
                        .pick_folder()
                    {
                        dialog.parent = parent;
                        dialog.error = None;
                    }
                }
                ui.label("Creates chapters/, characters/, world-rules.md and AGENT.md.");
                ui.label("Templates are optional. Existing folders will not be overwritten.");
                if let Some(error) = &dialog.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.pending_discard_action.is_none(),
                            egui::Button::new("Create"),
                        )
                        .clicked()
                    {
                        match project::destination(&dialog.parent, &dialog.name) {
                            Ok(path) => {
                                destination = Some(path);
                                dialog.error = None;
                            }
                            Err(err) => dialog.error = Some(err.to_string()),
                        }
                    }
                    cancelled = ui.button("Cancel").clicked();
                });
            });
        if !open || cancelled {
            self.new_project = None;
            if matches!(
                self.pending_discard_action,
                Some(DiscardAction::CreateProject(_))
            ) {
                self.pending_discard_action = None;
            }
        }
        if let Some(path) = destination {
            self.run_or_prompt_discard(DiscardAction::CreateProject(path));
        }
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

    fn render_directory(&mut self, ui: &mut egui::Ui, path: &Path, is_root: bool) {
        let name = if is_root {
            path.display().to_string()
        } else {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string())
        };

        let header = egui::CollapsingHeader::new(name)
            .id_salt(path.display().to_string())
            .default_open(is_root);

        header.show(ui, |ui| self.render_entries(ui, path));
    }

    fn render_entries(&mut self, ui: &mut egui::Ui, path: &Path) {
        match fs::read_dir(path) {
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
                        if reference::Kind::of(&file).viewer_only() {
                            self.open_reference(file.clone());
                        } else {
                            self.run_or_prompt_discard(DiscardAction::OpenPath(file.clone()));
                        }
                    }
                }
            }
            Err(err) => {
                ui.label(format!("Cannot read {}: {}", path.display(), err));
            }
        }
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

        let mut cmd = process::command(&command[0]);
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
                        self.update_modified_state();
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
        self.editor = editor::Editor::default();
        self.base_missing = false;
        self.disk_conflict = None;
        self.sync_notice = None;
        self.current_line = 0;
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
        self.modified = self.base_missing || self.content != self.original_content;
    }

    fn char_to_byte_index(text: &str, char_index: usize) -> usize {
        text.char_indices()
            .nth(char_index)
            .map_or(text.len(), |(byte_idx, _)| byte_idx)
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
        if self.content[start_byte..end_byte] != self.find_query {
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
            if let Some(last_line) = self.content.split_inclusive('\n').next_back() {
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
                self.show_home = false;
                self.external_load = true;
            }
            Err(err) => {
                self.show_tool_message(format!("Could not open {}: {err}", path.display()));
            }
        }
    }

    fn save_file(&mut self, save_as: bool) {
        if save_as || self.file_path.is_none() {
            if let Some(path) = FileDialog::new()
                .set_directory(
                    self.file_path
                        .as_deref()
                        .and_then(Path::parent)
                        .unwrap_or(&self.working_dir),
                )
                .save_file()
            {
                // The dialog already confirmed overwriting an existing file.
                match file_sync::write_checked(&path, &self.content, None) {
                    Ok(()) => {
                        self.file_path = Some(Self::canonicalize_path(&path));
                        self.mark_saved();
                    }
                    Err(disk) => self.show_tool_message(format!("Save error: {disk:?}")),
                }
            }
        } else if let Err(err) = self.save_in_place() {
            self.show_tool_message(format!("Save error: {err}"));
        }
    }

    fn mark_saved(&mut self) {
        self.index_dirty = true;
        self.original_content = self.content.clone();
        self.base_missing = false;
        self.disk_conflict = None;
        self.modified = false;
    }

    /// What we believe is on disk for the open file.
    fn expected_disk(&self) -> file_sync::Disk {
        if self.base_missing {
            file_sync::Disk::Missing
        } else {
            file_sync::Disk::Text(self.original_content.clone())
        }
    }

    /// Point the watcher at the current project/file and apply its reports.
    fn sync_with_disk(&mut self, ctx: &Context) {
        let monitor = self
            .monitor
            .get_or_insert_with(|| file_sync::Monitor::new(ctx.clone()));
        let focused = ctx.input(|i| i.focused);
        if focused && !self.window_focused {
            // Returning to the window is a good moment to double-check the disk.
            monitor.check();
        }
        if !focused && self.window_focused {
            self.saving.focus_lost = true;
        }
        self.window_focused = focused;

        let current = self.monitor_target.as_ref();
        if current.map(|t| (&t.root, &t.file)) != Some((&self.working_dir, &self.file_path)) {
            let target = file_sync::Target {
                generation: current.map_or(0, |t| t.generation + 1),
                root: self.working_dir.clone(),
                file: self.file_path.clone(),
            };
            monitor.configure(target.clone());
            self.monitor_target = Some(target);
        }

        let updates: Vec<_> = monitor.updates.try_iter().collect();
        for update in updates {
            if Some(&update.target) != self.monitor_target.as_ref() {
                continue; // Stale report for a previous file or project.
            }
            // Something in the project changed: refresh chapter titles and counts.
            self.index_dirty = true;
            if let Some(warning) = &update.warning {
                self.sync_notice = Some(format!("⚠ File watching degraded: {warning}"));
            }
            // Follow renames of the project folder or the open file.
            if update.root != self.working_dir && update.root.is_dir() {
                self.working_dir = update.root.clone();
                self.config.working_dir = Some(update.root.clone());
                self.save_config();
            }
            if update.file != self.file_path {
                self.file_path = update.file.clone();
            }
            if let Some(disk) = update.disk {
                self.apply_disk(disk);
            }
        }
    }

    fn apply_disk(&mut self, disk: file_sync::Disk) {
        use file_sync::{Decision, Disk};
        if disk == self.expected_disk() {
            // Disk is back to what we expect (e.g. an agent reverted its change).
            if !self.base_missing {
                self.disk_conflict = None;
            }
            return;
        }
        let decision = file_sync::reconcile(
            &self.original_content,
            &self.content,
            &disk,
            self.editor.is_composing(),
        );
        match (decision, disk) {
            (Decision::Unchanged, _) => {}
            (Decision::Accept, Disk::Text(text)) => {
                // The disk now matches the editor (e.g. our own save, or identical text).
                self.original_content = text;
                self.base_missing = false;
                self.disk_conflict = None;
                self.update_modified_state();
            }
            (Decision::Reload, Disk::Text(text)) => {
                self.reload_from_disk(text);
                self.sync_notice = Some("↻ Reloaded external changes".to_string());
            }
            (_, disk) => self.disk_conflict = Some(disk),
        }
    }

    /// Replace the buffer with disk contents, keeping the cursor near the same text.
    fn reload_from_disk(&mut self, text: String) {
        // Keep the text being replaced: the last saved version and any unsaved edits.
        let (saved, edited) = (self.original_content.clone(), self.content.clone());
        self.record_history(&saved, true);
        self.record_history(&edited, true);
        let selection = self.editor.selection;
        let map = |index| file_sync::map_cursor(&self.content, &text, index);
        self.editor.selection = egui::text::CCursorRange::two(
            egui::text::CCursor::new(map(selection.secondary.index.0)),
            egui::text::CCursor::new(map(selection.primary.index.0)),
        );
        self.content = text.clone();
        self.original_content = text;
        self.base_missing = false;
        self.disk_conflict = None;
        self.modified = false;
        self.external_load = true;
    }

    fn show_disk_conflict_dialog(&mut self, ctx: &Context) {
        let Some(disk) = self.disk_conflict.clone() else {
            return;
        };
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "This file".to_string());
        egui::Window::new("File Changed on Disk")
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                match &disk {
                    file_sync::Disk::Text(_) => ui.label(format!(
                        "{name} was changed outside Quillwright (for example, by an agent) \
                         while you had unsaved edits."
                    )),
                    file_sync::Disk::Missing => {
                        ui.label(format!("{name} was deleted or moved outside Quillwright."))
                    }
                    file_sync::Disk::Unreadable(err) => {
                        ui.label(format!("{name} can no longer be read: {err}"))
                    }
                };
                ui.horizontal(|ui| {
                    if let file_sync::Disk::Text(text) = &disk {
                        if ui
                            .button("Reload from Disk")
                            .on_hover_text("Discard your unsaved edits")
                            .clicked()
                        {
                            self.reload_from_disk(text.clone());
                        }
                    }
                    if ui
                        .button("Keep My Version")
                        .on_hover_text("Keep editing; saving will overwrite the disk version")
                        .clicked()
                    {
                        match &disk {
                            file_sync::Disk::Text(text) => {
                                self.original_content = text.clone();
                                self.base_missing = false;
                            }
                            file_sync::Disk::Missing => self.base_missing = true,
                            file_sync::Disk::Unreadable(_) => {}
                        }
                        self.disk_conflict = None;
                        self.update_modified_state();
                    }
                });
            });
    }

    fn run_or_prompt_discard(&mut self, action: DiscardAction) {
        if self.save_before_leaving() {
            self.perform_discard_action(action);
        } else if self.disk_conflict.is_none() {
            // A conflict opens its own dialog; otherwise ask what to do with the edits.
            self.pending_discard_action = Some(action);
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
                self.show_home = false;
                self.external_load = true;
            }
            DiscardAction::Home => {
                self.content.clear();
                self.original_content.clear();
                self.file_path = None;
                self.modified = false;
                self.reset_view_state();
                self.show_home = true;
                self.external_load = true;
            }
            DiscardAction::OpenDialog => {
                if let Some(path) = FileDialog::new()
                    .set_directory(&self.working_dir)
                    .pick_file()
                {
                    self.open_file_from_path(&path);
                }
            }
            DiscardAction::OpenPath(path) => {
                self.open_file_from_path(&path);
            }
            DiscardAction::CreateProject(path) => match project::create(&path) {
                Ok(root) => {
                    self.activate_project(root.clone());
                    self.open_file_from_path(&root.join(project::FIRST_CHAPTER));
                    self.new_project = None;
                }
                Err(err) => {
                    let message = format!("Could not create project {}: {err}", path.display());
                    if let Some(dialog) = &mut self.new_project {
                        dialog.error = Some(message);
                    } else {
                        self.show_tool_message(message);
                    }
                }
            },
            DiscardAction::OpenProject(path) => match project::open(&path) {
                Ok(root) => self.activate_project(root),
                Err(err) => self
                    .show_tool_message(format!("Could not open project {}: {err}", path.display())),
            },
        }
    }
}
