use std::io;

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// Concrete terminal type used by the runtime.
pub type AppTerminal = Terminal<CrosstermBackend<io::Stdout>>;

/// Owns terminal lifecycle (raw mode + alternate screen) for one game session.
///
/// On drop, this type restores terminal state best-effort.
pub struct TerminalSession {
    terminal: AppTerminal,
}

impl TerminalSession {
    /// Enters raw mode, switches to alternate screen, and creates a ratatui terminal.
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }

        let backend = CrosstermBackend::new(stdout);
        match Terminal::new(backend) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                let _ = cleanup_terminal_best_effort();
                Err(error)
            }
        }
    }

    /// Returns mutable access to the inner ratatui terminal.
    pub fn terminal_mut(&mut self) -> &mut AppTerminal {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = cleanup_terminal_best_effort();
    }
}

fn cleanup_terminal_best_effort() -> io::Result<()> {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    execute!(stdout, Show, LeaveAlternateScreen)
}
