use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::Connection;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use super::event::AppEvent;
use super::mode::AppMode;
use crate::core::fs::{self, DirEntry};
use crate::core::history::{self as history_fs, RecentAccess};

#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPane {
    History,
    FileList,
    Preview,
}

#[derive(Debug, Clone)]
pub enum PreviewState {
    None,
    Text(String),
    Summary(String),
}

pub struct App {
    // Core State
    pub current_path: PathBuf,
    pub file_list: Vec<DirEntry>,
    pub selected_index: usize,
    pub mode: AppMode,
    pub should_quit: bool,
    pub tick_rate: Duration,
    pub status_message: String,

    // UI State
    pub focused_pane: FocusedPane,
    pub history_selected_index: usize,

    // History State (Temporary Navigation)
    pub history: Vec<PathBuf>,
    pub history_index: usize,

    // Persistent State (Recent Access)
    pub recent_files: Vec<RecentAccess>,
    pub db_connection: Connection,

    // Feature State
    pub preview_state: PreviewState,
    pub search_query: String,
    pub command_input: String,
    pub show_hidden: bool,
    pub filtered_file_list: Vec<DirEntry>,
    pub is_filtering: bool,

    // Tab Completion State
    pub completion_candidates: Vec<String>,
    pub completion_index: usize,
}

impl App {
    pub fn new(db_conn: Connection) -> Result<Self> {
        let initial_path = std::env::current_dir()?;
        let initial_list = fs::read_directory(&initial_path, false)?;
        let recent_files = history_fs::get_recent_files(&db_conn, 10).unwrap_or_default();

        Ok(App {
            current_path: initial_path.clone(),
            file_list: initial_list,
            selected_index: 0,
            mode: AppMode::Normal,
            should_quit: false,
            tick_rate: Duration::from_millis(250),
            status_message: String::from("Welcome to files TUI!"),

            focused_pane: FocusedPane::FileList,
            history_selected_index: 0,

            history: vec![initial_path],
            history_index: 0,

            recent_files,
            db_connection: db_conn,

            preview_state: PreviewState::None,
            search_query: String::new(),
            command_input: String::new(),
            show_hidden: false,
            filtered_file_list: Vec::new(),
            is_filtering: false,

            completion_candidates: Vec::new(),
            completion_index: 0,
        })
    }

    pub async fn update(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Quit => self.should_quit = true,
            AppEvent::Tick => self.on_tick(),
            AppEvent::Key(key_event) => self.handle_key_event(key_event).await?,
            AppEvent::DirectoryLoaded(path, entries) => self.load_directory(path, entries),
            AppEvent::FileOpened(path) => self.handle_file_opened(path)?,
            AppEvent::HistoryBack => self.navigate_history(false)?,
            AppEvent::HistoryForward => self.navigate_history(true)?,
            AppEvent::SearchInput(input) => self.search_query = input,
            AppEvent::CommandInput(input) => self.command_input = input,
            AppEvent::CommandExecute => self.execute_command().await?,
            _ => {}
        }
        Ok(())
    }

    fn on_tick(&mut self) {
        self.recent_files =
            history_fs::get_recent_files(&self.db_connection, 10).unwrap_or_default();
    }

    fn load_directory(&mut self, path: PathBuf, entries: Vec<DirEntry>) {
        self.current_path = path;
        self.file_list = entries;
        self.selected_index = 0;
        self.update_preview();
    }

    pub fn change_directory(&mut self, new_path: PathBuf) -> Result<()> {
        let entries = fs::read_directory(&new_path, self.show_hidden)?;
        self.push_to_history(new_path.clone());
        self.load_directory(new_path, entries);
        self.status_message = format!("Changed directory to: {}", self.current_path.display());
        Ok(())
    }

    pub fn refresh_directory(&mut self) -> Result<()> {
        let entries = fs::read_directory(&self.current_path, self.show_hidden)?;
        self.file_list = entries;
        self.selected_index = 0;
        self.update_preview();
        Ok(())
    }

    fn push_to_history(&mut self, path: PathBuf) {
        self.history.truncate(self.history_index + 1);
        self.history.push(path);
        self.history_index = self.history.len() - 1;
    }

    fn navigate_history(&mut self, forward: bool) -> Result<()> {
        let new_index = if forward {
            self.history_index.saturating_add(1)
        } else {
            self.history_index.saturating_sub(1)
        };

        if new_index < self.history.len() {
            self.history_index = new_index;
            let new_path = self.history[new_index].clone();
            let entries = fs::read_directory(&new_path, self.show_hidden)?;
            self.load_directory(new_path, entries);
            self.status_message = format!("Navigated history to: {}", self.current_path.display());
        }
        Ok(())
    }

    fn filter_files(&mut self) {
        if self.search_query.is_empty() {
            self.is_filtering = false;
            self.filtered_file_list.clear();
            return;
        }

        let query = self.search_query.to_lowercase();
        self.filtered_file_list = self
            .file_list
            .iter()
            .filter(|entry| entry.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        self.is_filtering = true;
        self.selected_index = 0;
    }

    pub fn get_display_list(&self) -> &[DirEntry] {
        if self.is_filtering {
            &self.filtered_file_list
        } else {
            &self.file_list
        }
    }

    async fn search_file_contents(&mut self, pattern: &str) -> Result<()> {
        self.status_message = format!("Searching for '{}'...", pattern);

        let (cmd, args) = if tokio::process::Command::new("rg")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_ok()
        {
            ("rg", vec!["-i", "-n", "--color", "never", pattern, "."])
        } else {
            ("grep", vec!["-r", "-i", "-n", pattern, "."])
        };

        let output = tokio::process::Command::new(cmd)
            .args(&args)
            .current_dir(&self.current_path)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.is_empty() {
                    self.preview_state =
                        PreviewState::Text(format!("No matches found for '{}'", pattern));
                    self.status_message = format!("Search complete: no matches for '{}'", pattern);
                } else {
                    let lines: Vec<&str> = stdout.lines().take(100).collect();
                    self.preview_state = PreviewState::Text(format!(
                        "Search results for '{}':\n\n{}{}",
                        pattern,
                        lines.join("\n"),
                        if stdout.lines().count() > 100 {
                            "\n\n... (showing first 100 matches)"
                        } else {
                            ""
                        }
                    ));
                    self.status_message =
                        format!("Found {} matches for '{}'", stdout.lines().count(), pattern);
                }
            }
            Err(e) => {
                self.preview_state = PreviewState::Text(format!("Search failed: {}", e));
                self.status_message = format!("Search error: {}", e);
            }
        }

        Ok(())
    }

    fn handle_file_opened(&mut self, path: PathBuf) -> Result<()> {
        history_fs::log_access(&self.db_connection, &path)?;

        opener::open(&path)?;

        self.status_message = format!(
            "Opened file: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        Ok(())
    }

    async fn execute_command(&mut self) -> Result<()> {
        let command_line = self.command_input.trim().to_string();
        if command_line.is_empty() {
            self.status_message = String::from("Command input is empty.");
            self.command_input.clear();
            self.mode = AppMode::Normal;
            return Ok(());
        }

        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if parts.is_empty() {
            self.status_message = String::from("Invalid command.");
            self.command_input.clear();
            self.mode = AppMode::Normal;
            return Ok(());
        }

        let command = parts[0];
        let args = &parts[1..];

        let output = tokio::process::Command::new(command)
            .args(args)
            .current_dir(&self.current_path)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let result_text = if output.status.success() {
                    format!("Command executed successfully:\n{}", stdout)
                } else {
                    format!(
                        "Command failed with status: {}\nStdout:\n{}\nStderr:\n{}",
                        output.status, stdout, stderr
                    )
                };

                self.preview_state = PreviewState::Text(result_text);
                self.status_message = format!("Command '{}' finished.", command);

                if let Err(e) = self.refresh_directory() {
                    self.status_message = format!("Command finished, but refresh failed: {}", e);
                }
            }
            Err(e) => {
                let error_message = format!("Failed to execute command '{}': {}", command, e);
                self.preview_state = PreviewState::Text(error_message.clone());
                self.status_message = error_message;
            }
        }

        self.command_input.clear();
        self.mode = AppMode::Normal;
        Ok(())
    }

    fn update_preview(&mut self) {
        let display_list = self.get_display_list();
        if display_list.is_empty() {
            self.preview_state = PreviewState::None;
            return;
        }

        let selected = &display_list[self.selected_index];
        if selected.is_dir {
            self.preview_state = PreviewState::Summary(format!(
                "Directory: {}\nItems: {}",
                selected.name,
                self.file_list.len()
            ));
        } else {
            // Simple text preview for files up to a certain size
            if selected.size < 1024 * 100 {
                // 100KB limit
                match std::fs::read_to_string(&selected.path) {
                    Ok(content) => {
                        let lines: Vec<&str> = content.lines().take(20).collect();
                        self.preview_state = PreviewState::Text(lines.join("\n"));
                    }
                    Err(_) => {
                        self.preview_state = PreviewState::Summary(format!(
                            "Binary file or failed to read: {}",
                            selected.name
                        ))
                    }
                }
            } else {
                self.preview_state = PreviewState::Summary(format!(
                    "File too large for preview: {} ({} bytes)",
                    selected.name, selected.size
                ));
            }
        }
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key).await,
            AppMode::Search => self.handle_search_mode(key).await,
            AppMode::Command => self.handle_command_mode(key).await,
            _ => Ok(()),
        }
    }

    async fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.cycle_focused_pane(),
            KeyCode::Char('1') => self.focused_pane = FocusedPane::History,
            KeyCode::Char('2') => self.focused_pane = FocusedPane::FileList,
            KeyCode::Char('3') => self.focused_pane = FocusedPane::Preview,
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => self.enter_selected()?,
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => self.go_up()?,
            KeyCode::Char('/') => {
                self.mode = AppMode::Search;
                self.search_query.clear();
                self.is_filtering = false;
            }
            KeyCode::Char(':') => self.mode = AppMode::Command,
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                self.refresh_directory()?;
                self.status_message = format!(
                    "Hidden files: {}",
                    if self.show_hidden { "shown" } else { "hidden" }
                );
            }
            KeyCode::Char('r') => {
                self.refresh_directory()?;
                self.status_message = String::from("Directory refreshed");
            }
            KeyCode::Char('H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.navigate_history(false)?
            }
            KeyCode::Char('L') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.navigate_history(true)?
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_search_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.search_query.clear();
                self.is_filtering = false;
                self.filtered_file_list.clear();
            }
            KeyCode::Enter => {
                // If search starts with '@', search file contents instead of filenames
                if self.search_query.starts_with('@') {
                    let pattern = self.search_query.trim_start_matches('@').trim().to_string();
                    if !pattern.is_empty() {
                        self.search_file_contents(&pattern).await?;
                    }
                    self.mode = AppMode::Normal;
                    self.search_query.clear();
                    self.is_filtering = false;
                } else {
                    // Keep filter active in normal mode
                    self.mode = AppMode::Normal;
                    if self.is_filtering {
                        self.status_message =
                            format!("Filtered {} items", self.filtered_file_list.len());
                    }
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                if !self.search_query.starts_with('@') {
                    self.filter_files();
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                if !self.search_query.starts_with('@') {
                    self.filter_files();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_tab_completion(&mut self) -> Result<()> {
        // If we don't have candidates yet, generate them
        if self.completion_candidates.is_empty() {
            let (prefix, word_to_complete) = self.extract_word_to_complete();

            if word_to_complete.is_empty() {
                return Ok(());
            }

            // Get all files/directories in current directory
            let entries = fs::read_directory(&self.current_path, self.show_hidden)?;

            // Find matching entries
            let matches: Vec<String> = entries
                .iter()
                .filter(|entry| {
                    entry.name.starts_with(&word_to_complete) && entry.name != ".."
                })
                .map(|entry| {
                    if entry.is_dir {
                        format!("{}/", entry.name)
                    } else {
                        entry.name.clone()
                    }
                })
                .collect();

            if matches.is_empty() {
                self.status_message = String::from("No matches found");
                return Ok(());
            }

            // If only one match, complete it immediately
            if matches.len() == 1 {
                self.command_input = format!("{}{}", prefix, matches[0]);
                self.status_message = String::from("Completed");
                return Ok(());
            }

            // Multiple matches - store them and start cycling
            self.completion_candidates = matches;
            self.completion_index = 0;
            self.command_input = format!("{}{}", prefix, self.completion_candidates[0]);
            self.status_message = format!(
                "Match 1/{} (Tab to cycle)",
                self.completion_candidates.len()
            );
        } else {
            // Cycle through existing candidates
            self.completion_index = (self.completion_index + 1) % self.completion_candidates.len();
            let (prefix, _) = self.extract_word_to_complete();
            self.command_input = format!("{}{}", prefix, self.completion_candidates[self.completion_index]);
            self.status_message = format!(
                "Match {}/{} (Tab to cycle)",
                self.completion_index + 1,
                self.completion_candidates.len()
            );
        }

        Ok(())
    }

    fn extract_word_to_complete(&self) -> (String, String) {
        let input = &self.command_input;

        // Find the last word (after the last space)
        if let Some(last_space_idx) = input.rfind(' ') {
            let prefix = &input[..=last_space_idx];
            let word = &input[last_space_idx + 1..];
            (prefix.to_string(), word.to_string())
        } else {
            // No space found, the entire input is the word to complete
            // But only complete if it looks like a filename (not a command)
            // Allow completion for everything in command mode
            (String::new(), input.to_string())
        }
    }

    async fn handle_command_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.command_input.clear();
                self.completion_candidates.clear();
                self.completion_index = 0;
            }
            KeyCode::Enter => {
                self.completion_candidates.clear();
                self.completion_index = 0;
                self.execute_command().await?;
            }
            KeyCode::Tab => {
                self.handle_tab_completion()?;
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
                self.completion_candidates.clear();
                self.completion_index = 0;
            }
            KeyCode::Backspace => {
                self.command_input.pop();
                self.completion_candidates.clear();
                self.completion_index = 0;
            }
            _ => {}
        }
        Ok(())
    }

    fn cycle_focused_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::History => FocusedPane::FileList,
            FocusedPane::FileList => FocusedPane::Preview,
            FocusedPane::Preview => FocusedPane::History,
        };
        self.status_message = format!("Focused: {:?}", self.focused_pane);
    }

    fn move_selection(&mut self, delta: i32) {
        match self.focused_pane {
            FocusedPane::FileList => {
                let display_list = self.get_display_list();
                let len = display_list.len() as i32;
                if len == 0 {
                    return;
                }
                let new_index = (self.selected_index as i32 + delta).rem_euclid(len) as usize;
                self.selected_index = new_index;
                self.update_preview();
            }
            FocusedPane::History => {
                let len = self.recent_files.len() as i32;
                if len == 0 {
                    return;
                }
                let new_index =
                    (self.history_selected_index as i32 + delta).rem_euclid(len) as usize;
                self.history_selected_index = new_index;
            }
            FocusedPane::Preview => {
                // Preview pane doesn't have navigation
                self.status_message = String::from("Preview pane has no navigation");
            }
        }
    }

    fn enter_selected(&mut self) -> Result<()> {
        match self.focused_pane {
            FocusedPane::FileList => {
                let display_list = self.get_display_list();
                if display_list.is_empty() {
                    return Ok(());
                }
                let selected = display_list[self.selected_index].clone();

                self.is_filtering = false;
                self.search_query.clear();
                self.filtered_file_list.clear();

                if selected.is_dir {
                    self.change_directory(selected.path)?;
                } else {
                    self.handle_file_opened(selected.path)?;
                }
            }
            FocusedPane::History => {
                if self.recent_files.is_empty() {
                    return Ok(());
                }
                let selected = self.recent_files[self.history_selected_index].clone();
                if selected.path.is_dir() {
                    self.change_directory(selected.path)?;
                } else {
                    self.handle_file_opened(selected.path)?;
                }
            }
            FocusedPane::Preview => {
                self.status_message = String::from("Cannot enter from preview pane");
            }
        }
        Ok(())
    }

    fn go_up(&mut self) -> Result<()> {
        match self.focused_pane {
            FocusedPane::FileList => {
                if let Some(parent) = self.current_path.parent() {
                    self.change_directory(parent.to_path_buf())?;
                }
            }
            FocusedPane::History | FocusedPane::Preview => {
                self.status_message = String::from("Can only navigate up from file list pane");
            }
        }
        Ok(())
    }
}
