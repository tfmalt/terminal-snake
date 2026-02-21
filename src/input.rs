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

/// Returns whether a direction change is legal (no immediate 180° turns).
#[must_use]
pub fn direction_change_is_valid(current: Direction, next: Direction) -> bool {
    next != current.opposite()
}

#[cfg(test)]
mod tests {
    use super::{direction_change_is_valid, Direction};

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
}
