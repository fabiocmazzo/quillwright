//! The assistant side panel: actions, the author's instruction, streaming output,
//! and accept / discard of suggestions.
use crate::{
    assistant::{self, Action, ResultKind},
    fonts, manuscript, theme, MarkdownApp,
};
use eframe::egui;
use egui::{text::LayoutJob, Align, Button, Context, FontId, Layout, RichText, Stroke, TextFormat, Ui};

#[derive(Default)]
pub struct AssistantState {
    pub open: bool,
    instruction: String,
    job: Option<assistant::Job>,
    request: Option<assistant::Request>,
    prompt: String,
    result: Option<String>,
    error: Option<String>,
    details: String,
    show_settings: bool,
    command_line: String,
    prompt_as_argument: bool,
    focus_instruction: bool,
}

impl AssistantState {
    pub fn running(&self) -> bool {
        self.job.is_some()
    }
}

enum PanelAction {
    Send(Action),
    Cancel,
    Retry,
    Replace,
    Insert,
    Copy(String),
    Discard,
    Close,
    SaveSettings,
}

impl MarkdownApp {
    pub(crate) fn toggle_assistant(&mut self) {
        let state = &mut self.assistant;
        state.open = !state.open;
        if state.open {
            state.focus_instruction = true;
            if self.config.assistant.command.is_none() {
                state.show_settings = true;
            }
        }
    }

    pub(crate) fn open_assistant_settings(&mut self) {
        self.assistant.open = true;
        self.assistant.show_settings = true;
    }

    /// Collect output from a running command. Called every frame.
    pub(crate) fn poll_assistant(&mut self) {
        let state = &mut self.assistant;
        let Some(job) = &mut state.job else {
            return;
        };
        job.poll();
        let Some(result) = job.finished.clone() else {
            return;
        };
        state.details = job.stderr.trim().to_string();
        match result {
            Ok(()) => {
                let text = assistant::clean_output(&job.output);
                if text.is_empty() {
                    state.error = Some("The assistant returned an empty answer.".into());
                } else {
                    state.result = Some(text);
                }
            }
            Err(err) if err == "Cancelled" => {}
            Err(err) => {
                let tail: String = {
                    let lines: Vec<&str> = state.details.lines().collect();
                    lines[lines.len().saturating_sub(4)..].join("\n")
                };
                state.error = Some(if tail.is_empty() {
                    err
                } else {
                    format!("{err}:\n{tail}")
                });
            }
        }
        state.job = None;
    }

    /// Start a request. Public so menus and shortcuts can trigger actions directly.
    pub(crate) fn send_assistant(&mut self, ctx: &Context, action: Action) {
        let Some(command) = self.config.assistant.command.clone().filter(|c| !c.is_empty())
        else {
            self.open_assistant_settings();
            return;
        };
        let has_selection = self.selection_start != self.selection_end;
        if action.needs_selection() && !has_selection {
            self.assistant.open = true;
            self.assistant.error = Some("Select a passage in the editor first.".into());
            return;
        }
        let selected = self.selected_text().unwrap_or_default();
        let request = assistant::Request {
            action,
            instruction: self.assistant.instruction.trim().to_string(),
            file: self
                .file_path
                .as_ref()
                .map(|p| p.strip_prefix(&self.working_dir).unwrap_or(p).to_path_buf()),
            chapter_title: self.document_title(),
            start: self.selection_start,
            end: self.selection_end,
            selected,
        };
        self.run_request(ctx, request, &command);
    }

    fn run_request(&mut self, ctx: &Context, request: assistant::Request, command: &[String]) {
        let characters: Vec<_> = self
            .index
            .characters
            .iter()
            .map(|e| (e.path.clone(), e.title.clone()))
            .collect();
        let context = assistant::Context::gather(&self.working_dir, &self.content, &characters);
        let prompt = assistant::build_prompt(&request, &self.content, &context);
        let state = &mut self.assistant;
        state.open = true;
        state.result = None;
        state.error = None;
        state.details.clear();
        state.prompt = prompt.clone();
        match assistant::Job::start(&self.config.assistant, &self.working_dir, prompt, ctx.clone())
        {
            Ok(job) => {
                state.job = Some(job);
                if request.action == Action::Ask {
                    state.instruction.clear();
                }
                state.request = Some(request);
            }
            Err(err) => {
                let hint = if err.kind() == std::io::ErrorKind::NotFound {
                    " Is it installed and on your PATH? Check the assistant settings."
                } else {
                    ""
                };
                state.error = Some(format!("Could not start `{}`: {err}.{hint}", command[0]));
            }
        }
    }

    pub(crate) fn show_assistant_panel(&mut self, ctx: &Context) {
        let p = theme::current(ctx);
        let mut action = None;
        egui::SidePanel::right("assistant")
            .resizable(true)
            .default_width(360.0)
            .width_range(280.0..=640.0)
            .frame(
                egui::Frame::none()
                    .fill(p.desk)
                    .inner_margin(egui::Margin::symmetric(14.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Assistant")
                            .font(FontId::new(19.0, fonts::family(true, false)))
                            .color(p.text),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if quiet(ui, "Close", &p).clicked() {
                            action = Some(PanelAction::Close);
                        }
                        let label = if self.assistant.show_settings { "Done" } else { "Settings" };
                        if quiet(ui, label, &p).clicked() {
                            let state = &mut self.assistant;
                            state.show_settings = !state.show_settings;
                        }
                    });
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if self.assistant.show_settings || self.config.assistant.command.is_none() {
                            self.assistant_settings(ui, &p, &mut action);
                            if self.config.assistant.command.is_none() {
                                return;
                            }
                            ui.add_space(8.0);
                            ui.separator();
                        }
                        self.assistant_body(ui, &p, &mut action);
                    });
            });
        if let Some(action) = action {
            self.apply_panel_action(ctx, action);
        }
    }

    fn assistant_settings(&mut self, ui: &mut Ui, p: &theme::Palette, action: &mut Option<PanelAction>) {
        let state = &mut self.assistant;
        if state.command_line.is_empty() {
            if let Some(command) = &self.config.assistant.command {
                state.command_line = assistant::join_command(command);
                state.prompt_as_argument = self.config.assistant.prompt_as_argument;
            }
        }
        muted(
            ui,
            "Quillwright runs an AI tool installed on your computer and shows its answer \
             here. Your manuscript only changes when you accept a suggestion.",
            p,
        );
        ui.add_space(8.0);
        heading(ui, "Choose a tool", p);
        ui.horizontal_wrapped(|ui| {
            for preset in assistant::PRESETS {
                if ui.button(preset.name).on_hover_text(preset.note).clicked() {
                    let parts: Vec<String> = preset.command.iter().map(|s| s.to_string()).collect();
                    state.command_line = assistant::join_command(&parts);
                    state.prompt_as_argument = preset.prompt_as_argument;
                }
            }
        });
        ui.add_space(6.0);
        heading(ui, "Command", p);
        ui.add(
            egui::TextEdit::singleline(&mut state.command_line)
                .font(egui::TextStyle::Monospace)
                .hint_text("claude -p")
                .desired_width(f32::INFINITY),
        );
        ui.checkbox(
            &mut state.prompt_as_argument,
            "Pass the prompt as the last argument (default: stdin)",
        );
        muted(
            ui,
            "The command runs in the project folder. Login and API keys stay with the tool.",
            p,
        );
        ui.add_space(4.0);
        if ui
            .add(Button::new(RichText::new("Save").color(p.page)).fill(p.accent))
            .clicked()
        {
            *action = Some(PanelAction::SaveSettings);
        }
    }

    fn assistant_body(&mut self, ui: &mut Ui, p: &theme::Palette, action: &mut Option<PanelAction>) {
        if self.file_path.is_none() && self.content.trim().is_empty() {
            muted(ui, "Open a chapter to work with the assistant.", p);
            return;
        }
        let has_selection = self.selection_start != self.selection_end;
        let selected_words = if has_selection {
            self.selected_text()
                .map(|t| manuscript::word_count(&t))
                .unwrap_or(0)
        } else {
            0
        };
        let target = if has_selection {
            format!("{} words selected", manuscript::group(selected_words))
        } else {
            format!("cursor on line {}", self.current_line + 1)
        };
        muted(ui, &format!("“{}” · {target}", self.document_title()), p);
        ui.add_space(6.0);

        let running = self.assistant.running();
        ui.columns(2, |cols| {
            let actions = [
                (0, Action::Rewrite, "Rewrite the selected passage"),
                (1, Action::Continue, "Write what comes next at the cursor"),
                (0, Action::Alternatives, "Three different versions of the selection"),
                (1, Action::Continuity, "Review this chapter against the canon"),
            ];
            for (col, act, tip) in actions {
                let enabled = !running && (!act.needs_selection() || has_selection);
                let tip = if act.needs_selection() && !has_selection {
                    "Select a passage first"
                } else {
                    tip
                };
                let ui = &mut cols[col];
                if ui
                    .add_enabled(
                        enabled,
                        Button::new(act.label()).min_size(egui::vec2(ui.available_width(), 30.0)),
                    )
                    .on_hover_text(tip)
                    .on_disabled_hover_text(tip)
                    .clicked()
                {
                    *action = Some(PanelAction::Send(act));
                }
            }
        });
        ui.add_space(6.0);
        let response = ui.add(
            egui::TextEdit::multiline(&mut self.assistant.instruction)
                .hint_text("Ask anything, or guide the buttons above (“more tension, shorter sentences”)")
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        if self.assistant.focus_instruction {
            response.request_focus();
            self.assistant.focus_instruction = false;
        }
        let ctrl_enter = response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
        ui.horizontal(|ui| {
            let can_ask = !running && !self.assistant.instruction.trim().is_empty();
            if ui
                .add_enabled(can_ask, Button::new("Ask  (Ctrl+Enter)"))
                .clicked()
                || (ctrl_enter && can_ask)
            {
                *action = Some(PanelAction::Send(Action::Ask));
            }
        });
        let tool = self
            .config
            .assistant
            .command
            .as_ref()
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or_default();
        let summary = {
            let characters: Vec<_> = self
                .index
                .characters
                .iter()
                .map(|e| (e.path.clone(), e.title.clone()))
                .collect();
            // Cheap enough: a few small files, and only while the panel is open.
            assistant::Context::gather(&self.working_dir, &self.content, &characters).summary()
        };
        let sends = if summary.is_empty() {
            "this chapter".to_string()
        } else {
            format!("this chapter, {summary}")
        };
        ui.label(
            RichText::new(format!("Sends {sends} to `{tool}`."))
                .size(11.5)
                .color(p.muted),
        );

        ui.add_space(10.0);
        let state = &self.assistant;
        if let Some(job) = &state.job {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Writing…").color(p.muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        *action = Some(PanelAction::Cancel);
                    }
                });
            });
            if !job.output.trim().is_empty() {
                ui.label(RichText::new(job.output.trim()).color(p.muted));
            }
        }
        if let Some(error) = &state.error {
            ui.colored_label(p.accent, error);
            if !state.details.is_empty() {
                egui::CollapsingHeader::new(RichText::new("Details").size(12.0).color(p.muted))
                    .id_source("assistant_details")
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(&state.details).monospace().size(11.0))
                                .selectable(true),
                        );
                    });
            }
        }
        if let (Some(result), Some(request)) = (&state.result, &state.request) {
            self.show_suggestion(ui, p, result, request, action);
        }
        if !state.prompt.is_empty() {
            ui.add_space(8.0);
            egui::CollapsingHeader::new(RichText::new("Prompt sent").size(12.0).color(p.muted))
                .id_source("assistant_prompt")
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&state.prompt).monospace().size(11.0))
                            .selectable(true),
                    );
                });
        }
    }

    fn show_suggestion(
        &self,
        ui: &mut Ui,
        p: &theme::Palette,
        result: &str,
        request: &assistant::Request,
        action: &mut Option<PanelAction>,
    ) {
        let kind = request.action.result_kind();
        heading(
            ui,
            match kind {
                ResultKind::Replacement => "Suggested revision",
                ResultKind::Insertion => "Suggested continuation",
                ResultKind::Notes => "Answer",
            },
            p,
        );
        let same_file = request.file.as_ref().map(|f| self.working_dir.join(f)) == self.file_path
            || (request.file.is_none() && self.file_path.is_none());
        egui::Frame::none()
            .fill(p.page)
            .rounding(8.0)
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if kind == ResultKind::Replacement {
                    ui.label(diff_job(&request.selected, result, p));
                } else {
                    ui.add(
                        egui::Label::new(
                            RichText::new(result)
                                .font(FontId::new(15.0, fonts::family(false, false)))
                                .color(p.text),
                        )
                        .selectable(true),
                    );
                }
            });
        ui.add_space(6.0);
        let primary = |text: &str| Button::new(RichText::new(text).color(p.page)).fill(p.accent);
        ui.horizontal_wrapped(|ui| {
            let can_replace = same_file && request.start != request.end;
            match kind {
                ResultKind::Replacement => {
                    if ui.add_enabled(can_replace, primary("Replace selection")).clicked() {
                        *action = Some(PanelAction::Replace);
                    }
                }
                ResultKind::Insertion => {
                    if ui.add_enabled(same_file, primary("Insert at cursor")).clicked() {
                        *action = Some(PanelAction::Insert);
                    }
                }
                ResultKind::Notes if request.action == Action::Ask => {
                    if can_replace && ui.button("Replace selection").clicked() {
                        *action = Some(PanelAction::Replace);
                    }
                    if same_file && ui.button("Insert at cursor").clicked() {
                        *action = Some(PanelAction::Insert);
                    }
                }
                ResultKind::Notes => {}
            }
            if ui.button("Copy").clicked() {
                *action = Some(PanelAction::Copy(result.to_string()));
            }
            if ui.button("Try again").clicked() {
                *action = Some(PanelAction::Retry);
            }
            if ui.button("Discard").clicked() {
                *action = Some(PanelAction::Discard);
            }
        });
        if !same_file {
            muted(ui, "This answer is for another file; open it again to apply.", p);
        }
    }

    fn apply_panel_action(&mut self, ctx: &Context, action: PanelAction) {
        match action {
            PanelAction::Send(act) => self.send_assistant(ctx, act),
            PanelAction::Cancel => {
                if let Some(job) = &self.assistant.job {
                    job.cancel();
                }
            }
            PanelAction::Retry => {
                if let (Some(request), Some(command)) = (
                    self.assistant.request.clone(),
                    self.config.assistant.command.clone(),
                ) {
                    self.run_request(ctx, request, &command);
                }
            }
            PanelAction::Replace => {
                let (Some(result), Some(request)) =
                    (self.assistant.result.clone(), self.assistant.request.clone())
                else {
                    return;
                };
                match assistant::apply_replacement(&mut self.content, &request, &result) {
                    Ok((start, end)) => self.accepted_suggestion(start, end),
                    Err(err) => self.assistant.error = Some(err),
                }
            }
            PanelAction::Insert => {
                let Some(result) = self.assistant.result.clone() else {
                    return;
                };
                let at = self.selection_end;
                let (start, end) = assistant::apply_insertion(&mut self.content, at, &result);
                self.accepted_suggestion(start, end);
            }
            PanelAction::Copy(text) => ctx.output_mut(|o| o.copied_text = text),
            PanelAction::Discard => {
                self.assistant.result = None;
                self.assistant.error = None;
            }
            PanelAction::Close => self.assistant.open = false,
            PanelAction::SaveSettings => {
                let state = &mut self.assistant;
                let parts = assistant::split_command(&state.command_line);
                self.config.assistant.command = (!parts.is_empty()).then_some(parts);
                self.config.assistant.prompt_as_argument = state.prompt_as_argument;
                state.show_settings = self.config.assistant.command.is_none();
                state.error = None;
                self.save_config();
            }
        }
    }

    /// The suggestion is in the buffer: select it so the author sees what changed.
    fn accepted_suggestion(&mut self, start: usize, end: usize) {
        self.pending_selection = Some((start, end));
        self.update_modified_state();
        // Accepted AI text is not counted as words the author typed this session.
        self.external_load = true;
        self.assistant.result = None;
        self.assistant.error = None;
    }
}

fn quiet(ui: &mut Ui, text: &str, p: &theme::Palette) -> egui::Response {
    ui.add(Button::new(RichText::new(text).size(12.5).color(p.muted)).frame(false))
}

fn muted(ui: &mut Ui, text: &str, p: &theme::Palette) {
    ui.label(RichText::new(text).size(12.5).color(p.muted));
}

fn heading(ui: &mut Ui, text: &str, p: &theme::Palette) {
    ui.add_space(4.0);
    ui.label(RichText::new(text.to_uppercase()).size(11.0).strong().color(p.muted));
}

/// Word-level diff: removed words struck through, new words in the accent color.
pub(crate) fn diff_job(old: &str, new: &str, p: &theme::Palette) -> LayoutJob {
    let font = FontId::new(15.0, fonts::family(false, false));
    let mut job = LayoutJob::default();
    let diff = similar::TextDiff::configure()
        .timeout(std::time::Duration::from_millis(100))
        .diff_words(old, new);
    for change in diff.iter_all_changes() {
        let format = match change.tag() {
            similar::ChangeTag::Equal => TextFormat::simple(font.clone(), p.text),
            similar::ChangeTag::Delete => TextFormat {
                font_id: font.clone(),
                color: p.muted.gamma_multiply(0.8),
                strikethrough: Stroke::new(1.0_f32, p.muted),
                ..Default::default()
            },
            similar::ChangeTag::Insert => TextFormat {
                font_id: font.clone(),
                color: p.accent,
                background: theme::mix(p.page, p.accent, 0.12),
                ..Default::default()
            },
        };
        job.append(change.value(), 0.0, format);
    }
    job
}
