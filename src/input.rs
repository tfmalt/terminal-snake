use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Canonical movement directions for snake input.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Returns the opposite direction.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// High-level input events consumed by the game loop.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GameInput {
    Direction(Direction),
    Pause,
    Quit,
    Confirm,
    CycleTheme,
    Resize,
}

/// Non-blocking keyboard input poller.
pub struct InputHandler;

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    /// Builds a new input handler.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Polls for one input event without blocking the game loop.
    ///
    /// Keyboard events are drained in a single batch so the latest direction
    /// intent wins while quit/confirm actions still get through immediately.
    pub fn poll_input(&mut self) -> io::Result<Option<GameInput>> {
        let mut queued_direction: Option<GameInput> = None;
        let mut queued_action: Option<GameInput> = None;

        while event::poll(Duration::from_millis(0))? {
            let terminal_event = event::read()?;
            let Some(mapped) = map_terminal_event(terminal_event) else {
                continue;
            };

            if matches!(mapped, GameInput::Quit) {
                return Ok(Some(mapped));
            }

            if let GameInput::Direction(direction) = mapped {
                // Keep the last direction in the batch (most recent intent).
                // OS key-repeat of the same direction is harmless — the snake's
                // buffer_direction handles dedup at the game-logic level.
                queued_direction = Some(GameInput::Direction(direction));
                continue;
            }

            queued_action = Some(mapped);
        }

        if queued_direction.is_some() || queued_action.is_some() {
            return Ok(select_buffered_input(queued_direction, queued_action));
        }

        Ok(None)
    }
}

fn select_buffered_input(
    queued_direction: Option<GameInput>,
    queued_action: Option<GameInput>,
) -> Option<GameInput> {
    // Priority policy for one drained batch:
    // 1) Quit (handled eagerly in the loop above)
    // 2) Non-direction actions (Pause/Confirm/CycleTheme/Resize)
    // 3) Latest direction intent
    queued_action.or(queued_direction)
}

fn map_terminal_event(event: Event) -> Option<GameInput> {
    match event {
        Event::Key(key_event) => map_key_event(key_event),
        Event::Resize(_, _) => Some(GameInput::Resize),
        _ => None,
    }
}

fn map_key_event(key_event: KeyEvent) -> Option<GameInput> {
    if !matches!(key_event.kind, KeyEventKind::Press) {
        return None;
    }

    let key_code = key_event.code;

    if matches!(key_code, KeyCode::Char('c')) && key_event.modifiers.contains(KeyModifiers::CONTROL)
    {
        return Some(GameInput::Quit);
    }

    match key_code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
            Some(GameInput::Direction(Direction::Up))
        }
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
            Some(GameInput::Direction(Direction::Down))
        }
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
            Some(GameInput::Direction(Direction::Left))
        }
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
            Some(GameInput::Direction(Direction::Right))
        }
        KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Esc => Some(GameInput::Pause),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(GameInput::Quit),
        KeyCode::Enter | KeyCode::Char(' ') => Some(GameInput::Confirm),
        KeyCode::Char('t') | KeyCode::Char('T') => Some(GameInput::CycleTheme),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::{Direction, GameInput, map_key_event, map_terminal_event, select_buffered_input};

    #[test]
    fn opposite_direction_is_correct() {
        assert_eq!(Direction::Up.opposite(), Direction::Down);
        assert_eq!(Direction::Down.opposite(), Direction::Up);
        assert_eq!(Direction::Left.opposite(), Direction::Right);
        assert_eq!(Direction::Right.opposite(), Direction::Left);
    }

    #[test]
    fn keyboard_mapping_supports_wasd_and_arrows() {
        let up = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);

        assert_eq!(map_key_event(up), Some(GameInput::Direction(Direction::Up)));
        assert_eq!(
            map_key_event(right),
            Some(GameInput::Direction(Direction::Right))
        );
    }

    #[test]
    fn keyboard_mapping_supports_quit_pause_and_confirm() {
        let quit = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let pause = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let confirm = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert_eq!(map_key_event(quit), Some(GameInput::Quit));
        assert_eq!(map_key_event(pause), Some(GameInput::Pause));
        assert_eq!(map_key_event(confirm), Some(GameInput::Confirm));
        assert_eq!(map_key_event(ctrl_c), Some(GameInput::Quit));
    }

    #[test]
    fn keyboard_mapping_ignores_non_press_key_events() {
        let release = KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };

        assert_eq!(map_key_event(release), None);
    }

    #[test]
    fn terminal_resize_event_maps_to_resize_input() {
        assert_eq!(
            map_terminal_event(Event::Resize(120, 40)),
            Some(GameInput::Resize)
        );
    }

    #[test]
    fn buffered_input_prioritizes_action_over_direction() {
        let selected = select_buffered_input(
            Some(GameInput::Direction(Direction::Left)),
            Some(GameInput::Confirm),
        );
        assert_eq!(selected, Some(GameInput::Confirm));
    }

    #[test]
    fn buffered_input_uses_latest_direction_when_no_action_exists() {
        let selected = select_buffered_input(Some(GameInput::Direction(Direction::Right)), None);
        assert_eq!(selected, Some(GameInput::Direction(Direction::Right)));
    }
}
