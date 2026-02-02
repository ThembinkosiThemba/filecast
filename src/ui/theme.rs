use egui::{Color32, FontId, Rounding, Stroke, Style, Visuals};

pub const BG_PRIMARY: Color32 = Color32::from_rgb(30, 30, 30);
pub const BG_SECONDARY: Color32 = Color32::from_rgb(40, 40, 40);
pub const BG_HOVER: Color32 = Color32::from_rgb(50, 50, 50);
pub const BG_SELECTED: Color32 = Color32::from_rgb(60, 80, 60);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 220, 220);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(150, 150, 150);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 100);

pub const ACCENT: Color32 = Color32::from_rgb(100, 200, 100);

pub const BORDER: Color32 = Color32::from_rgb(60, 60, 60);

// Spacing
pub const PADDING: f32 = 12.0;
pub const SPACING: f32 = 8.0;
pub const ICON_SIZE: f32 = 20.0;
pub const ROUNDING: f32 = 8.0;

// Window
// pub const WINDOW_WIDTH: f32 = 600.0;
// pub const WINDOW_MIN_HEIGHT: f32 = 60.0;
// pub const WINDOW_MAX_HEIGHT: f32 = 500.0;

pub fn configure_style(ctx: &egui::Context) {
    let mut style = Style::default();

    // Dark visuals
    let mut visuals = Visuals::dark();

    visuals.window_fill = BG_PRIMARY;
    visuals.panel_fill = BG_PRIMARY;
    visuals.faint_bg_color = BG_SECONDARY;
    visuals.extreme_bg_color = BG_PRIMARY;

    visuals.widgets.noninteractive.bg_fill = BG_SECONDARY;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = BG_SECONDARY;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    visuals.widgets.active.bg_fill = BG_SELECTED;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);

    visuals.selection.bg_fill = BG_SELECTED;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.window_rounding = Rounding::same(ROUNDING);
    visuals.window_stroke = Stroke::new(1.0, BORDER);

    style.visuals = visuals;

    // Spacing
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING);
    style.spacing.window_margin = egui::Margin::same(PADDING);
    style.spacing.button_padding = egui::vec2(PADDING, PADDING / 2.0);

    ctx.set_style(style);
}

pub fn search_input_font() -> FontId {
    FontId::proportional(18.0)
}

pub fn result_name_font() -> FontId {
    FontId::proportional(14.0)
}

pub fn result_desc_font() -> FontId {
    FontId::proportional(11.0)
}
