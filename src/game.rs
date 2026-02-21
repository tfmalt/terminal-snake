use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::config::POINTS_PER_SPEED_LEVEL;
use crate::food::Food;
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

/// Complete mutable game state for one session.
#[derive(Debug, Clone)]
pub struct GameState {
    pub snake: Snake,
    pub food: Food,
    pub score: u32,
    pub speed_level: u32,
    pub tick_count: u64,
    pub status: GameStatus,
    bounds: (u16, u16),
    rng: StdRng,
}

impl GameState {
    /// Creates a deterministic state for tests and reproducible simulations.
    #[must_use]
    pub fn new_with_seed(bounds: (u16, u16), seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let start = Position {
            x: i32::from(bounds.0 / 2),
            y: i32::from(bounds.1 / 2),
        };
        let snake = Snake::new(start, crate::input::Direction::Right);
        let food = Food::spawn(&mut rng, bounds, &snake);

        Self {
            snake,
            food,
            score: 0,
            speed_level: 1,
            tick_count: 0,
            status: GameStatus::Playing,
            bounds,
            rng,
        }
    }

    /// Advances simulation by one gameplay tick.
    pub fn tick(&mut self) {
        if self.status != GameStatus::Playing {
            return;
        }

        self.tick_count += 1;
        self.snake.move_forward(self.bounds);

        let head = self.snake.head();
        if !head.is_within_bounds(self.bounds) {
            self.status = GameStatus::GameOver;
            return;
        }

        if self.snake.head_overlaps_body() {
            self.status = GameStatus::GameOver;
            return;
        }

        if head == self.food.position {
            self.score += self.food.points();
            self.snake.grow_next();
            self.update_speed_level();

            if self.snake.len() == usize::from(self.bounds.0) * usize::from(self.bounds.1) {
                self.status = GameStatus::Victory;
                return;
            }

            self.food = Food::spawn(&mut self.rng, self.bounds, &self.snake);
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
        self.speed_level = 1 + (self.score / POINTS_PER_SPEED_LEVEL);
    }
}

#[cfg(test)]
mod tests {
    use crate::food::Food;
    use crate::input::Direction;

    use super::{GameState, GameStatus};
    use crate::snake::{Position, Snake};

    #[test]
    fn snake_grows_after_eating_food() {
        let mut state = GameState::new_with_seed((10, 10), 1);
        state.snake = Snake::new(Position { x: 1, y: 1 }, Direction::Right);
        state.food = Food::normal(Position { x: 2, y: 1 });

        state.tick();
        assert_eq!(state.snake.len(), 1);

        state.tick();
        assert_eq!(state.snake.len(), 2);
    }

    #[test]
    fn snake_collision_with_wall_sets_game_over() {
        let mut state = GameState::new_with_seed((4, 4), 2);
        state.snake = Snake::new(Position { x: 3, y: 1 }, Direction::Right);

        state.tick();

        assert_eq!(state.status, GameStatus::GameOver);
    }

    #[test]
    fn snake_collision_with_self_sets_game_over() {
        let mut state = GameState::new_with_seed((6, 6), 3);
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
        let mut state = GameState::new_with_seed((10, 10), 4);
        state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
        state.food = Food::normal(Position { x: 6, y: 5 });

        state.tick();

        assert_eq!(state.score, 1);
        assert_eq!(state.speed_level, 1);
    }
}
