use std::path::PathBuf;
use std::process::Command;

use crate::core::apps::DesktopApp;
use crate::core::fs::DirEntry;
use crate::core::history::RecentAccess;
use crate::core::search_config::SearchConfig;

#[derive(Debug, Clone)]
pub enum SearchResultKind {
    File(PathBuf),
    RecentFile(PathBuf),
    Application(DesktopApp),
    Command(String),
    GrepResult {
        path: PathBuf,
        line: u32,
        content: String,
    },
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub kind: SearchResultKind,
    pub icon: String,
    pub score: u32,
}

impl SearchResult {
    pub fn file(entry: &DirEntry, score: u32) -> Self {
        let icon = if entry.is_dir {
            "📁".to_string()
        } else {
            get_file_icon(&entry.name)
        };

        SearchResult {
            name: entry.name.clone(),
            description: entry.path.to_string_lossy().to_string(),
            kind: SearchResultKind::File(entry.path.clone()),
            icon,
            score,
        }
    }

    pub fn recent_file(recent: &RecentAccess, score: u32) -> Self {
        let name = recent
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| recent.path.to_string_lossy().to_string());

        let is_dir = recent.path.is_dir();
        let icon = if is_dir {
            "📁".to_string()
        } else {
            get_file_icon(&name)
        };

        SearchResult {
            name,
            description: format!("Recent • {}", recent.path.to_string_lossy()),
            kind: SearchResultKind::RecentFile(recent.path.clone()),
            icon,
            score,
        }
    }

    pub fn application(app: &DesktopApp, score: u32) -> Self {
        SearchResult {
            name: app.name.clone(),
            description: app
                .description
                .clone()
                .unwrap_or_else(|| "Application".to_string()),
            kind: SearchResultKind::Application(app.clone()),
            icon: "🚀".to_string(),
            score,
        }
    }

    pub fn command(cmd: &str) -> Self {
        SearchResult {
            name: format!("Run: {}", cmd),
            description: "Execute shell command".to_string(),
            kind: SearchResultKind::Command(cmd.to_string()),
            icon: "⚡".to_string(),
            score: 10,
        }
    }

    pub fn grep_result(path: PathBuf, line: u32, content: String) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        SearchResult {
            name: format!("{}:{}", name, line),
            description: content.trim().chars().take(80).collect(),
            kind: SearchResultKind::GrepResult {
                path,
                line,
                content,
            },
            icon: "🔎".to_string(),
            score: 30,
        }
    }
}

pub fn fuzzy_score(query: &str, text: &str) -> u32 {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    if text_lower == query_lower {
        return 100;
    }

    if text_lower.starts_with(&query_lower) {
        return 90;
    }

    if text_lower.contains(&query_lower) {
        return 70;
    }

    let query_chars: Vec<char> = query_lower.chars().collect();
    let text_chars: Vec<char> = text_lower.chars().collect();

    let mut query_idx = 0;
    let mut consecutive_bonus = 0;
    let mut last_match_idx: Option<usize> = None;

    for (i, c) in text_chars.iter().enumerate() {
        if query_idx < query_chars.len() && *c == query_chars[query_idx] {
            if let Some(last) = last_match_idx {
                if i == last + 1 {
                    consecutive_bonus += 5;
                }
            }
            last_match_idx = Some(i);
            query_idx += 1;
        }
    }

    if query_idx == query_chars.len() {
        let base_score = 40 + consecutive_bonus;
        let boundary_bonus = if text_lower
            .split_whitespace()
            .any(|word| word.starts_with(&query_lower.chars().next().unwrap_or(' ').to_string()))
        {
            10
        } else {
            0
        };
        return (base_score + boundary_bonus).min(65);
    }

    0
}

/// Search across all sources and return unified results
pub fn search_all(
    query: &str,
    files: &[DirEntry],
    recent: &[RecentAccess],
    apps: &[DesktopApp],
    config: &SearchConfig,
) -> Vec<SearchResult> {
    let mut results = Vec::new();

    if query.is_empty() {
        return results;
    }

    if query.starts_with(':') {
        let cmd = query.trim_start_matches(':').trim();
        if !cmd.is_empty() {
            results.push(SearchResult::command(cmd));
        }
        return results;
    }

    if query.starts_with('@') {
        let pattern = query.trim_start_matches('@').trim();
        if !pattern.is_empty() {
            return search_file_contents(pattern, config);
        }
        return results;
    }

    if query.starts_with('/') {
        let pattern = query.trim_start_matches('/').trim();
        if !pattern.is_empty() {
            return find_files(pattern, config);
        }
        return results;
    }

    for app in apps {
        let score = fuzzy_score(query, &app.name);
        if score > 0 {
            results.push(SearchResult::application(app, score));
        } else if let Some(ref desc) = app.description {
            let desc_score = fuzzy_score(query, desc);
            if desc_score > 30 {
                results.push(SearchResult::application(app, desc_score / 2));
            }
        }
    }

    for recent_file in recent {
        let name = recent_file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let score = fuzzy_score(query, &name);
        if score > 0 {
            results.push(SearchResult::recent_file(recent_file, score + 10)); // Bonus for recent
        }
    }

    for file in files {
        if file.name == ".." {
            continue;
        }

        let score = fuzzy_score(query, &file.name);
        if score > 0 {
            results.push(SearchResult::file(file, score));
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));

    results.truncate(20);

    results
}

/// Search file contents using grep/ripgrep
pub fn search_file_contents(pattern: &str, config: &SearchConfig) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Try ripgrep first with exclusions
    let output = {
        let mut cmd = Command::new("rg");
        cmd.args(["-n", "-i", "--max-count", "20"]);
        for arg in config.rg_exclude_args() {
            cmd.arg(&arg);
        }
        cmd.args([pattern, "."]);
        cmd.output()
    }
    .or_else(|_| {
        // Fall back to grep with exclusions
        let mut cmd = Command::new("grep");
        cmd.args(["-r", "-n", "-i"]);
        for arg in config.grep_exclude_args() {
            cmd.arg(&arg);
        }
        cmd.args([pattern, "."]);
        cmd.output()
    });

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().take(15) {
            // Parse grep output: filename:line:content
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let path = PathBuf::from(parts[0]);
                if let Ok(line_num) = parts[1].parse::<u32>() {
                    results.push(SearchResult::grep_result(
                        path,
                        line_num,
                        parts[2].to_string(),
                    ));
                }
            }
        }
    }

    results
}

pub fn find_files(pattern: &str, config: &SearchConfig) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Try fd first (faster) with exclusions
    let output = {
        let mut cmd = Command::new("fd");
        cmd.args(["-i", "--max-results", "20"]);
        for arg in config.fd_exclude_args() {
            cmd.arg(&arg);
        }
        cmd.arg(pattern);
        cmd.output()
    }
    .or_else(|_| {
        // Fall back to find with exclusions
        let mut cmd = Command::new("find");
        cmd.args([".", "-maxdepth", "5"]);
        for arg in config.find_exclude_args() {
            cmd.arg(&arg);
        }
        cmd.args(["-iname", &format!("*{}*", pattern)]);
        cmd.output()
    });

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().take(15) {
            let path = PathBuf::from(line.trim());
            if path.exists() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                let is_dir = path.is_dir();
                let icon = if is_dir {
                    "📁".to_string()
                } else {
                    get_file_icon(&name)
                };

                results.push(SearchResult {
                    name,
                    description: path.to_string_lossy().to_string(),
                    kind: SearchResultKind::File(path),
                    icon,
                    score: 50,
                });
            }
        }
    }

    results
}

fn get_file_icon(name: &str) -> String {
    let extension = name.rsplit('.').next().unwrap_or("").to_lowercase();

    match extension.as_str() {
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => "🖼️",
        // Videos
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpeg" | "mpg" => "🎬",
        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" => "🎵",
        // Documents
        "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" => "📝",
        "xls" | "xlsx" | "csv" | "ods" => "📊",
        "ppt" | "pptx" | "odp" => "📊",
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "7z" | "rar" | "xz" | "tgz" => "📦",
        // Code files
        "rs" | "py" | "js" | "ts" | "java" | "c" | "cpp" | "h" | "hpp" | "go" | "rb" | "php"
        | "tsx" | "jsx" => "💻",
        "html" | "css" | "json" | "xml" | "yaml" | "yml" | "toml" => "📋",
        // Executables
        "exe" | "bin" | "sh" | "bat" | "cmd" => "⚙️",
        // Default
        _ => "📄",
    }
    .to_string()
}
