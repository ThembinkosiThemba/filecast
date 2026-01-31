use anyhow::Result;
use std::io;
use std::path::Path;

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

    let db_path = Path::new("files-tui-history.db");
    let db_conn = history::initialise(db_path)?;

    let mut terminal = setup_terminal()?;

    let mut app = App::new(db_conn)?;

    run(&mut terminal, &mut app).await?;

    restore_terminal(terminal)?;

    Ok(())
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
