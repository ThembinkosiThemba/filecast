use devicons::FileIcon;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::time::SystemTime;

use crate::core::{
    app::{App, FocusedPane, PreviewState},
    mode::AppMode,
};

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

fn format_size(size: u64) -> String {
    const K: u64 = 1024;
    const M: u64 = K * 1024;
    const G: u64 = M * 1024;

    if size >= G {
        format!("{:.1}G", size as f64 / G as f64)
    } else if size >= M {
        format!("{:.1}M", size as f64 / M as f64)
    } else if size >= K {
        format!("{:.1}K", size as f64 / K as f64)
    } else {
        format!("{}B", size)
    }
}

fn format_time(time: Option<SystemTime>) -> String {
    time.map(|t| {
        let datetime: chrono::DateTime<chrono::Local> = t.into();
        datetime.format("%Y-%m-%d").to_string()
    })
    .unwrap_or_else(|| "N/A".to_string())
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header (Path Bar)
            Constraint::Min(0),    // Main Content (Three Panes)
            Constraint::Length(1), // Status Bar
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);

    draw_main_content(f, app, chunks[1]);

    draw_status_bar(f, app, chunks[2]);

    if app.mode == AppMode::Search || app.mode == AppMode::Command {
        draw_modal(f, app);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let path_str = app.current_path.to_string_lossy().to_string();
    let header = Paragraph::new(path_str).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, area);
}

fn draw_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Left Pane (History)
            Constraint::Percentage(50), // Center Pane (File List)
            Constraint::Percentage(25), // Right Pane (Preview)
        ])
        .split(area);

    draw_history_pane(f, app, main_chunks[0]);
    draw_file_list_pane(f, app, main_chunks[1]);
    draw_preview_pane(f, app, main_chunks[2]);
}

fn draw_history_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::History;

    let items: Vec<ListItem> = app
        .recent_files
        .iter()
        .enumerate()
        .map(|(i, ra)| {
            let path_str = ra
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let style = if is_focused && i == app.history_selected_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            ListItem::new(path_str).style(style)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let title = if is_focused {
        " History & Recent [ACTIVE] "
    } else {
        " History & Recent "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_file_list_pane(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::FileList;
    let display_list = app.get_display_list();

    let items: Vec<ListItem> = display_list
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = is_focused && i == app.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let icon_data: FileIcon = entry.path.to_str().unwrap_or("").into();
            let icon_color = parse_hex_color(icon_data.color).unwrap_or(Color::White);
            let icon = Span::styled(icon_data.icon.to_string(), Style::default().fg(icon_color));

            let name = Span::styled(entry.name.clone(), style);
            let size = Span::styled(format_size(entry.size), style);
            let time = Span::styled(format_time(entry.modified), style);

            let filler = " ".repeat(
                area.width
                    .saturating_sub(2 + entry.name.len() as u16 + 10 + 12) as usize,
            );
            let line = Line::from(vec![
                icon,
                Span::raw(" "),
                name,
                Span::raw(filler),
                size,
                Span::raw("  "),
                time,
            ]);
            ListItem::new(line)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let title = if app.is_filtering {
        format!(" File List [FILTERED: {}] ", display_list.len())
    } else if is_focused {
        " File List [ACTIVE] ".to_string()
    } else {
        " File List ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.as_str())
                .border_style(border_style),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Preview;

    let content = match &app.preview_state {
        PreviewState::None => "No file selected or preview disabled.".to_string(),
        PreviewState::Text(text) => text.clone(),
        PreviewState::Summary(summary) => summary.clone(),
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let title = if is_focused {
        " Preview [ACTIVE] "
    } else {
        " Preview "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10), // Mode
            Constraint::Min(0),     // Status Message
            Constraint::Length(40), // Keybinding Hints
        ])
        .split(area);

    // Mode
    let mode_text = format!(" {} ", app.mode);
    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let mode_widget = Paragraph::new(mode_text).style(mode_style);
    f.render_widget(mode_widget, chunks[0]);

    // Status Message
    let status_widget =
        Paragraph::new(app.status_message.clone()).style(Style::default().fg(Color::White));
    f.render_widget(status_widget, chunks[1]);

    // Keybinding Hints
    let hints = match app.mode {
        AppMode::Normal => "Tab:Switch | /:Search | @:Grep | .:Hidden | r:Refresh | ::Cmd | q:Quit",
        AppMode::Search => "Esc:Cancel | Enter:Apply | @prefix:Content search",
        AppMode::Command => "Esc:Cancel | Enter:Execute",
        _ => "",
    };
    let hints_widget = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
    f.render_widget(hints_widget, chunks[2]);
}

fn draw_modal(f: &mut Frame, app: &App) {
    let area = f.area();
    let modal_height = 4;
    let modal_area = Rect::new(
        area.x,
        area.height.saturating_sub(modal_height),
        area.width,
        modal_height,
    );

    let (input_text, hint) = match app.mode {
        AppMode::Search => {
            let hint = if app.search_query.starts_with('@') {
                "Search file contents (ripgrep/grep)"
            } else {
                "Filter by filename (live)"
            };
            (format!("/{}", app.search_query), hint)
        }
        AppMode::Command => (format!(":{}", app.command_input), "Execute shell command"),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} Input ", app.mode));

    let content = format!("{}\n{}", input_text, hint);

    let input_widget = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(input_widget, modal_area);
    f.set_cursor_position((modal_area.x + input_text.len() as u16 + 1, modal_area.y + 1));
}
