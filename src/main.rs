use anyhow::{Context, Result};
use eframe::egui;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;

mod core;
mod ui;

use crate::core::app::App;
use crate::core::history;
use crate::core::settings::{LauncherSettings, WindowPosition};
use crate::ui::launcher::LauncherUI;

fn main() -> Result<()> {
    let db_path = get_db_path()?;
    let db_conn = history::initialise(&db_path)?;

    let settings = LauncherSettings::load();

    let app = App::new(db_conn)?;

    let hotkey_manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");

    let hotkey = HotKey::new(Some(Modifiers::SUPER), Code::Space);
    hotkey_manager
        .register(hotkey)
        .expect("Failed to register hotkey");

    let (hotkey_tx, hotkey_rx) = mpsc::channel();

    std::thread::spawn(move || {
        loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                if event.state == HotKeyState::Pressed {
                    let _ = hotkey_tx.send(event);
                }
            }
        }
    });

    let icon = load_icon();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([650.0, 450.0])
        .with_min_inner_size([650.0, 100.0])
        .with_decorations(false)
        .with_always_on_top()
        .with_resizable(false)
        .with_title("Filecast");

    if let Some(icon_data) = icon {
        viewport = viewport.with_icon(std::sync::Arc::new(icon_data));
    }

    let use_centered = matches!(settings.position, WindowPosition::Center);
    if !use_centered {
        viewport = viewport.with_position(settings.get_window_position());
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: use_centered,
        ..Default::default()
    };

    let result = eframe::run_native(
        "Filecast",
        options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);

            Ok(Box::new(LauncherApp {
                app,
                ui: LauncherUI::new(),
                hotkey_rx,
                settings,
                was_visible: true,
            }))
        }),
    );

    std::process::exit(if result.is_ok() { 0 } else { 1 });
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("assets/icon.png");

    let image = image::load_from_memory(icon_bytes).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn get_db_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("filecast");

    fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

    Ok(config_dir.join("history.db"))
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "emoji".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"
        ))),
    );

    for family in fonts.families.values_mut() {
        family.push("emoji".to_owned());
    }

    ctx.set_fonts(fonts);
}

struct LauncherApp {
    app: App,
    ui: LauncherUI,
    hotkey_rx: mpsc::Receiver<GlobalHotKeyEvent>,
    settings: LauncherSettings,
    was_visible: bool,
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.check_clipboard_updates();

        while let Ok(_event) = self.hotkey_rx.try_recv() {
            self.app.window_visible = !self.app.window_visible;
            if self.app.window_visible {
                self.app.search_query.clear();
                self.app.search_results.clear();
                self.app.refresh_history();
                self.app.refresh_clipboard();
                let _ = self.app.refresh_directory();
                self.ui.search_focused = true;
            }
        }

        if self.app.window_visible != self.was_visible {
            if self.app.window_visible {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            self.was_visible = self.app.window_visible;
        }

        if self.app.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        self.ui.show(ctx, &mut self.app, &mut self.settings);

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.1, 0.1, 0.12, 1.0]
    }
}
