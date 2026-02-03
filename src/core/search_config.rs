use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default)]
    pub exclude_dirs: Vec<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            exclude_dirs: vec![
                "node_modules".to_string(),
                ".next".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "__pycache__".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                ".cache".to_string(),
            ],
        }
    }
}

impl SearchConfig {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_yaml::from_str(&content) {
                    return config;
                }
            }
        }
        // Create default config file if it doesn't exist
        let default = Self::default();
        default.save();
        default
    }

    pub fn save(&self) {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_yaml::to_string(self) {
            let header = "# Filecast Search Configuration\n# Add directories to exclude from @ (grep) and / (find) searches\n\n";
            let _ = fs::write(config_path, format!("{}{}", header, content));
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("filecast")
            .join("search.yaml")
    }

    /// Generate exclude flags for ripgrep
    pub fn rg_exclude_args(&self) -> Vec<String> {
        self.exclude_dirs
            .iter()
            .flat_map(|dir| vec!["--glob".to_string(), format!("!{}/**", dir)])
            .collect()
    }

    /// Generate exclude flags for fd
    pub fn fd_exclude_args(&self) -> Vec<String> {
        self.exclude_dirs
            .iter()
            .flat_map(|dir| vec!["--exclude".to_string(), dir.clone()])
            .collect()
    }

    /// Generate exclude flags for grep
    pub fn grep_exclude_args(&self) -> Vec<String> {
        self.exclude_dirs
            .iter()
            .map(|dir| format!("--exclude-dir={}", dir))
            .collect()
    }

    /// Generate exclude flags for find
    pub fn find_exclude_args(&self) -> Vec<String> {
        self.exclude_dirs
            .iter()
            .flat_map(|dir| {
                vec![
                    "-not".to_string(),
                    "-path".to_string(),
                    format!("*{}*", dir),
                ]
            })
            .collect()
    }
}
