use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::time::Duration;

use crate::config::{GridSize, POINTS_PER_SPEED_LEVEL};
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
    pub foods: Vec<Food>,
    pub score: u32,
    pub speed_level: u32,
    pub tick_count: u64,
    pub status: GameStatus,
    pub death_reason: Option<DeathReason>,
    elapsed_millis: u64,
    bounds: GridSize,
    base_speed_level: u32,
    food_density: FoodDensity,
    rng: StdRng,
}

/// Configures food density as `foods_per` cells_per cells.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FoodDensity {
    pub foods_per: usize,
    pub cells_per: usize,
}

/// Returns the default food density configuration.
#[must_use]
pub fn default_food_density() -> FoodDensity {
    FoodDensity {
        foods_per: 1,
        cells_per: 200,
    }
}

impl GameState {
    /// Creates a runtime state with non-deterministic RNG seeding.
    #[must_use]
    pub fn new(bounds: GridSize) -> Self {
        let seed = rand::random::<u64>();
        Self::new_with_seed_speed_and_food(bounds, seed, 1, default_food_density())
    }

    /// Creates a runtime state with explicit starting speed level.
    #[must_use]
    pub fn new_with_options(bounds: GridSize, starting_speed_level: u32) -> Self {
        Self::new_with_options_and_food_density(
            bounds,
            starting_speed_level,
            default_food_density(),
        )
    }

    /// Creates a runtime state with explicit starting speed and food density.
    #[must_use]
    pub fn new_with_options_and_food_density(
        bounds: GridSize,
        starting_speed_level: u32,
        food_density: FoodDensity,
    ) -> Self {
        let seed = rand::random::<u64>();
        Self::new_with_seed_speed_and_food(bounds, seed, starting_speed_level, food_density)
    }

    /// Creates a deterministic state for tests and reproducible simulations.
    #[must_use]
    pub fn new_with_seed(bounds: GridSize, seed: u64) -> Self {
        Self::new_with_seed_speed_and_food(bounds, seed, 1, default_food_density())
    }

    fn new_with_seed_speed_and_food(
        bounds: GridSize,
        seed: u64,
        starting_speed_level: u32,
        food_density: FoodDensity,
    ) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        let base_speed_level = starting_speed_level.max(1);
        let normalized_density = normalize_food_density(food_density);
        let start = Position {
            x: i32::from(bounds.width / 2),
            y: i32::from(bounds.height / 2),
        };
        let snake = Snake::new(start, crate::input::Direction::Right);
        let foods = Vec::new();

        let mut state = Self {
            snake,
            foods,
            score: 0,
            speed_level: base_speed_level,
            tick_count: 0,
            status: GameStatus::Playing,
            death_reason: None,
            elapsed_millis: 0,
            bounds,
            base_speed_level,
            food_density: normalized_density,
            rng,
        };

        state.sync_food_count_to_density();
        state
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

        let eaten_food_idx = self
            .foods
            .iter()
            .position(|food| next_head == food.position);
        let ate_food = eaten_food_idx.is_some();
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
            let eaten_food = self
                .foods
                .swap_remove(eaten_food_idx.expect("food index should exist when eaten"));
            self.score += eaten_food.points();
            self.update_speed_level();

            if self.snake.len() >= self.bounds.total_cells() {
                self.status = GameStatus::Victory;
                self.death_reason = None;
                return;
            }

            self.sync_food_count_to_density();
        }
    }

    /// Resizes the logical game bounds and reconciles snake/food state.
    pub fn resize_bounds(&mut self, bounds: GridSize) {
        self.bounds = bounds;
        self.snake.wrap_into_bounds(bounds);

        self.foods.retain(|food| {
            food.position.is_within_bounds(bounds) && !self.snake.occupies(food.position)
        });
        dedupe_food_positions(&mut self.foods);

        if self.snake.len() >= self.bounds.total_cells() {
            self.status = GameStatus::Victory;
            self.death_reason = None;
            return;
        }

        self.sync_food_count_to_density();
    }

    /// Updates the configured food density and applies it immediately.
    pub fn set_food_density(&mut self, food_density: FoodDensity) {
        self.food_density = normalize_food_density(food_density);
        self.sync_food_count_to_density();
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
            GameInput::Quit | GameInput::Confirm | GameInput::CycleTheme | GameInput::Resize => {}
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
        Self::new_with_options_and_food_density(
            self.bounds,
            self.base_speed_level,
            self.food_density,
        )
    }

    /// Returns true when the game is on the initial start screen.
    ///
    /// The start screen is the paused state before any tick has run and before
    /// any score has been accumulated.
    #[must_use]
    pub fn is_start_screen(&self) -> bool {
        self.status == GameStatus::Paused && self.tick_count == 0 && self.score == 0
    }

    /// Adds gameplay time for one simulation step.
    pub fn record_tick_duration(&mut self, duration: Duration) {
        self.elapsed_millis = self
            .elapsed_millis
            .saturating_add(duration.as_millis().min(u128::from(u64::MAX)) as u64);
    }

    /// Returns total gameplay duration accumulated from simulation ticks.
    #[must_use]
    pub fn elapsed_duration(&self) -> Duration {
        Duration::from_millis(self.elapsed_millis)
    }

    /// Returns the currently calculated food target from density and free cells.
    #[must_use]
    pub fn calculated_food_count(&self) -> usize {
        desired_food_count(self.bounds, self.snake.len(), self.food_density)
    }

    fn sync_food_count_to_density(&mut self) {
        let target_count = desired_food_count(self.bounds, self.snake.len(), self.food_density);

        if self.foods.len() > target_count {
            self.foods.truncate(target_count);
        }

        while self.foods.len() < target_count {
            let Some(food) =
                spawn_food_avoiding(&mut self.rng, self.bounds, &self.snake, &self.foods)
            else {
                break;
            };
            self.foods.push(food);
        }
    }
}

fn normalize_food_density(food_density: FoodDensity) -> FoodDensity {
    FoodDensity {
        foods_per: food_density.foods_per.max(1),
        cells_per: food_density.cells_per.max(1),
    }
}

fn desired_food_count(bounds: GridSize, snake_len: usize, food_density: FoodDensity) -> usize {
    let free_cells = bounds.total_cells().saturating_sub(snake_len);
    if free_cells == 0 {
        return 0;
    }

    let desired = free_cells.saturating_mul(food_density.foods_per) / food_density.cells_per;
    desired.max(1).min(free_cells)
}

fn dedupe_food_positions(foods: &mut Vec<Food>) {
    let mut unique = Vec::with_capacity(foods.len());
    for food in foods.drain(..) {
        if unique
            .iter()
            .any(|existing: &Food| existing.position == food.position)
        {
            continue;
        }
        unique.push(food);
    }
    *foods = unique;
}

fn spawn_food_avoiding<R: Rng + ?Sized>(
    rng: &mut R,
    bounds: GridSize,
    snake: &Snake,
    existing_foods: &[Food],
) -> Option<Food> {
    let mut candidates = Vec::new();

    for y in 0..i32::from(bounds.height) {
        for x in 0..i32::from(bounds.width) {
            let position = Position { x, y };
            if snake.occupies(position) {
                continue;
            }

            if existing_foods.iter().any(|food| food.position == position) {
                continue;
            }

            candidates.push(position);
        }
    }

    if candidates.is_empty() {
        return None;
    }

    let index = rng.gen_range(0..candidates.len());
    Some(Food::new(candidates[index]))
}

#[cfg(test)]
mod tests {
    use crate::config::GridSize;
    use crate::food::Food;
    use crate::input::Direction;

    use super::{GameState, GameStatus};
    use crate::input::GameInput;
    use crate::snake::{Position, Snake};

    #[test]
    fn snake_grows_after_eating_food() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 10,
                height: 10,
            },
            1,
        );
        state.snake = Snake::new(Position { x: 1, y: 1 }, Direction::Right);
        state.foods = vec![Food::new(Position { x: 2, y: 1 })];

        state.tick();
        assert_eq!(state.snake.len(), 3);
        assert_eq!(state.status, GameStatus::Playing);
    }

    #[test]
    fn snake_collision_with_wall_sets_game_over() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 4,
                height: 4,
            },
            2,
        );
        state.snake = Snake::new(Position { x: 3, y: 1 }, Direction::Right);

        state.tick();

        assert_eq!(state.status, GameStatus::GameOver);
    }

    #[test]
    fn snake_collision_with_self_sets_game_over() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 6,
                height: 6,
            },
            3,
        );
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
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 10,
                height: 10,
            },
            4,
        );
        state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
        state.foods = vec![Food::new(Position { x: 6, y: 5 })];

        state.tick();

        assert_eq!(state.score, 1);
        assert_eq!(state.speed_level, 1);
    }

    #[test]
    fn starting_speed_level_is_respected() {
        let state = GameState::new_with_options(
            GridSize {
                width: 10,
                height: 10,
            },
            3,
        );
        assert_eq!(state.speed_level, 3);
    }

    #[test]
    fn player_can_turn_at_last_cell_before_wall() {
        let bounds = GridSize {
            width: 10,
            height: 10,
        };
        let mut state = GameState::new_with_seed(bounds, 10);
        // Place the snake at the second-to-last cell heading right.
        state.snake = Snake::new(Position { x: 8, y: 5 }, Direction::Right);
        state.foods = vec![Food::new(Position { x: 0, y: 0 })];

        // Tick moves the snake to x=9 (last cell). Should NOT be game over.
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
        assert_eq!(state.snake.head(), Position { x: 9, y: 5 });

        // Player changes direction before the next tick.
        state.apply_input(GameInput::Direction(Direction::Down));

        // Next tick moves the snake down instead of into the wall.
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
        assert_eq!(state.snake.head(), Position { x: 9, y: 6 });
    }

    #[test]
    fn player_can_reach_top_row_and_turn() {
        let bounds = GridSize {
            width: 10,
            height: 10,
        };
        let mut state = GameState::new_with_seed(bounds, 10);
        state.snake = Snake::new(Position { x: 5, y: 1 }, Direction::Up);
        state.foods = vec![Food::new(Position { x: 9, y: 9 })];

        // Tick moves the snake to y=0 (top row). Should NOT be game over.
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
        assert_eq!(state.snake.head(), Position { x: 5, y: 0 });

        // Player changes direction before the next tick.
        state.apply_input(GameInput::Direction(Direction::Right));

        // Next tick moves right instead of into the wall.
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
        assert_eq!(state.snake.head(), Position { x: 6, y: 0 });
    }

    #[test]
    fn player_can_reach_bottom_row_and_turn() {
        let bounds = GridSize {
            width: 10,
            height: 10,
        };
        let mut state = GameState::new_with_seed(bounds, 10);
        state.snake = Snake::new(Position { x: 5, y: 8 }, Direction::Down);
        state.foods = vec![Food::new(Position { x: 9, y: 0 })];

        // Tick moves the snake to y=9 (bottom row). Should NOT be game over.
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
        assert_eq!(state.snake.head(), Position { x: 5, y: 9 });

        // Player changes direction before the next tick.
        state.apply_input(GameInput::Direction(Direction::Right));

        // Next tick moves right instead of into the wall.
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
        assert_eq!(state.snake.head(), Position { x: 6, y: 9 });
    }
}
