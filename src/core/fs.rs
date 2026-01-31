use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl DirEntry {
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let metadata = fs::metadata(&path)?;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        Ok(DirEntry {
            path: path.clone(),
            name,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
}

// Function to read a directory and return a vector of DirEntry
pub fn read_directory(path: &Path, show_hidden: bool) -> Result<Vec<DirEntry>> {
    let mut entries = Vec::new();

    // Add parent directory entry (..)
    if path.parent().is_some() {
        entries.push(DirEntry {
            path: path.parent().unwrap().to_path_buf(),
            name: String::from(".."),
            is_dir: true,
            size: 0,
            modified: None,
        });
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files/directories (starting with .) unless show_hidden is true
        if !show_hidden
            && path
                .file_name()
                .map_or(false, |s| s.to_string_lossy().starts_with('.'))
            && path
                .file_name()
                .map_or(false, |s| s.to_string_lossy() != "..")
        {
            continue;
        }

        if let Ok(dir_entry) = DirEntry::from_path(path) {
            entries.push(dir_entry);
        }
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}
