use std::collections::VecDeque;

use crate::config::GridSize;
use crate::input::Direction;

/// Grid position in logical cell coordinates.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Returns true when the position lies inside the bounds.
    #[must_use]
    pub fn is_within_bounds(self, bounds: GridSize) -> bool {
        self.x >= 0
            && self.y >= 0
            && self.x < i32::from(bounds.width)
            && self.y < i32::from(bounds.height)
    }

    /// Returns this position wrapped into bounds on both axes.
    #[must_use]
    pub fn wrapped(self, bounds: GridSize) -> Self {
        Self {
            x: wrap_axis(self.x, i32::from(bounds.width)),
            y: wrap_axis(self.y, i32::from(bounds.height)),
        }
    }
}

fn wrap_axis(value: i32, upper_bound: i32) -> i32 {
    let wrapped = value % upper_bound;
    if wrapped < 0 {
        wrapped + upper_bound
    } else {
        wrapped
    }
}

/// Mutable snake state and movement buffering behavior.
#[derive(Debug, Clone)]
pub struct Snake {
    body: VecDeque<Position>,
    direction: Direction,
    buffered_direction: Direction,
    grow: bool,
}

impl Snake {
    /// Creates a one-cell snake at `start` with the provided direction.
    #[must_use]
    pub fn new(start: Position, direction: Direction) -> Self {
        let mut body = VecDeque::new();
        body.push_front(start);

        Self {
            body,
            direction,
            buffered_direction: direction,
            grow: false,
        }
    }

    /// Creates a snake from explicit body segments (front is head).
    #[must_use]
    pub fn from_segments(segments: Vec<Position>, direction: Direction) -> Self {
        Self {
            body: VecDeque::from(segments),
            direction,
            buffered_direction: direction,
            grow: false,
        }
    }

    /// Queues growth on the next movement tick.
    pub fn grow_next(&mut self) {
        self.grow = true;
    }

    /// Applies one buffered movement step.
    pub fn move_forward(&mut self, bounds: GridSize) {
        debug_assert!(bounds.width > 0 && bounds.height > 0);

        self.direction = self.buffered_direction;

        let next_head = self.next_head_position();

        self.body.push_front(next_head);
        if !self.grow {
            let _ = self.body.pop_back();
        }
        self.grow = false;
    }

    /// Returns the head position for the next movement tick.
    #[must_use]
    pub fn next_head_position(&self) -> Position {
        let head = self.head();
        match self.buffered_direction {
            Direction::Up => Position {
                x: head.x,
                y: head.y - 1,
            },
            Direction::Down => Position {
                x: head.x,
                y: head.y + 1,
            },
            Direction::Left => Position {
                x: head.x - 1,
                y: head.y,
            },
            Direction::Right => Position {
                x: head.x + 1,
                y: head.y,
            },
        }
    }

    /// Buffers the next direction if it is not a direct reversal.
    pub fn buffer_direction(&mut self, direction: Direction) {
        // Allow at most one queued turn per tick. This prevents rapid key-repeat
        // events from overwriting an already queued direction before movement.
        if self.buffered_direction != self.direction {
            return;
        }

        if direction == self.direction.opposite() {
            return;
        }

        self.buffered_direction = direction;
    }

    /// Returns the current head position.
    #[must_use]
    pub fn head(&self) -> Position {
        *self
            .body
            .front()
            .expect("snake body must always contain at least one segment")
    }

    /// Returns true if any segment occupies `position`.
    #[must_use]
    pub fn occupies(&self, position: Position) -> bool {
        self.body.contains(&position)
    }

    /// Returns true if the head overlaps any non-head segment.
    #[must_use]
    pub fn head_overlaps_body(&self) -> bool {
        let head = self.head();
        self.body.iter().skip(1).any(|segment| *segment == head)
    }

    /// Returns current segment count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.body.len()
    }

    /// Returns true when there are no segments.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }

    /// Returns the current movement direction.
    #[must_use]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Iterates over body segments from head to tail.
    pub fn segments(&self) -> impl Iterator<Item = &Position> {
        self.body.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::GridSize;
    use crate::input::Direction;

    use super::{Position, Snake};

    #[test]
    fn position_wrapping_keeps_coordinates_inside_bounds() {
        let bounds = GridSize {
            width: 10,
            height: 8,
        };

        let wrapped_left = Position { x: -1, y: 3 }.wrapped(bounds);
        let wrapped_bottom = Position { x: 4, y: 8 }.wrapped(bounds);

        assert_eq!(wrapped_left, Position { x: 9, y: 3 });
        assert_eq!(wrapped_bottom, Position { x: 4, y: 0 });
    }

    #[test]
    fn snake_moves_one_cell_per_tick() {
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);

        snake.move_forward(GridSize {
            width: 40,
            height: 20,
        });

        assert_eq!(snake.head(), Position { x: 6, y: 5 });
        assert_eq!(snake.len(), 1);
    }

    #[test]
    fn snake_growth_keeps_previous_tail() {
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);

        snake.grow_next();
        snake.move_forward(GridSize {
            width: 40,
            height: 20,
        });

        assert_eq!(snake.len(), 2);
    }

    #[test]
    fn direction_buffer_rejects_reverse() {
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Up);

        snake.buffer_direction(Direction::Down);
        snake.move_forward(GridSize {
            width: 40,
            height: 20,
        });

        assert_eq!(snake.head(), Position { x: 5, y: 4 });
    }

    #[test]
    fn direction_buffer_consumes_at_most_one_turn_per_tick() {
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Down);

        snake.buffer_direction(Direction::Right);
        snake.buffer_direction(Direction::Down);
        snake.buffer_direction(Direction::Left);

        snake.move_forward(GridSize {
            width: 40,
            height: 20,
        });

        assert_eq!(snake.head(), Position { x: 6, y: 5 });
    }
}
