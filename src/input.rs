use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use gilrs::{Axis, Button, EventType, Gilrs};

const STICK_DEADZONE: f32 = 0.5;
const STICK_RELEASE_DEADZONE: f32 = 0.25;

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
}

/// Configuration flags for input source initialization.
#[derive(Debug, Clone, Copy)]
pub struct InputConfig {
    pub enable_controller: bool,
    pub is_wsl: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            enable_controller: true,
            is_wsl: false,
        }
    }
}

/// Non-blocking input poller for keyboard and controller sources.
pub struct InputHandler {
    gilrs: Option<Gilrs>,
    last_stick_direction: Option<Direction>,
    last_keyboard_direction: Option<Direction>,
    active_source: Option<InputSource>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InputSource {
    Keyboard,
    Controller,
}

impl InputHandler {
    /// Builds a new input handler.
    #[must_use]
    pub fn new(config: InputConfig) -> Self {
        Self {
            gilrs: initialize_gilrs(config),
            last_stick_direction: None,
            last_keyboard_direction: None,
            active_source: None,
        }
    }

    /// Polls for one input event without blocking the game loop.
    pub fn poll_input(&mut self) -> io::Result<Option<GameInput>> {
        let mut queued_direction: Option<GameInput> = None;
        let mut queued_other: Option<GameInput> = None;

        while event::poll(Duration::from_millis(0))? {
            let terminal_event = event::read()?;
            let Some(mapped) = map_terminal_event(terminal_event) else {
                continue;
            };

            if matches!(mapped, GameInput::Quit) {
                return Ok(Some(mapped));
            }

            if let GameInput::Direction(direction) = mapped {
                if self.last_keyboard_direction == Some(direction) {
                    continue;
                }
                self.last_keyboard_direction = Some(direction);
                queued_direction.get_or_insert(GameInput::Direction(direction));
                continue;
            }

            queued_other = Some(mapped);
        }

        if queued_direction.is_some() || queued_other.is_some() {
            self.active_source = Some(InputSource::Keyboard);
            return Ok(queued_direction.or(queued_other));
        }

        if let Some(gilrs) = &mut self.gilrs {
            if self.active_source == Some(InputSource::Keyboard) {
                return Ok(None);
            }

            while let Some(controller_event) = gilrs.next_event() {
                match controller_event.event {
                    EventType::ButtonPressed(button, _) => {
                        if let Some(mapped_input) = map_controller_button(button) {
                            self.active_source = Some(InputSource::Controller);
                            return Ok(Some(mapped_input));
                        }
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        if value.abs() < STICK_RELEASE_DEADZONE {
                            self.last_stick_direction = None;
                            continue;
                        }

                        let Some(GameInput::Direction(direction)) =
                            map_controller_axis(axis, value)
                        else {
                            continue;
                        };

                        if self.last_stick_direction == Some(direction) {
                            continue;
                        }

                        self.last_stick_direction = Some(direction);
                        self.active_source = Some(InputSource::Controller);
                        return Ok(Some(GameInput::Direction(direction)));
                    }
                    _ => {}
                }
            }
        }

        Ok(None)
    }
}

/// Returns whether a direction change is legal (no immediate 180° turns).
#[must_use]
pub fn direction_change_is_valid(current: Direction, next: Direction) -> bool {
    next != current.opposite()
}

fn map_terminal_event(event: Event) -> Option<GameInput> {
    let Event::Key(key_event) = event else {
        return None;
    };

    map_key_event(key_event)
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
        _ => None,
    }
}

fn initialize_gilrs(config: InputConfig) -> Option<Gilrs> {
    if !config.enable_controller || config.is_wsl {
        return None;
    }

    let gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(error) => {
            eprintln!("Controller input disabled: {error}");
            return None;
        }
    };

    for (_, gamepad) in gilrs.gamepads() {
        eprintln!("Detected gamepad: {}", gamepad.name());
    }

    Some(gilrs)
}

fn map_controller_button(button: Button) -> Option<GameInput> {
    match button {
        Button::DPadUp => Some(GameInput::Direction(Direction::Up)),
        Button::DPadDown => Some(GameInput::Direction(Direction::Down)),
        Button::DPadLeft => Some(GameInput::Direction(Direction::Left)),
        Button::DPadRight => Some(GameInput::Direction(Direction::Right)),
        Button::Start => Some(GameInput::Pause),
        Button::Select | Button::Mode => Some(GameInput::Quit),
        Button::South => Some(GameInput::Confirm),
        _ => None,
    }
}

fn map_controller_axis(axis: Axis, value: f32) -> Option<GameInput> {
    if value.abs() < STICK_DEADZONE {
        return None;
    }

    match axis {
        Axis::LeftStickX => {
            if value < 0.0 {
                Some(GameInput::Direction(Direction::Left))
            } else {
                Some(GameInput::Direction(Direction::Right))
            }
        }
        Axis::LeftStickY => {
            if value < 0.0 {
                Some(GameInput::Direction(Direction::Up))
            } else {
                Some(GameInput::Direction(Direction::Down))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use gilrs::{Axis, Button};

    use super::{
        direction_change_is_valid, map_controller_axis, map_controller_button, map_key_event,
        Direction, GameInput,
    };

    #[test]
    fn opposite_direction_is_correct() {
        assert_eq!(Direction::Up.opposite(), Direction::Down);
        assert_eq!(Direction::Down.opposite(), Direction::Up);
        assert_eq!(Direction::Left.opposite(), Direction::Right);
        assert_eq!(Direction::Right.opposite(), Direction::Left);
    }

    #[test]
    fn direction_buffer_rejects_reverse() {
        assert!(!direction_change_is_valid(Direction::Up, Direction::Down));
        assert!(!direction_change_is_valid(Direction::Down, Direction::Up));
        assert!(!direction_change_is_valid(
            Direction::Left,
            Direction::Right
        ));
        assert!(!direction_change_is_valid(
            Direction::Right,
            Direction::Left
        ));

        assert!(direction_change_is_valid(Direction::Up, Direction::Left));
        assert!(direction_change_is_valid(Direction::Up, Direction::Right));
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
    fn controller_button_mapping_supports_dpad_and_actions() {
        assert_eq!(
            map_controller_button(Button::DPadUp),
            Some(GameInput::Direction(Direction::Up))
        );
        assert_eq!(
            map_controller_button(Button::DPadRight),
            Some(GameInput::Direction(Direction::Right))
        );
        assert_eq!(map_controller_button(Button::Start), Some(GameInput::Pause));
        assert_eq!(map_controller_button(Button::Select), Some(GameInput::Quit));
        assert_eq!(
            map_controller_button(Button::South),
            Some(GameInput::Confirm)
        );
    }

    #[test]
    fn controller_axis_mapping_respects_deadzone() {
        assert_eq!(map_controller_axis(Axis::LeftStickX, 0.2), None);
        assert_eq!(
            map_controller_axis(Axis::LeftStickX, -0.8),
            Some(GameInput::Direction(Direction::Left))
        );
        assert_eq!(
            map_controller_axis(Axis::LeftStickY, 0.9),
            Some(GameInput::Direction(Direction::Down))
        );
    }
}
