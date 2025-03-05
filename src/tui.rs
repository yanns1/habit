use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::DisableMouseCapture;
use ratatui::crossterm::event::EnableMouseCapture;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::disable_raw_mode;
use ratatui::crossterm::terminal::enable_raw_mode;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::crossterm::terminal::LeaveAlternateScreen;
use ratatui::Terminal;
use std::io;
use std::io::stdout;
use std::io::Stdout;

/// A type alias for the terminal type used in this application
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal
pub fn init() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to its original state
pub fn restore(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
