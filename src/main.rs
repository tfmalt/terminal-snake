use std::io;

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::{Block, Borders};
use ratatui::{Frame, Terminal};

fn main() -> io::Result<()> {
    run()?;
    cleanup_terminal()?;
    Ok(())
}

fn run() -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    terminal.draw(render_placeholder)?;
    Ok(())
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn cleanup_terminal() -> io::Result<()> {
    disable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, Show, LeaveAlternateScreen)?;

    Ok(())
}

fn render_placeholder(frame: &mut Frame<'_>) {
    let area = frame.area();

    let block = Block::default()
        .title(" snake phase 2 scaffold ")
        .borders(Borders::ALL);

    frame.render_widget(block, area);
}
