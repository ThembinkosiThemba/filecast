use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowPosition {
    Center,
    TopCenter,
    TopLeft,
    TopRight,
    BottomCenter,
    BottomLeft,
    BottomRight,
    Custom(i32, i32),
}

impl Default for WindowPosition {
    fn default() -> Self {
        WindowPosition::TopCenter
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LauncherView {
    Search,
    Files,
    Clipboard,
    Settings,
}

impl Default for LauncherView {
    fn default() -> Self {
        LauncherView::Search
    }
}

#[derive(Debug, Clone)]
pub struct LauncherSettings {
    pub position: WindowPosition,
    pub width: f32,
    pub height: f32,
    pub current_view: LauncherView,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            position: WindowPosition::TopCenter,
            width: 600.0,
            height: 400.0,
            current_view: LauncherView::Search,
        }
    }
}

impl LauncherSettings {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                return Self::parse(&content);
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = self.serialize();
        let _ = fs::write(config_path, content);
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("filecast")
            .join("settings.conf")
    }

    fn parse(content: &str) -> Self {
        let mut settings = Self::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "position" => {
                        settings.position = match value {
                            "center" => WindowPosition::Center,
                            "top_center" => WindowPosition::TopCenter,
                            "top_left" => WindowPosition::TopLeft,
                            "top_right" => WindowPosition::TopRight,
                            "bottom_center" => WindowPosition::BottomCenter,
                            "bottom_left" => WindowPosition::BottomLeft,
                            "bottom_right" => WindowPosition::BottomRight,
                            s if s.starts_with("custom:") => {
                                if let Some(coords) = s.strip_prefix("custom:") {
                                    if let Some((x, y)) = coords.split_once(',') {
                                        if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                                            WindowPosition::Custom(x, y)
                                        } else {
                                            WindowPosition::TopCenter
                                        }
                                    } else {
                                        WindowPosition::TopCenter
                                    }
                                } else {
                                    WindowPosition::TopCenter
                                }
                            }
                            _ => WindowPosition::TopCenter,
                        };
                    }
                    "width" => {
                        if let Ok(w) = value.parse() {
                            settings.width = w;
                        }
                    }
                    "height" => {
                        if let Ok(h) = value.parse() {
                            settings.height = h;
                        }
                    }
                    _ => {}
                }
            }
        }

        settings
    }

    fn serialize(&self) -> String {
        let position_str = match self.position {
            WindowPosition::Center => "center".to_string(),
            WindowPosition::TopCenter => "top_center".to_string(),
            WindowPosition::TopLeft => "top_left".to_string(),
            WindowPosition::TopRight => "top_right".to_string(),
            WindowPosition::BottomCenter => "bottom_center".to_string(),
            WindowPosition::BottomLeft => "bottom_left".to_string(),
            WindowPosition::BottomRight => "bottom_right".to_string(),
            WindowPosition::Custom(x, y) => format!("custom:{},{}", x, y),
        };

        format!(
            "# Files Launcher Settings\nposition={}\nwidth={}\nheight={}\n",
            position_str, self.width, self.height
        )
    }

    pub fn get_window_position(&self) -> egui::Pos2 {
        let (screen_width, screen_height) = Self::detect_screen_size();

        let margin = 30.0;

        match self.position {
            WindowPosition::Center => egui::pos2(
                (screen_width - self.width) / 2.0,
                (screen_height - self.height) / 2.0,
            ),
            WindowPosition::TopCenter => {
                // Horizontally centered, near top
                egui::pos2((screen_width - self.width) / 2.0, margin)
            }
            WindowPosition::TopLeft => egui::pos2(margin, margin),
            WindowPosition::TopRight => egui::pos2(screen_width - self.width - margin, margin),
            WindowPosition::BottomCenter => egui::pos2(
                (screen_width - self.width) / 2.0,
                screen_height - self.height - margin,
            ),
            WindowPosition::BottomLeft => egui::pos2(margin, screen_height - self.height - margin),
            WindowPosition::BottomRight => egui::pos2(
                screen_width - self.width - margin,
                screen_height - self.height - margin,
            ),
            WindowPosition::Custom(x, y) => egui::pos2(x as f32, y as f32),
        }
    }

    fn detect_screen_size() -> (f32, f32) {
        // Try xrandr first (works on X11)
        if let Ok(output) = std::process::Command::new("xrandr")
            .arg("--current")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains(" connected") && line.contains(" primary") {
                    if let Some(res) = line.split_whitespace().find(|s| {
                        s.contains('x')
                            && s.chars()
                                .next()
                                .map(|c| c.is_ascii_digit())
                                .unwrap_or(false)
                    }) {
                        let res = res.split('+').next().unwrap_or(res);
                        if let Some((w, h)) = res.split_once('x') {
                            if let (Ok(width), Ok(height)) = (w.parse::<f32>(), h.parse::<f32>()) {
                                return (width, height);
                            }
                        }
                    }
                }
            }
            for line in stdout.lines() {
                if line.contains(" connected") {
                    if let Some(res) = line.split_whitespace().find(|s| {
                        s.contains('x')
                            && s.chars()
                                .next()
                                .map(|c| c.is_ascii_digit())
                                .unwrap_or(false)
                    }) {
                        let res = res.split('+').next().unwrap_or(res);
                        if let Some((w, h)) = res.split_once('x') {
                            if let (Ok(width), Ok(height)) = (w.parse::<f32>(), h.parse::<f32>()) {
                                return (width, height);
                            }
                        }
                    }
                }
            }
        }

        (1920.0, 1080.0)
    }
}
