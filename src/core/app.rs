use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Duration;

use super::mode::AppMode;
use crate::core::apps::DesktopApp;
use crate::core::fs::{self, DirEntry};
use crate::core::history::{self as history_fs, RecentAccess};
use crate::core::search::{SearchResult, SearchResultKind};

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

    // Launcher State
    pub applications: Vec<DesktopApp>,
    pub search_results: Vec<SearchResult>,
    pub window_visible: bool,
}

impl App {
    pub fn new(db_conn: Connection) -> Result<Self> {
        use crate::core::apps;

        let initial_path = std::env::current_dir()?;
        let initial_list = fs::read_directory(&initial_path, false)?;
        let recent_files = history_fs::get_recent_files(&db_conn, 10).unwrap_or_default();
        let applications = apps::discover_applications();

        Ok(App {
            current_path: initial_path.clone(),
            file_list: initial_list,
            selected_index: 0,
            mode: AppMode::Normal,
            should_quit: false,
            tick_rate: Duration::from_millis(250),
            status_message: String::from("Welcome to Files Launcher!"),

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

            applications,
            search_results: Vec::new(),
            window_visible: true,
        })
    }

    pub fn _on_tick(&mut self) {
        self.recent_files =
            history_fs::get_recent_files(&self.db_connection, 10).unwrap_or_default();
    }

    pub fn _quit(&mut self) {
        self.should_quit = true;
    }

    pub fn toggle_visibility(&mut self) {
        self.window_visible = !self.window_visible;
        if self.window_visible {
            self.search_query.clear();
            self.search_results.clear();
        }
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

    pub fn enter_selected(&mut self) -> Result<()> {
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
                    self.open_file(selected.path)?;
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
                    self.open_file(selected.path)?;
                }
            }
            FocusedPane::Preview => {
                self.status_message = String::from("Cannot enter from preview pane");
            }
        }
        Ok(())
    }

    pub fn go_up(&mut self) -> Result<()> {
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

    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        history_fs::log_access(&self.db_connection, &path)?;
        opener::open(&path)?;
        self.status_message = format!(
            "Opened: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        Ok(())
    }

    pub fn update_search(&mut self, query: &str) {
        use crate::core::search;
        self.search_query = query.to_string();

        if query.is_empty() {
            self.search_results.clear();
            self.is_filtering = false;
            self.filtered_file_list.clear();
            return;
        }

        // Update search results (files + apps)
        self.search_results = search::search_all(
            query,
            &self.file_list,
            &self.recent_files,
            &self.applications,
        );

        self.filter_files();
    }

    pub fn execute_search_result(&mut self, index: usize) -> Result<()> {
        if index >= self.search_results.len() {
            return Ok(());
        }

        let result = &self.search_results[index];
        match &result.kind {
            SearchResultKind::File(path) | SearchResultKind::RecentFile(path) => {
                if path.is_dir() {
                    self.change_directory(path.clone())?;
                } else {
                    self.open_file(path.clone())?;
                }
            }
            SearchResultKind::Application(app) => {
                app.launch()?;
                self.status_message = format!("Launched: {}", app.name);
            }
            SearchResultKind::Command(cmd) => {
                self.command_input = cmd.clone();
                self.mode = AppMode::Command;
            }
            SearchResultKind::GrepResult {
                path,
                line: _,
                content: _,
            } => {
                self.open_file(path.clone())?;
            }
        }
        Ok(())
    }
}
