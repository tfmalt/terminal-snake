use rand::Rng;

use crate::config::GridSize;
use crate::snake::{Position, Snake};

/// Distinguishes normal food from time-limited super food.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FoodKind {
    Normal,
    Super { ticks_remaining: u32 },
}

/// Food entity currently active on the board.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Food {
    pub position: Position,
    pub kind: FoodKind,
}

impl Food {
    /// Creates normal food at `position`.
    #[must_use]
    pub fn new(position: Position) -> Self {
        Self {
            position,
            kind: FoodKind::Normal,
        }
    }

    /// Creates super food at `position` with a tick countdown.
    #[must_use]
    pub fn new_super(position: Position, ticks: u32) -> Self {
        Self {
            position,
            kind: FoodKind::Super {
                ticks_remaining: ticks,
            },
        }
    }

    /// Returns the score value granted when eaten.
    #[must_use]
    pub fn points(self) -> u32 {
        match self.kind {
            FoodKind::Normal => 1,
            FoodKind::Super { .. } => 5,
        }
    }

    /// Returns the number of segments the snake gains when eating this food.
    #[must_use]
    pub fn growth(self) -> u32 {
        match self.kind {
            FoodKind::Normal => 1,
            FoodKind::Super { .. } => 5,
        }
    }

    /// Returns true if this is super food.
    #[must_use]
    pub fn is_super(self) -> bool {
        matches!(self.kind, FoodKind::Super { .. })
    }

    /// Decrements the super food timer. Returns true if the food is still alive.
    /// Normal food always returns true.
    pub fn tick(&mut self) -> bool {
        match &mut self.kind {
            FoodKind::Normal => true,
            FoodKind::Super { ticks_remaining } => {
                *ticks_remaining = ticks_remaining.saturating_sub(1);
                *ticks_remaining > 0
            }
        }
    }

    /// Degrades super food to normal food. No-op on normal food.
    pub fn degrade(&mut self) {
        self.kind = FoodKind::Normal;
    }

    /// Spawns food in an unoccupied cell.
    pub fn spawn<R: Rng + ?Sized>(rng: &mut R, bounds: GridSize, snake: &Snake) -> Option<Self> {
        spawn_position(rng, bounds, snake).map(Self::new)
    }
}

/// Returns a random position not currently occupied by the snake.
#[must_use]
pub fn spawn_position<R: Rng + ?Sized>(
    rng: &mut R,
    bounds: GridSize,
    snake: &Snake,
) -> Option<Position> {
    let mut candidates = Vec::new();

    for y in 0..i32::from(bounds.height) {
        for x in 0..i32::from(bounds.width) {
            let position = Position { x, y };
            if !snake.occupies(position) {
                candidates.push(position);
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    let index = rng.gen_range(0..candidates.len());
    Some(candidates[index])
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::config::GridSize;
    use crate::input::Direction;

    use super::{Food, spawn_position};
    use crate::snake::{Position, Snake};

    #[test]
    fn food_spawn_never_overlaps_snake() {
        let mut rng = StdRng::seed_from_u64(7);
        let snake = Snake::from_segments(
            vec![
                Position { x: 0, y: 0 },
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
            ],
            Direction::Right,
        )
        .expect("test snake segments should be valid");

        for _ in 0..100 {
            let food_position = spawn_position(
                &mut rng,
                GridSize {
                    width: 8,
                    height: 6,
                },
                &snake,
            )
            .expect("there should always be free cells");
            assert!(!snake.occupies(food_position));
        }
    }

    #[test]
    fn spawn_position_returns_none_when_board_is_full() {
        let mut rng = StdRng::seed_from_u64(11);
        let snake = Snake::from_segments(vec![Position { x: 0, y: 0 }], Direction::Right)
            .expect("single segment is valid");

        let spawned = spawn_position(
            &mut rng,
            GridSize {
                width: 1,
                height: 1,
            },
            &snake,
        );

        assert_eq!(spawned, None);
    }

    #[test]
    fn normal_food_grants_one_point() {
        let food = Food::new(Position { x: 1, y: 1 });
        assert_eq!(food.points(), 1);
        assert_eq!(food.growth(), 1);
        assert!(!food.is_super());
    }

    #[test]
    fn super_food_grants_five_points() {
        let food = Food::new_super(Position { x: 1, y: 1 }, 10);
        assert_eq!(food.points(), 5);
        assert_eq!(food.growth(), 5);
        assert!(food.is_super());
    }

    #[test]
    fn super_food_tick_counts_down() {
        let mut food = Food::new_super(Position { x: 0, y: 0 }, 3);
        assert!(food.tick()); // 2 remaining
        assert!(food.tick()); // 1 remaining
        assert!(!food.tick()); // 0 remaining — expired
    }

    #[test]
    fn super_food_degrades_to_normal() {
        let mut food = Food::new_super(Position { x: 0, y: 0 }, 10);
        food.degrade();
        assert!(!food.is_super());
        assert_eq!(food.points(), 1);
    }
}
