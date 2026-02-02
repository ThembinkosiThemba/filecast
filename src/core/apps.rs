use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct DesktopApp {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub terminal: bool,
    pub path: PathBuf,
}

impl DesktopApp {
    pub fn launch(&self) -> Result<()> {
        let exec_clean = self
            .exec
            .replace("%f", "")
            .replace("%F", "")
            .replace("%u", "")
            .replace("%U", "")
            .replace("%d", "")
            .replace("%D", "")
            .replace("%n", "")
            .replace("%N", "")
            .replace("%i", "")
            .replace("%c", "")
            .replace("%k", "")
            .trim()
            .to_string();

        let parts: Vec<&str> = exec_clean.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty exec command");
        }

        let program = parts[0];
        let args = &parts[1..];

        if self.terminal {
            // Launch in terminal
            Command::new("x-terminal-emulator")
                .arg("-e")
                .arg(&exec_clean)
                .spawn()?;
        } else {
            Command::new(program).args(args).spawn()?;
        }

        Ok(())
    }
}

/// Discover all installed applications by parsing .desktop files
pub fn discover_applications() -> Vec<DesktopApp> {
    let mut apps = Vec::new();

    // Standard XDG application directories
    let search_dirs = get_application_dirs();

    for dir in search_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                    if let Some(app) = parse_desktop_file(&path) {
                        if !apps.iter().any(|a: &DesktopApp| a.name == app.name) {
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    // Sort alphabetically
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    apps
}

fn get_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = dirs::data_local_dir() {
        dirs.push(data_home.join("applications"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs.push(PathBuf::from("/usr/local/share/applications"));

    if let Some(data_home) = dirs::data_local_dir() {
        dirs.push(data_home.join("flatpak/exports/share/applications"));
    }
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));

    dirs
}

fn parse_desktop_file(path: &PathBuf) -> Option<DesktopApp> {
    use freedesktop_desktop_entry::DesktopEntry;

    let locales: &[&str] = &[];
    let content = std::fs::read_to_string(path).ok()?;
    let entry = DesktopEntry::from_str(path, &content, None::<&[&str]>).ok()?;

    if entry.no_display() {
        return None;
    }

    if entry
        .desktop_entry("Hidden")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        return None;
    }

    let name = entry.name(locales)?.to_string();
    let exec = entry.exec()?.to_string();

    let icon = entry.icon().map(|s| s.to_string());
    let description = entry.comment(locales).map(|s| s.to_string());
    let terminal = entry.terminal();

    let categories: Vec<String> = entry
        .categories()
        .map(|cats| cats.iter().map(|c| c.to_string()).collect())
        .unwrap_or_default();

    let keywords: Vec<String> = entry
        .keywords(locales)
        .map(|kws| kws.iter().map(|k| k.to_string()).collect())
        .unwrap_or_default();

    Some(DesktopApp {
        name,
        exec,
        icon,
        description,
        categories,
        keywords,
        terminal,
        path: path.clone(),
    })
}
