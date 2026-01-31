use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search,
    Command,
    Quit,
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppMode::Normal => write!(f, "NORMAL"),
            AppMode::Search => write!(f, "SEARCH"),
            AppMode::Command => write!(f, "COMMAND"),
            AppMode::Quit => write!(f, "QUIT"),
        }
    }
}
