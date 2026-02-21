use std::io;
use std::panic;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::{Block, Borders};
use ratatui::{Frame, Terminal};

fn main() -> io::Result<()> {
    install_panic_hook();

    run()?;
    cleanup_terminal()?;
    Ok(())
}

fn run() -> io::Result<()> {
    let mut terminal = setup_terminal()?;

    loop {
        terminal.draw(render_placeholder)?;

        if event::poll(Duration::from_millis(50))? {
            if should_quit(event::read()?) {
                break;
            }
        }
    }

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

fn install_panic_hook() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal_after_panic();
        default_hook(panic_info);
    }));
}

fn restore_terminal_after_panic() {
    let _ = disable_raw_mode();

    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show, LeaveAlternateScreen);
}

fn should_quit(event: Event) -> bool {
    let Event::Key(key_event) = event else {
        return false;
    };

    match key_event.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => true,
        KeyCode::Char('c') => key_event.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

fn render_placeholder(frame: &mut Frame<'_>) {
    let area = frame.area();

    let block = Block::default()
        .title(" snake phase 2 scaffold ")
        .borders(Borders::ALL);

    frame.render_widget(block, area);
}
