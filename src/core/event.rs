use crate::core::fs::DirEntry;
use crossterm::event::KeyEvent;
use std::path::PathBuf;

/// Application events
#[derive(Debug)]
pub enum AppEvent {
    // User Input Events
    /// Keyboard input event
    Key(KeyEvent),
    /// Mouse event (for future use)
    Mouse,
    /// Terminal resize event
    Resize(u16, u16),

    // Application Control Events
    /// Periodic tick for updates
    Tick,
    /// Quit application
    Quit,

    // File System Events
    /// Directory loaded with entries
    DirectoryLoaded(PathBuf, Vec<DirEntry>),
    /// File opened successfully
    FileOpened(PathBuf),

    // History Events
    /// Navigate backward in history
    HistoryBack,
    /// Navigate forward in history
    HistoryForward,

    // Search Events
    /// Search input changed
    SearchInput(String),
    /// Execute search
    SearchExecute,

    // Command Events
    /// Command input changed
    CommandInput(String),
    /// Execute command
    CommandExecute,
}
