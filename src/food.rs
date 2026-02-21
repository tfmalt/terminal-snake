use rand::Rng;

use crate::config::GridSize;
use crate::snake::{Position, Snake};

/// Bonus food lifetime in ticks.
pub const BONUS_FOOD_LIFETIME_TICKS: u16 = 50;

/// Food type and associated metadata.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FoodKind {
    Normal,
    Bonus { ttl_ticks: u16 },
}

/// Food entity currently active on the board.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Food {
    pub position: Position,
    pub kind: FoodKind,
}

impl Food {
    /// Creates a normal food at `position`.
    #[must_use]
    pub fn normal(position: Position) -> Self {
        Self {
            position,
            kind: FoodKind::Normal,
        }
    }

    /// Creates a bonus food at `position`.
    #[must_use]
    pub fn bonus(position: Position) -> Self {
        Self {
            position,
            kind: FoodKind::Bonus {
                ttl_ticks: BONUS_FOOD_LIFETIME_TICKS,
            },
        }
    }

    /// Advances bonus food TTL by one tick. Returns `true` if the food has
    /// expired and should be replaced. Has no effect on normal food.
    pub fn tick_ttl(&mut self) -> bool {
        if let FoodKind::Bonus { ref mut ttl_ticks } = self.kind {
            *ttl_ticks = ttl_ticks.saturating_sub(1);
            return *ttl_ticks == 0;
        }
        false
    }

    /// Returns the score value granted when eaten.
    #[must_use]
    pub fn points(self) -> u32 {
        match self.kind {
            FoodKind::Normal => 1,
            FoodKind::Bonus { .. } => 5,
        }
    }

    /// Spawns regular food in an unoccupied cell.
    #[must_use]
    pub fn spawn<R: Rng + ?Sized>(rng: &mut R, bounds: GridSize, snake: &Snake) -> Self {
        Self::normal(spawn_position(rng, bounds, snake))
    }

    /// Spawns bonus food in an unoccupied cell.
    #[must_use]
    pub fn spawn_bonus<R: Rng + ?Sized>(rng: &mut R, bounds: GridSize, snake: &Snake) -> Self {
        Self::bonus(spawn_position(rng, bounds, snake))
    }
}

/// Spawns a free position that is not currently occupied by the snake.
#[must_use]
pub fn spawn_position<R: Rng + ?Sized>(rng: &mut R, bounds: GridSize, snake: &Snake) -> Position {
    let mut candidates = Vec::new();

    for y in 0..i32::from(bounds.height) {
        for x in 0..i32::from(bounds.width) {
            let position = Position { x, y };
            if !snake.occupies(position) {
                candidates.push(position);
            }
        }
    }

    assert!(
        !candidates.is_empty(),
        "spawn_position: no free cells on the board ({}×{})",
        bounds.width,
        bounds.height,
    );

    let index = rng.gen_range(0..candidates.len());
    candidates[index]
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use crate::config::GridSize;
    use crate::input::Direction;

    use super::{spawn_position, Food, FoodKind, BONUS_FOOD_LIFETIME_TICKS};
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
        );

        for _ in 0..100 {
            let food_position = spawn_position(
                &mut rng,
                GridSize {
                    width: 8,
                    height: 6,
                },
                &snake,
            );
            assert!(!snake.occupies(food_position));
        }
    }

    #[test]
    fn bonus_food_ttl_decrements_and_expires() {
        let mut food = Food::bonus(Position { x: 1, y: 1 });

        // Should not expire before TTL runs down.
        for _ in 0..BONUS_FOOD_LIFETIME_TICKS - 1 {
            assert!(!food.tick_ttl());
        }

        // Final tick should signal expiry.
        assert!(food.tick_ttl());
    }

    #[test]
    fn normal_food_ttl_never_expires() {
        let mut food = Food::normal(Position { x: 1, y: 1 });
        for _ in 0..200 {
            assert!(!food.tick_ttl());
        }
    }

    #[test]
    fn bonus_food_has_higher_points() {
        let normal = Food::normal(Position { x: 1, y: 1 });
        let bonus = Food::bonus(Position { x: 2, y: 2 });

        assert_eq!(normal.points(), 1);
        assert_eq!(bonus.points(), 5);

        assert_eq!(
            bonus.kind,
            FoodKind::Bonus {
                ttl_ticks: BONUS_FOOD_LIFETIME_TICKS
            }
        );
    }
}
