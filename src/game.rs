use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::config::{BONUS_FOOD_SPAWN_INTERVAL_TICKS, POINTS_PER_SPEED_LEVEL};
use crate::config::GridSize;
use crate::food::{Food, FoodKind};
use crate::input::GameInput;
use crate::snake::{Position, Snake};

/// Current high-level gameplay state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GameStatus {
    Playing,
    Paused,
    GameOver,
    Victory,
}

/// Why the most recent game-over state was reached.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DeathReason {
    WallCollision,
    SelfCollision,
}

/// Complete mutable game state for one session.
#[derive(Debug, Clone)]
pub struct GameState {
    pub snake: Snake,
    pub food: Food,
    pub score: u32,
    pub speed_level: u32,
    pub tick_count: u64,
    pub status: GameStatus,
    pub death_reason: Option<DeathReason>,
    bounds: GridSize,
    base_speed_level: u32,
    rng: StdRng,
}

impl GameState {
    /// Creates a runtime state with non-deterministic RNG seeding.
    #[must_use]
    pub fn new(bounds: GridSize) -> Self {
        let seed = rand::random::<u64>();
        Self::new_with_seed_and_speed(bounds, seed, 1)
    }

    /// Creates a runtime state with explicit starting speed level.
    #[must_use]
    pub fn new_with_options(bounds: GridSize, starting_speed_level: u32) -> Self {
        let seed = rand::random::<u64>();
        Self::new_with_seed_and_speed(bounds, seed, starting_speed_level)
    }

    /// Creates a deterministic state for tests and reproducible simulations.
    #[must_use]
    pub fn new_with_seed(bounds: GridSize, seed: u64) -> Self {
        Self::new_with_seed_and_speed(bounds, seed, 1)
    }

    fn new_with_seed_and_speed(bounds: GridSize, seed: u64, starting_speed_level: u32) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let base_speed_level = starting_speed_level.max(1);
        let start = Position {
            x: i32::from(bounds.width / 2),
            y: i32::from(bounds.height / 2),
        };
        let snake = Snake::new(start, crate::input::Direction::Right);
        let food = Food::spawn(&mut rng, bounds, &snake);

        Self {
            snake,
            food,
            score: 0,
            speed_level: base_speed_level,
            tick_count: 0,
            status: GameStatus::Playing,
            death_reason: None,
            bounds,
            base_speed_level,
            rng,
        }
    }

    /// Advances simulation by one gameplay tick.
    pub fn tick(&mut self) {
        if self.status != GameStatus::Playing {
            return;
        }

        self.tick_count += 1;
        let next_head = self.snake.next_head_position();
        if !next_head.is_within_bounds(self.bounds) {
            self.status = GameStatus::GameOver;
            self.death_reason = Some(DeathReason::WallCollision);
            return;
        }

        let ate_food = next_head == self.food.position;
        if ate_food {
            self.snake.grow_next();
        }

        self.snake.move_forward(self.bounds);

        if self.snake.head_overlaps_body() {
            self.status = GameStatus::GameOver;
            self.death_reason = Some(DeathReason::SelfCollision);
            return;
        }

        if ate_food {
            self.score += self.food.points();
            self.update_speed_level();

            if self.snake.len() == self.bounds.total_cells() {
                self.status = GameStatus::Victory;
                self.death_reason = None;
                return;
            }

            self.food = Food::spawn(&mut self.rng, self.bounds, &self.snake);
        }

        // Advance bonus food TTL; revert to normal food if it expires.
        if self.food.tick_ttl() {
            self.food = Food::spawn(&mut self.rng, self.bounds, &self.snake);
        }

        // Periodically upgrade normal food to bonus food.
        if matches!(self.food.kind, FoodKind::Normal)
            && self.tick_count % BONUS_FOOD_SPAWN_INTERVAL_TICKS == 0
        {
            self.food = Food::spawn_bonus(&mut self.rng, self.bounds, &self.snake);
        }
    }

    /// Applies one external input event.
    pub fn apply_input(&mut self, input: GameInput) {
        match input {
            GameInput::Direction(direction) => {
                if self.status == GameStatus::Playing {
                    self.snake.buffer_direction(direction);
                }
            }
            GameInput::Pause => {
                self.status = match self.status {
                    GameStatus::Playing => GameStatus::Paused,
                    GameStatus::Paused => GameStatus::Playing,
                    other => other,
                };
            }
            GameInput::Quit | GameInput::Confirm => {}
        }
    }

    fn update_speed_level(&mut self) {
        self.speed_level = self.base_speed_level + (self.score / POINTS_PER_SPEED_LEVEL);
    }

    /// Returns immutable logical board bounds.
    #[must_use]
    pub fn bounds(&self) -> GridSize {
        self.bounds
    }

    /// Creates a fresh game state reusing the same grid bounds and starting speed.
    ///
    /// The returned state is in `Playing` status; the caller is responsible for
    /// setting it to `Paused` if it should start on the start/pause screen.
    #[must_use]
    pub fn restart(&self) -> Self {
        Self::new_with_options(self.bounds, self.base_speed_level)
    }

    /// Returns true when the game is on the initial start screen.
    ///
    /// The start screen is the paused state before any tick has run and before
    /// any score has been accumulated.
    #[must_use]
    pub fn is_start_screen(&self) -> bool {
        self.status == GameStatus::Paused && self.tick_count == 0 && self.score == 0
    }
}

#[cfg(test)]
mod tests {
    use crate::config::GridSize;
    use crate::food::Food;
    use crate::input::Direction;

    use super::{GameState, GameStatus};
    use crate::snake::{Position, Snake};

    #[test]
    fn snake_grows_after_eating_food() {
        let mut state = GameState::new_with_seed(GridSize { width: 10, height: 10 }, 1);
        state.snake = Snake::new(Position { x: 1, y: 1 }, Direction::Right);
        state.food = Food::normal(Position { x: 2, y: 1 });

        state.tick();
        assert_eq!(state.snake.len(), 2);
        assert_eq!(state.status, GameStatus::Playing);
    }

    #[test]
    fn snake_collision_with_wall_sets_game_over() {
        let mut state = GameState::new_with_seed(GridSize { width: 4, height: 4 }, 2);
        state.snake = Snake::new(Position { x: 3, y: 1 }, Direction::Right);

        state.tick();

        assert_eq!(state.status, GameStatus::GameOver);
    }

    #[test]
    fn snake_collision_with_self_sets_game_over() {
        let mut state = GameState::new_with_seed(GridSize { width: 6, height: 6 }, 3);
        state.snake = Snake::from_segments(
            vec![
                Position { x: 2, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 1, y: 3 },
                Position { x: 2, y: 3 },
                Position { x: 3, y: 3 },
                Position { x: 3, y: 2 },
            ],
            Direction::Left,
        );

        state.tick();

        assert_eq!(state.status, GameStatus::GameOver);
    }

    #[test]
    fn score_increments_when_food_is_eaten() {
        let mut state = GameState::new_with_seed(GridSize { width: 10, height: 10 }, 4);
        state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
        state.food = Food::normal(Position { x: 6, y: 5 });

        state.tick();

        assert_eq!(state.score, 1);
        assert_eq!(state.speed_level, 1);
    }

    #[test]
    fn starting_speed_level_is_respected() {
        let state = GameState::new_with_options(GridSize { width: 10, height: 10 }, 3);
        assert_eq!(state.speed_level, 3);
    }

    #[test]
    fn bonus_food_spawns_at_interval() {
        use crate::config::BONUS_FOOD_SPAWN_INTERVAL_TICKS;
        use crate::food::FoodKind;

        let mut state = GameState::new_with_seed(GridSize { width: 10, height: 10 }, 42);
        // Place the snake away from the food area and ensure normal food.
        state.snake = Snake::new(Position { x: 0, y: 0 }, Direction::Right);
        state.food = Food::normal(Position { x: 5, y: 5 });
        // Advance tick_count so the next tick lands on an interval boundary.
        state.tick_count = BONUS_FOOD_SPAWN_INTERVAL_TICKS - 1;

        state.tick();

        assert!(
            matches!(state.food.kind, FoodKind::Bonus { .. }),
            "food should be bonus after interval tick"
        );
    }

    #[test]
    fn bonus_food_reverts_to_normal_after_ttl_expires() {
        use crate::food::{FoodKind, BONUS_FOOD_LIFETIME_TICKS};

        // Grid must be wide enough that the snake doesn't hit the wall before
        // the bonus TTL expires. BONUS_FOOD_LIFETIME_TICKS + margin is enough.
        let wide_bounds = GridSize {
            width: BONUS_FOOD_LIFETIME_TICKS + 20,
            height: 10,
        };
        let mut state = GameState::new_with_seed(wide_bounds, 99);
        // Snake moves right along y=0; food is on a different row, no collision.
        state.snake = Snake::new(Position { x: 0, y: 0 }, Direction::Right);
        state.food = Food::bonus(Position { x: 5, y: 5 });

        for _ in 0..=BONUS_FOOD_LIFETIME_TICKS {
            state.tick();
        }

        assert_eq!(state.status, GameStatus::Playing, "game should still be running");
        assert!(
            matches!(state.food.kind, FoodKind::Normal),
            "food should revert to normal after TTL expires"
        );
    }
}
