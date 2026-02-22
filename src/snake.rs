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
    next_buffered_direction: Option<Direction>,
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
            next_buffered_direction: None,
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
            next_buffered_direction: None,
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

        if let Some(next) = self.next_buffered_direction.take() {
            self.buffered_direction = next;
        }

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

    /// Buffers the next direction, supporting a two-deep queue for quick turns.
    ///
    /// When no turn is queued yet, the direction is stored as the primary
    /// buffered direction (rejecting direct reversals of the current direction).
    /// When a turn is already queued, a second direction is stored with
    /// last-input-wins semantics (rejecting reversals of the *queued* direction).
    pub fn buffer_direction(&mut self, direction: Direction) {
        if self.buffered_direction == self.direction {
            // No turn queued yet — reject reversal of the current direction.
            if direction == self.direction.opposite() {
                return;
            }
            self.buffered_direction = direction;
        } else {
            // A turn is already queued — queue a second one.
            // Reject reversal of the *queued* direction, not the current one.
            if direction == self.buffered_direction.opposite() {
                return;
            }
            self.next_buffered_direction = Some(direction);
        }
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
    fn direction_buffer_two_deep_queue() {
        let bounds = GridSize {
            width: 40,
            height: 20,
        };
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Down);

        // Buffer Right, then Up within one tick.
        // Right goes into buffered_direction, Up into next_buffered.
        snake.buffer_direction(Direction::Right);
        snake.buffer_direction(Direction::Up);

        // First tick consumes Right, promotes Up into buffered.
        snake.move_forward(bounds);
        assert_eq!(snake.head(), Position { x: 6, y: 5 });

        // Second tick consumes Up (the promoted second direction).
        snake.move_forward(bounds);
        assert_eq!(snake.head(), Position { x: 6, y: 4 });
    }

    #[test]
    fn direction_buffer_allows_180_turn_via_two_step_queue() {
        let bounds = GridSize {
            width: 40,
            height: 20,
        };
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Left);

        // Buffer Right (perpendicular), then Down.
        snake.buffer_direction(Direction::Up);
        snake.buffer_direction(Direction::Right);

        // First tick: moves Up.
        snake.move_forward(bounds);
        assert_eq!(snake.head(), Position { x: 5, y: 4 });

        // Second tick: moves Right (promoted from next_buffered).
        snake.move_forward(bounds);
        assert_eq!(snake.head(), Position { x: 6, y: 4 });
    }

    #[test]
    fn direction_buffer_second_slot_uses_last_input() {
        let bounds = GridSize {
            width: 40,
            height: 20,
        };
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Down);

        // Buffer Right into primary, then Up, then Down into second slot.
        // Down should win (last-input-wins).
        snake.buffer_direction(Direction::Right);
        snake.buffer_direction(Direction::Up);
        snake.buffer_direction(Direction::Down);

        snake.move_forward(bounds);
        assert_eq!(snake.head(), Position { x: 6, y: 5 });

        // Down was the last input for the second slot.
        snake.move_forward(bounds);
        assert_eq!(snake.head(), Position { x: 6, y: 6 });
    }

    #[test]
    fn direction_buffer_rejects_reversal_of_queued_direction() {
        let mut snake = Snake::new(Position { x: 5, y: 5 }, Direction::Down);

        // Buffer Right into primary.
        snake.buffer_direction(Direction::Right);
        // Left is the opposite of Right (the queued direction) — should be rejected.
        snake.buffer_direction(Direction::Left);

        assert!(snake.next_buffered_direction.is_none());
    }
}
