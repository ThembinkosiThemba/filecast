use egui::{CentralPanel, Context, Frame, Key, RichText, ScrollArea, TextEdit, Ui};

use crate::core::app::App;
use crate::core::search::SearchResultKind;
use crate::core::settings::{LauncherSettings, LauncherView, WindowPosition};
use crate::ui::theme;

const OUTER_MARGIN: f32 = 16.0;
const ITEM_HEIGHT: f32 = 36.0;

pub struct LauncherUI {
    pub selected_result: usize,
    pub selected_file: usize,
    pub selected_recent: usize,
    pub search_focused: bool,
    pub command_output: Option<String>,
    scroll_to_selected: bool,
    pub files_command_mode: bool,
    pub files_command_input: String,
}

impl Default for LauncherUI {
    fn default() -> Self {
        Self {
            selected_result: 0,
            selected_file: 0,
            selected_recent: 0,
            search_focused: true,
            command_output: None,
            scroll_to_selected: false,
            files_command_mode: false,
            files_command_input: String::new(),
        }
    }
}

impl LauncherUI {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &Context, app: &mut App, settings: &mut LauncherSettings) {
        // Configure theme on first frame
        theme::configure_style(ctx);

        // Handle keyboard shortcuts
        self.handle_global_keys(ctx, app, settings);

        CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(theme::BG_PRIMARY)
                    .inner_margin(egui::Margin::same(OUTER_MARGIN))
                    .rounding(theme::ROUNDING)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Navigation tabs
                    self.draw_tabs(ui, settings);

                    ui.add_space(theme::SPACING);

                    // View content
                    match settings.current_view {
                        LauncherView::Search => self.draw_search_view(ui, app),
                        LauncherView::Files => self.draw_files_view(ui, app),
                        LauncherView::Settings => self.draw_settings_view(ui, settings),
                    }
                });
            });
    }

    fn handle_global_keys(
        &mut self,
        ctx: &Context,
        app: &mut App,
        settings: &mut LauncherSettings,
    ) {
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                match settings.current_view {
                    LauncherView::Search => {
                        if !app.search_query.is_empty() {
                            app.search_query.clear();
                            app.search_results.clear();
                            self.selected_result = 0;
                            self.command_output = None;
                        } else if self.search_focused {
                            self.search_focused = false;
                        } else {
                            app.toggle_visibility();
                        }
                    }
                    LauncherView::Files | LauncherView::Settings => {
                        settings.current_view = LauncherView::Search;
                    }
                }
            }

            // Tab to cycle views (only when not in special modes)
            if i.key_pressed(Key::Tab) && !self.search_focused && !self.files_command_mode {
                settings.current_view = match settings.current_view {
                    LauncherView::Search => LauncherView::Files,
                    LauncherView::Files => LauncherView::Settings,
                    LauncherView::Settings => LauncherView::Search,
                };
            }

            // Ctrl+1/2/3 for quick view switching
            if i.modifiers.ctrl {
                if i.key_pressed(Key::Num1) {
                    settings.current_view = LauncherView::Search;
                }
                if i.key_pressed(Key::Num2) {
                    settings.current_view = LauncherView::Files;
                }
                if i.key_pressed(Key::Num3) {
                    settings.current_view = LauncherView::Settings;
                }
            }

            // View-specific keys
            match settings.current_view {
                LauncherView::Search => {
                    if !app.search_results.is_empty() {
                        if i.key_pressed(Key::ArrowDown) {
                            let max = app.search_results.len().saturating_sub(1);
                            self.selected_result = (self.selected_result + 1).min(max);
                            self.scroll_to_selected = true;
                        }
                        if i.key_pressed(Key::ArrowUp) {
                            self.selected_result = self.selected_result.saturating_sub(1);
                            self.scroll_to_selected = true;            // Tab to cycle views (only when not in special modes)

                        }
                        if i.key_pressed(Key::Enter) && !self.search_focused {
                            let _ = app.execute_search_result(self.selected_result);
                            app.search_query.clear();
                            app.search_results.clear();
                            self.selected_result = 0;
                        }
                    } else if app.search_query.is_empty() && !self.search_focused {
                        let recent_count = app.recent_files.len().min(5);
                        let app_count = app.applications.len().min(5);
                        let total = recent_count + app_count;

                        if total > 0 {
                            let current = self.selected_recent;

                            if i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::J) {
                                self.selected_recent = (current + 1) % total;
                                self.scroll_to_selected = true;
                            }
                            if i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::K) {
                                self.selected_recent = current.checked_sub(1).unwrap_or(total - 1);
                                self.scroll_to_selected = true;
                            }

                            if i.key_pressed(Key::Enter) {
                                if self.selected_recent < recent_count {
                                    if let Some(recent) = app.recent_files.get(self.selected_recent)
                                    {
                                        let path = recent.path.clone();
                                        if path.is_dir() {
                                            let _ = app.change_directory(path);
                                        } else {
                                            let _ = app.open_file(path);
                                        }
                                    }
                                } else {
                                    let app_idx = self.selected_recent - recent_count;
                                    if let Some(desktop_app) = app.applications.get(app_idx) {
                                        let _ = desktop_app.launch();
                                    }
                                }
                            }
                        }
                    }
                }
                LauncherView::Files => {
                    // Skip navigation if in command mode
                    if self.files_command_mode {
                        if i.key_pressed(Key::Escape) {
                            self.files_command_mode = false;
                            self.files_command_input.clear();
                        }
                        return;
                    }

                    let file_count = app.get_display_list().len();
                    let old_selection = self.selected_file;

                    // Down navigation
                    if i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::J) {
                        if file_count > 0 && self.selected_file < file_count.saturating_sub(1) {
                            self.selected_file += 1;
                        }
                    }

                    // Up navigation
                    if i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::K) {
                        if self.selected_file > 0 {
                            self.selected_file -= 1;
                        }
                    }

                    // Only scroll if selection changed
                    if self.selected_file != old_selection {
                        app.selected_index = self.selected_file;
                        self.scroll_to_selected = true;
                    }

                    // Enter directory / open file
                    if i.key_pressed(Key::Enter)
                        || i.key_pressed(Key::L)
                        || i.key_pressed(Key::ArrowRight)
                    {
                        let _ = app.enter_selected();
                        self.selected_file = 0;
                        self.scroll_to_selected = true;
                    }

                    // Go up directory
                    if i.key_pressed(Key::ArrowLeft)
                        || i.key_pressed(Key::H)
                        || i.key_pressed(Key::Backspace)
                    {
                        let _ = app.go_up();
                        self.selected_file = 0;
                        self.scroll_to_selected = true;
                    }

                    // R to refresh
                    if i.key_pressed(Key::R) {
                        let _ = app.refresh_directory();
                    }

                    // C to enter command mode
                    if i.key_pressed(Key::C) {
                        self.files_command_mode = true;
                        self.files_command_input.clear();
                        self.command_output = None;
                    }
                }
                LauncherView::Settings => {}
            }
        });
    }

    fn draw_tabs(&mut self, ui: &mut Ui, settings: &mut LauncherSettings) {
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(egui::Margin::symmetric(theme::PADDING, theme::SPACING))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [
                        (LauncherView::Search, "🔍 Search", "Ctrl+1"),
                        (LauncherView::Files, "📁 Files", "Ctrl+2"),
                        (LauncherView::Settings, "☰ Settings", "Ctrl+3"),
                    ];

                    for (view, label, shortcut) in tabs {
                        let is_active = settings.current_view == view;
                        let color = if is_active {
                            theme::ACCENT
                        } else {
                            theme::TEXT_SECONDARY
                        };

                        let response = ui.selectable_label(
                            is_active,
                            RichText::new(label).color(color).size(13.0),
                        );

                        if response.clicked() {
                            settings.current_view = view;
                        }

                        response.on_hover_text(shortcut);
                        ui.add_space(theme::SPACING);
                    }
                });
            });
    }

    fn draw_search_view(&mut self, ui: &mut Ui, app: &mut App) {
        self.draw_search_input(ui, app);
        ui.add_space(theme::SPACING);

        // Show search hints when empty
        if app.search_query.is_empty() && app.search_results.is_empty() {
            self.draw_recent_and_apps(ui, app);
        } else if app.search_query.starts_with(':') {
            self.draw_command_view(ui, app);
        } else if !app.search_results.is_empty() {
            self.draw_results(ui, app);
        } else if !app.search_query.is_empty() {
            self.draw_no_results(ui, &app.search_query);
        }
    }

    fn draw_command_view(&mut self, ui: &mut Ui, app: &mut App) {
        let command = app.search_query.strip_prefix(':').unwrap_or("").trim();

        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Command Mode")
                        .color(theme::ACCENT)
                        .size(14.0),
                );
                ui.add_space(theme::SPACING);

                if command.is_empty() {
                    ui.label(
                        RichText::new("Type a command and press Enter to execute")
                            .color(theme::TEXT_MUTED)
                            .size(12.0),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("$ {}", command))
                            .color(theme::TEXT_PRIMARY)
                            .size(13.0)
                            .monospace(),
                    );
                }
            });

        if let Some(output) = &self.command_output {
            ui.add_space(theme::SPACING);
            ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                Frame::none()
                    .fill(theme::BG_SECONDARY)
                    .rounding(theme::ROUNDING)
                    .inner_margin(theme::PADDING)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(output)
                                .color(theme::TEXT_PRIMARY)
                                .size(11.0)
                                .monospace(),
                        );
                    });
            });
        }
    }

    fn draw_files_view(&mut self, ui: &mut Ui, app: &mut App) {
        // Path header
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📂").size(16.0));
                    ui.add_space(theme::SPACING);
                    ui.label(
                        RichText::new(app.current_path.to_string_lossy())
                            .color(theme::TEXT_PRIMARY)
                            .size(13.0),
                    );
                });
            });

        ui.add_space(theme::SPACING);

        // Command input (if in command mode)
        let mut should_run_command = false;
        let mut command_to_run = String::new();

        if self.files_command_mode {
            Frame::none()
                .fill(theme::BG_SECONDARY)
                .rounding(theme::ROUNDING)
                .inner_margin(theme::PADDING)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("$").size(16.0).color(theme::ACCENT));
                        ui.add_space(theme::SPACING);

                        let response = ui.add_sized(
                            [ui.available_width(), 22.0],
                            TextEdit::singleline(&mut self.files_command_input)
                                .hint_text("Enter command and press Enter...")
                                .font(egui::FontId::monospace(14.0))
                                .frame(false)
                                .text_color(theme::TEXT_PRIMARY),
                        );

                        response.request_focus();

                        // Check for Enter key to run command
                        if ui.input(|i| i.key_pressed(Key::Enter))
                            && !self.files_command_input.is_empty()
                        {
                            should_run_command = true;
                            command_to_run = self.files_command_input.clone();
                        }
                    });
                });
            ui.add_space(theme::SPACING);
        }

        // Execute command outside the UI closure
        if should_run_command {
            self.execute_command_sync(&command_to_run, app);
            self.files_command_mode = false;
            self.files_command_input.clear();
        }

        // Command output
        if let Some(output) = &self.command_output {
            if !self.files_command_mode {
                ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                    Frame::none()
                        .fill(theme::BG_SECONDARY)
                        .rounding(theme::ROUNDING)
                        .inner_margin(theme::PADDING)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(output)
                                    .color(theme::TEXT_PRIMARY)
                                    .size(10.0)
                                    .monospace(),
                            );
                        });
                });
                ui.add_space(theme::SPACING);
            }
        }

        let mut action: Option<usize> = None;
        let selected = self.selected_file;

        let max_height = if self.command_output.is_some() && !self.files_command_mode {
            200.0
        } else {
            280.0
        };

        let files: Vec<_> = app
            .get_display_list()
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.name.clone(), f.is_dir, f.size))
            .collect();

        let file_count = files.len();
        let do_scroll = self.scroll_to_selected;
        self.scroll_to_selected = false;

        ScrollArea::vertical()
            .max_height(max_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, name, is_dir, size) in &files {
                    let is_selected = *idx == selected;
                    let bg_color = if is_selected {
                        theme::BG_SELECTED
                    } else {
                        theme::BG_PRIMARY
                    };

                    let response = Frame::none()
                        .fill(bg_color)
                        .rounding(theme::ROUNDING / 2.0)
                        .inner_margin(egui::Margin::symmetric(theme::PADDING, 4.0))
                        .show(ui, |ui| {
                            ui.set_min_height(ITEM_HEIGHT - 8.0);
                            ui.horizontal(|ui| {
                                let icon = if *is_dir { "📁" } else { "📄" };
                                ui.label(RichText::new(icon).size(14.0));
                                ui.add_space(theme::SPACING);
                                ui.label(
                                    RichText::new(name)
                                        .color(if is_selected {
                                            theme::ACCENT
                                        } else {
                                            theme::TEXT_PRIMARY
                                        })
                                        .size(13.0),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if !*is_dir {
                                            ui.label(
                                                RichText::new(format_size(*size))
                                                    .color(theme::TEXT_MUTED)
                                                    .size(11.0),
                                            );
                                        }
                                    },
                                );
                            });
                        });

                    // Scroll selected item into view
                    if is_selected && do_scroll {
                        ui.scroll_to_rect(response.response.rect, Some(egui::Align::Center));
                    }

                    if response.response.clicked() {
                        action = Some(*idx);
                    }
                    if response.response.hovered() && !is_selected {
                        self.selected_file = *idx;
                        app.selected_index = *idx;
                    }
                }

                // Ensure we have something to show even if empty
                if file_count == 0 {
                    ui.label(
                        RichText::new("Empty directory")
                            .color(theme::TEXT_MUTED)
                            .size(12.0),
                    );
                }
            });

        if let Some(idx) = action {
            self.selected_file = idx;
            app.selected_index = idx;
            let _ = app.enter_selected();
            self.selected_file = 0;
        }

        // Status bar
        ui.add_space(theme::SPACING);
        let hint = if self.files_command_mode {
            "Enter: run command | Esc: cancel"
        } else {
            "↑↓ jk: Navigate | →l: Open | ←h: Up | r: Refresh | c: Command"
        };
        ui.label(RichText::new(hint).color(theme::TEXT_MUTED).size(10.0));
    }

    fn draw_settings_view(&mut self, ui: &mut Ui, settings: &mut LauncherSettings) {
        ui.label(
            RichText::new("Settings")
                .color(theme::TEXT_PRIMARY)
                .size(18.0),
        );
        ui.add_space(theme::PADDING);

        // Position setting
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Window Position")
                        .color(theme::TEXT_PRIMARY)
                        .size(14.0),
                );
                ui.add_space(theme::SPACING);

                let positions = [
                    (WindowPosition::TopCenter, "Top Center"),
                    (WindowPosition::Center, "Center"),
                    (WindowPosition::TopLeft, "Top Left"),
                    (WindowPosition::TopRight, "Top Right"),
                    (WindowPosition::BottomCenter, "Bottom Center"),
                    (WindowPosition::BottomLeft, "Bottom Left"),
                    (WindowPosition::BottomRight, "Bottom Right"),
                ];

                ui.horizontal_wrapped(|ui| {
                    for (pos, label) in positions {
                        let is_selected = std::mem::discriminant(&settings.position)
                            == std::mem::discriminant(&pos);
                        if ui.selectable_label(is_selected, label).clicked() {
                            settings.position = pos;
                            settings.save();
                        }
                    }
                });

                ui.add_space(theme::SPACING);
                ui.label(
                    RichText::new("Restart required for position changes")
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
            });

        ui.add_space(theme::PADDING);

        // Search tips
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Search Syntax")
                        .color(theme::TEXT_PRIMARY)
                        .size(14.0),
                );
                ui.add_space(theme::SPACING);

                let tips = [
                    ("query", "Fuzzy search apps & files"),
                    ("@pattern", "Grep file contents"),
                    ("/name", "Find files by name"),
                    (":command", "Run shell command"),
                ];

                for (syntax, desc) in tips {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(syntax)
                                .color(theme::ACCENT)
                                .size(12.0)
                                .monospace(),
                        );
                        ui.label(
                            RichText::new(format!(" - {}", desc))
                                .color(theme::TEXT_SECONDARY)
                                .size(12.0),
                        );
                    });
                }
            });

        ui.add_space(theme::PADDING);

        // Shortcuts
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Keyboard Shortcuts")
                        .color(theme::TEXT_PRIMARY)
                        .size(14.0),
                );
                ui.add_space(theme::SPACING);

                let shortcuts = [
                    ("Super+Space", "Toggle Filecast"),
                    ("Ctrl+1/2/3", "Switch views"),
                    ("Escape", "Clear / Unfocus / Hide"),
                    ("↑/↓", "Navigate"),
                    ("Enter", "Execute / Open"),
                ];

                for (key, action) in shortcuts {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(key).color(theme::ACCENT).size(12.0));
                        ui.label(
                            RichText::new(format!(" - {}", action))
                                .color(theme::TEXT_SECONDARY)
                                .size(12.0),
                        );
                    });
                }
            });
    }

    fn draw_search_input(&mut self, ui: &mut Ui, app: &mut App) {
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let icon = match app.search_query.chars().next() {
                        Some(':') => ">",
                        Some('@') => "🔎",
                        Some('/') => "📂",
                        _ => "🔍",
                    };
                    ui.label(RichText::new(icon).size(18.0).color(theme::TEXT_SECONDARY));
                    ui.add_space(theme::SPACING);

                    let response = ui.add_sized(
                        [ui.available_width(), 24.0],
                        TextEdit::singleline(&mut app.search_query)
                            .hint_text("Search apps, files... (@grep, /find, :cmd)")
                            .font(theme::search_input_font())
                            .frame(false)
                            .text_color(theme::TEXT_PRIMARY),
                    );

                    self.search_focused = response.has_focus();

                    if app.window_visible && self.search_focused {
                        response.request_focus();
                    }

                    // Handle Enter
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        if app.search_query.starts_with(':') {
                            let command = app
                                .search_query
                                .strip_prefix(':')
                                .unwrap_or("")
                                .trim()
                                .to_string();
                            if !command.is_empty() {
                                self.execute_command_sync(&command, app);
                            }
                        } else if !app.search_results.is_empty() {
                            let _ = app.execute_search_result(self.selected_result);
                            app.search_query.clear();
                            app.search_results.clear();
                            self.selected_result = 0;
                        }
                    }

                    if response.changed() {
                        if !app.search_query.starts_with(':') {
                            app.update_search(&app.search_query.clone());
                        }
                        self.selected_result = 0;
                        self.command_output = None;
                    }
                });
            });
    }

    fn execute_command_sync(&mut self, command: &str, app: &mut App) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let output = std::process::Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(&app.current_path)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    if stdout.is_empty() {
                        self.command_output = Some("(no output)".to_string());
                    } else {
                        self.command_output = Some(stdout.to_string());
                    }
                } else {
                    self.command_output = Some(format!("Error:\n{}{}", stdout, stderr));
                }

                let _ = app.refresh_directory();
            }
            Err(e) => {
                self.command_output = Some(format!("Failed: {}", e));
            }
        }
    }

    fn draw_results(&mut self, ui: &mut Ui, app: &mut App) {
        let mut clicked_idx: Option<usize> = None;
        let selected = self.selected_result;

        let results_data: Vec<_> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let type_label = match &result.kind {
                    SearchResultKind::File(_) => "file",
                    SearchResultKind::RecentFile(_) => "recent",
                    SearchResultKind::Application(_) => "app",
                    SearchResultKind::Command(_) => "cmd",
                    SearchResultKind::GrepResult { .. } => "grep",
                };
                (
                    idx,
                    result.icon.clone(),
                    result.name.clone(),
                    result.description.clone(),
                    type_label,
                )
            })
            .collect();

        ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, icon, name, description, type_text) in &results_data {
                    let is_selected = *idx == selected;
                    let bg_color = if is_selected {
                        theme::BG_SELECTED
                    } else {
                        theme::BG_PRIMARY
                    };

                    let response = Frame::none()
                        .fill(bg_color)
                        .rounding(theme::ROUNDING / 2.0)
                        .inner_margin(egui::Margin::symmetric(theme::PADDING, 6.0))
                        .show(ui, |ui| {
                            ui.set_min_height(ITEM_HEIGHT - 12.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(icon)
                                        .size(theme::ICON_SIZE)
                                        .color(theme::TEXT_PRIMARY),
                                );
                                ui.add_space(theme::SPACING);

                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(name).font(theme::result_name_font()).color(
                                            if is_selected {
                                                theme::ACCENT
                                            } else {
                                                theme::TEXT_PRIMARY
                                            },
                                        ),
                                    );
                                    ui.label(
                                        RichText::new(description)
                                            .font(theme::result_desc_font())
                                            .color(theme::TEXT_MUTED),
                                    );
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(*type_text)
                                                .font(theme::result_desc_font())
                                                .color(theme::TEXT_MUTED),
                                        );
                                    },
                                );
                            });
                        });

                    if is_selected && self.scroll_to_selected {
                        response.response.scroll_to_me(Some(egui::Align::Center));
                    }

                    let rect = response.response.rect;
                    let interact = ui.interact(rect, ui.id().with(idx), egui::Sense::click());
                    if interact.clicked() {
                        clicked_idx = Some(*idx);
                    }
                    if interact.hovered() {
                        self.selected_result = *idx;
                    }
                }
            });

        self.scroll_to_selected = false;

        if let Some(idx) = clicked_idx {
            let _ = app.execute_search_result(idx);
            app.search_query.clear();
            app.search_results.clear();
            self.selected_result = 0;
        }
    }

    fn draw_no_results(&mut self, ui: &mut Ui, query: &str) {
        Frame::none()
            .fill(theme::BG_SECONDARY)
            .rounding(theme::ROUNDING)
            .inner_margin(theme::PADDING)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(theme::PADDING);
                    ui.label(
                        RichText::new(format!("No results for \"{}\"", query))
                            .color(theme::TEXT_MUTED)
                            .size(14.0),
                    );
                    ui.add_space(theme::SPACING);
                    ui.label(
                        RichText::new("Try: @pattern (grep) or /name (find)")
                            .color(theme::TEXT_MUTED)
                            .size(11.0),
                    );
                    ui.add_space(theme::PADDING);
                });
            });
    }

    fn draw_recent_and_apps(&mut self, ui: &mut Ui, app: &mut App) {
        let recent_count = app.recent_files.len().min(5);

        let recent_data: Vec<_> = app
            .recent_files
            .iter()
            .take(5)
            .enumerate()
            .map(|(idx, recent)| {
                let name = recent
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| recent.path.to_string_lossy().to_string());
                let path = recent.path.clone();
                let is_dir = path.is_dir();
                (idx, name, path, is_dir)
            })
            .collect();

        let apps_data: Vec<_> = app
            .applications
            .iter()
            .take(5)
            .enumerate()
            .map(|(idx, a)| (idx, a.name.clone(), a.clone()))
            .collect();

        let mut clicked_recent: Option<(std::path::PathBuf, bool)> = None;
        let mut clicked_app: Option<crate::core::apps::DesktopApp> = None;

        ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if !recent_data.is_empty() {
                    ui.label(
                        RichText::new("Recent")
                            .color(theme::TEXT_SECONDARY)
                            .size(11.0),
                    );
                    ui.add_space(4.0);

                    for (idx, name, path, is_dir) in &recent_data {
                        let is_selected = !self.search_focused && self.selected_recent == *idx;
                        let bg_color = if is_selected {
                            theme::BG_SELECTED
                        } else {
                            theme::BG_PRIMARY
                        };

                        let response = Frame::none()
                            .fill(bg_color)
                            .rounding(theme::ROUNDING / 2.0)
                            .inner_margin(egui::Margin::symmetric(theme::PADDING, 4.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let icon = if *is_dir { "📁" } else { "📄" };
                                    ui.label(RichText::new(icon).size(14.0));
                                    ui.add_space(theme::SPACING);
                                    ui.label(
                                        RichText::new(name)
                                            .color(if is_selected {
                                                theme::ACCENT
                                            } else {
                                                theme::TEXT_PRIMARY
                                            })
                                            .size(13.0),
                                    );
                                });
                            });

                        if is_selected && self.scroll_to_selected {
                            response.response.scroll_to_me(Some(egui::Align::Center));
                        }

                        if response.response.clicked() {
                            clicked_recent = Some((path.clone(), *is_dir));
                        }
                        if response.response.hovered() {
                            self.selected_recent = *idx;
                        }
                    }

                    ui.add_space(theme::SPACING);
                }

                ui.label(
                    RichText::new("Applications")
                        .color(theme::TEXT_SECONDARY)
                        .size(11.0),
                );
                ui.add_space(4.0);

                for (idx, name, desktop_app) in &apps_data {
                    let global_idx = recent_count + *idx;
                    let is_selected = !self.search_focused && self.selected_recent == global_idx;
                    let bg_color = if is_selected {
                        theme::BG_SELECTED
                    } else {
                        theme::BG_PRIMARY
                    };

                    let response = Frame::none()
                        .fill(bg_color)
                        .rounding(theme::ROUNDING / 2.0)
                        .inner_margin(egui::Margin::symmetric(theme::PADDING, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🚀").size(14.0));
                                ui.add_space(theme::SPACING);
                                ui.label(
                                    RichText::new(name)
                                        .color(if is_selected {
                                            theme::ACCENT
                                        } else {
                                            theme::TEXT_PRIMARY
                                        })
                                        .size(13.0),
                                );
                            });
                        });

                    if is_selected && self.scroll_to_selected {
                        response.response.scroll_to_me(Some(egui::Align::Center));
                    }

                    if response.response.clicked() {
                        clicked_app = Some(desktop_app.clone());
                    }
                    if response.response.hovered() {
                        self.selected_recent = global_idx;
                    }
                }

                ui.add_space(theme::PADDING);
                ui.label(
                    RichText::new("Esc: unfocus search | ↑↓: navigate | Enter: open")
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
            });

        self.scroll_to_selected = false;

        if let Some((path, is_dir)) = clicked_recent {
            if is_dir {
                let _ = app.change_directory(path);
            } else {
                let _ = app.open_file(path);
            }
        }
        if let Some(desktop_app) = clicked_app {
            let _ = desktop_app.launch();
        }
    }
}

fn format_size(size: u64) -> String {
    const K: u64 = 1024;
    const M: u64 = K * 1024;
    const G: u64 = M * 1024;

    if size >= G {
        format!("{:.1}G", size as f64 / G as f64)
    } else if size >= M {
        format!("{:.1}M", size as f64 / M as f64)
    } else if size >= K {
        format!("{:.1}K", size as f64 / K as f64)
    } else {
        format!("{}B", size)
    }
}
