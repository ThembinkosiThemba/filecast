use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::path::PathBuf;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event as CrosstermEvent};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

mod core;

use crate::core::app::App;
use crate::core::event::AppEvent;
use crate::core::history;
use crate::core::ui::draw;

#[tokio::main]
async fn main() -> Result<()> {
    // simplelog::WriteLogger::init(
    //     simplelog::LevelFilter::Info,
    //     simplelog::Config::default(),
    //     std::fs::File::create("files-tui.log")?,
    // )?;

    let db_path = get_db_path()?;
    let db_conn = history::initialise(&db_path)?;

    let mut terminal = setup_terminal()?;

    let mut app = App::new(db_conn)?;

    run(&mut terminal, &mut app).await?;

    restore_terminal(terminal)?;

    Ok(())
}

fn get_db_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("files-tui");

    fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;

    Ok(config_dir.join("history.db"))
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    terminal::disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = app.tick_rate;

    loop {
        terminal.draw(|f| draw(f, app))?;

        if crossterm::event::poll(tick_rate)? {
            if let CrosstermEvent::Key(key) = event::read()? {
                app.update(AppEvent::Key(key)).await?;
            }
        } else {
            app.update(AppEvent::Tick).await?;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
