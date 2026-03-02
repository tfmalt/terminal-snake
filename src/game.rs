use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::config::{
    DEFAULT_TICK_INTERVAL_MS, FOOD_PER_SPEED_LEVEL, GridSize, MAX_START_SPEED_LEVEL,
    MIN_TICK_INTERVAL_MS,
};
use crate::food::Food;
use crate::input::GameInput;
use crate::snake::{Position, Snake};

/// Current high-level gameplay state.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum GameStatus {
    Playing,
    Paused,
    GameOver,
    Victory,
}

/// Why the most recent game-over state was reached.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeathReason {
    WallCollision,
    SelfCollision,
}

/// What triggered a glow effect on the snake.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum GlowTrigger {
    SpeedLevelUp,
    SuperFoodEaten,
}

/// A temporary visual pulse that fades over a fixed wall-clock duration.
#[derive(Debug, Clone, Copy)]
pub struct GlowEffect {
    pub trigger: GlowTrigger,
    started_at: Instant,
    duration: Duration,
}

impl GlowEffect {
    const SPEED_LEVEL_UP_DURATION: Duration = Duration::from_secs(3);
    const SUPER_FOOD_MIN_DURATION_SECONDS: f32 = 0.45;
    const SUPER_FOOD_MAX_DURATION_SECONDS: f32 = 1.8;
    pub const SUPER_FOOD_RIPPLE_SPEED_MULTIPLIER: f32 = 5.0;

    /// Creates a speed-level glow effect.
    #[must_use]
    pub fn speed_level_up() -> Self {
        Self {
            trigger: GlowTrigger::SpeedLevelUp,
            started_at: Instant::now(),
            duration: Self::SPEED_LEVEL_UP_DURATION,
        }
    }

    /// Creates a super-food ripple effect that lasts long enough to traverse
    /// the current snake body.
    #[must_use]
    pub fn super_food_ripple(snake_len: usize, speed_level: u32) -> Self {
        Self {
            trigger: GlowTrigger::SuperFoodEaten,
            started_at: Instant::now(),
            duration: Self::super_food_duration(snake_len, speed_level),
        }
    }

    /// Returns normalized effect progress where `0.0` is fresh and `1.0` is expired.
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }

        let elapsed = self.started_at.elapsed();
        if elapsed >= self.duration {
            return 1.0;
        }

        elapsed.as_secs_f32() / self.duration.as_secs_f32()
    }

    /// Returns the current intensity as a value from 1.0 (fresh) to 0.0 (expired).
    #[must_use]
    pub fn intensity(&self) -> f32 {
        if self.duration.is_zero() {
            return 0.0;
        }

        1.0 - self.progress()
    }

    /// Returns `true` while the effect is still within its duration window.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.progress() < 1.0
    }

    /// Returns elapsed wall-clock time since the effect started.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    fn super_food_duration(snake_len: usize, speed_level: u32) -> Duration {
        let tick_seconds = tick_interval_for_speed(speed_level).as_secs_f32();
        if tick_seconds <= 0.0 {
            return Duration::from_millis(1);
        }

        let snake_cells_per_second = 1.0 / tick_seconds;
        let ripple_cells_per_second =
            snake_cells_per_second * Self::SUPER_FOOD_RIPPLE_SPEED_MULTIPLIER;
        if ripple_cells_per_second <= 0.0 {
            return Duration::from_millis(1);
        }

        let body_len = snake_len.max(1) as f32;
        let traversal_seconds = body_len / ripple_cells_per_second;
        let padded_seconds = traversal_seconds + tick_seconds;
        let clamped_seconds = padded_seconds.clamp(
            Self::SUPER_FOOD_MIN_DURATION_SECONDS,
            Self::SUPER_FOOD_MAX_DURATION_SECONDS,
        );
        Duration::from_secs_f32(clamped_seconds)
    }
}

fn tick_interval_for_speed(speed_level: u32) -> Duration {
    let speed_penalty_ms = u64::from(speed_level.saturating_sub(1)) * 10;
    let clamped_ms = DEFAULT_TICK_INTERVAL_MS
        .saturating_sub(speed_penalty_ms)
        .max(MIN_TICK_INTERVAL_MS);
    Duration::from_millis(clamped_ms)
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
    glow: Option<GlowEffect>,
    elapsed_millis: u64,
    bounds: GridSize,
    base_speed_level: u32,
    food_density: FoodDensity,
    rng_seed: u64,
    rng_draws: u64,
    rng: StdRng,
}

/// Configures food density as `foods_per` cells_per cells.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct FoodDensity {
    pub foods_per: usize,
    pub cells_per: usize,
}

const COVERAGE_BONUS_RATE: f64 = 0.10;
const COVERAGE_BONUS_CAP: f64 = 9.0;

/// Returns the default food density configuration.
#[must_use]
pub fn default_food_density() -> FoodDensity {
    FoodDensity {
        foods_per: 1,
        cells_per: 200,
    }
}

/// Serializable game snapshot used for local resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshotV1 {
    pub bounds: GridSize,
    pub snake: SnakeSnapshotV1,
    pub foods: Vec<Food>,
    pub score: u32,
    pub speed_level: u32,
    pub tick_count: u64,
    pub status: GameStatus,
    pub death_reason: Option<DeathReason>,
    pub elapsed_millis: u64,
    pub base_speed_level: u32,
    pub food_density: FoodDensity,
    pub rng: RngSnapshotV1,
}

/// Serializable snake snapshot used by [`GameSnapshotV1`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnakeSnapshotV1 {
    pub segments: Vec<Position>,
    pub direction: crate::input::Direction,
    pub buffered_direction: crate::input::Direction,
    pub next_buffered_direction: Option<crate::input::Direction>,
    pub grow_remaining: u32,
}

/// Serializable RNG state needed for deterministic resume.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RngSnapshotV1 {
    pub seed: u64,
    pub draws: u64,
}

/// Snapshot restore failures.
#[derive(Debug, Error)]
pub enum SnapshotRestoreError {
    #[error("snapshot must contain at least one snake segment")]
    EmptySnake,
    #[error("snapshot contains snake segment out of bounds")]
    SnakeOutOfBounds,
    #[error("snapshot contains food out of bounds")]
    FoodOutOfBounds,
    #[error("snapshot contains duplicate food positions")]
    DuplicateFood,
    #[error("snapshot food overlaps snake")]
    FoodOverlapsSnake,
    #[error("snapshot has non-resumable status {0:?}")]
    NonResumableStatus(GameStatus),
    #[error("snapshot contains invalid snake data")]
    InvalidSnake,
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
        let base_speed_level = starting_speed_level.clamp(1, MAX_START_SPEED_LEVEL);
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
            glow: None,
            elapsed_millis: 0,
            bounds,
            base_speed_level,
            food_density: normalized_density,
            rng_seed: seed,
            rng_draws: 0,
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

        if self.glow.as_ref().is_some_and(|glow| !glow.is_active()) {
            self.glow = None;
        }

        // Tick super food timers and degrade expired ones to normal.
        for food in &mut self.foods {
            if food.is_super() && !food.tick() {
                food.degrade();
            }
        }

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
        let eaten_growth = eaten_food_idx.map(|idx| self.foods[idx].growth());
        if let Some(growth) = eaten_growth {
            self.snake.grow_by(growth);
        }

        self.snake.move_forward(self.bounds);

        if self.snake.head_overlaps_body() {
            self.status = GameStatus::GameOver;
            self.death_reason = Some(DeathReason::SelfCollision);
            return;
        }

        if let Some(idx) = eaten_food_idx {
            let eaten_food = self.foods.swap_remove(idx);
            let base_points = eaten_food.points() * self.speed_level;
            let awarded_points =
                score_with_coverage_bonus(base_points, self.play_area_coverage_percent());
            self.score += awarded_points;
            let prev_speed_level = self.speed_level;
            self.update_speed_level();

            if eaten_food.is_super() {
                self.glow = Some(GlowEffect::super_food_ripple(
                    self.snake.len(),
                    self.speed_level,
                ));
            } else if self.speed_level > prev_speed_level {
                self.glow = Some(GlowEffect::speed_level_up());
            }

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

    /// Updates the base starting speed without touching RNG, food, or snake state.
    ///
    /// Use this when the player adjusts the speed selector on the start screen —
    /// it keeps the backdrop (food positions, snake) stable across keypresses.
    pub fn set_base_speed_level(&mut self, level: u32) {
        self.base_speed_level = level.clamp(1, MAX_START_SPEED_LEVEL);
        self.speed_level = self.base_speed_level;
    }

    fn update_speed_level(&mut self) {
        let mut level = self.base_speed_level;
        let mut remaining_food = self.snake.len().saturating_sub(2) as u32;

        loop {
            let required_for_next = Self::food_required_for_next_level(level);
            if required_for_next == 0 || remaining_food < required_for_next {
                break;
            }

            remaining_food -= required_for_next;
            let next = level.saturating_add(1);
            if next == level {
                break;
            }
            level = next;
        }

        self.speed_level = level;
    }

    fn food_required_for_next_level(level: u32) -> u32 {
        if level <= 5 {
            FOOD_PER_SPEED_LEVEL.saturating_add(level)
        } else if level <= 10 {
            FOOD_PER_SPEED_LEVEL.saturating_add(level.saturating_mul(2))
        } else {
            level.saturating_mul(FOOD_PER_SPEED_LEVEL)
        }
    }

    /// Returns the currently active glow effect, if any.
    #[must_use]
    pub fn active_glow(&self) -> Option<&GlowEffect> {
        self.glow.as_ref()
    }

    /// Returns immutable logical board bounds.
    #[must_use]
    pub fn bounds(&self) -> GridSize {
        self.bounds
    }

    /// Captures a deterministic snapshot that can be restored later.
    #[must_use]
    pub fn to_snapshot_v1(&self) -> GameSnapshotV1 {
        GameSnapshotV1 {
            bounds: self.bounds,
            snake: SnakeSnapshotV1 {
                segments: self.snake.segments().copied().collect(),
                direction: self.snake.direction(),
                buffered_direction: self.snake.buffered_direction(),
                next_buffered_direction: self.snake.next_buffered_direction(),
                grow_remaining: self.snake.grow_remaining(),
            },
            foods: self.foods.clone(),
            score: self.score,
            speed_level: self.speed_level,
            tick_count: self.tick_count,
            status: self.status,
            death_reason: self.death_reason,
            elapsed_millis: self.elapsed_millis,
            base_speed_level: self.base_speed_level,
            food_density: self.food_density,
            rng: RngSnapshotV1 {
                seed: self.rng_seed,
                draws: self.rng_draws,
            },
        }
    }

    /// Restores game state from a previously captured snapshot.
    pub fn from_snapshot_v1(snapshot: GameSnapshotV1) -> Result<Self, SnapshotRestoreError> {
        if snapshot.snake.segments.is_empty() {
            return Err(SnapshotRestoreError::EmptySnake);
        }

        if !matches!(snapshot.status, GameStatus::Playing | GameStatus::Paused) {
            return Err(SnapshotRestoreError::NonResumableStatus(snapshot.status));
        }

        if snapshot
            .snake
            .segments
            .iter()
            .any(|segment| !segment.is_within_bounds(snapshot.bounds))
        {
            return Err(SnapshotRestoreError::SnakeOutOfBounds);
        }

        if snapshot
            .foods
            .iter()
            .any(|food| !food.position.is_within_bounds(snapshot.bounds))
        {
            return Err(SnapshotRestoreError::FoodOutOfBounds);
        }

        let mut seen_food_positions =
            std::collections::HashSet::with_capacity(snapshot.foods.len());
        for food in &snapshot.foods {
            if !seen_food_positions.insert(food.position) {
                return Err(SnapshotRestoreError::DuplicateFood);
            }
        }

        let snake = Snake::from_parts(
            snapshot.snake.segments,
            snapshot.snake.direction,
            snapshot.snake.buffered_direction,
            snapshot.snake.next_buffered_direction,
            snapshot.snake.grow_remaining,
        )
        .map_err(|_| SnapshotRestoreError::InvalidSnake)?;

        if snapshot
            .foods
            .iter()
            .any(|food| snake.occupies(food.position))
        {
            return Err(SnapshotRestoreError::FoodOverlapsSnake);
        }

        let mut rng = StdRng::seed_from_u64(snapshot.rng.seed);
        advance_rng_draws(&mut rng, snapshot.rng.draws);

        let mut restored = Self {
            snake,
            foods: snapshot.foods,
            score: snapshot.score,
            speed_level: snapshot.speed_level,
            tick_count: snapshot.tick_count,
            status: GameStatus::Paused,
            death_reason: snapshot.death_reason,
            glow: None,
            elapsed_millis: snapshot.elapsed_millis,
            bounds: snapshot.bounds,
            base_speed_level: snapshot.base_speed_level,
            food_density: normalize_food_density(snapshot.food_density),
            rng_seed: snapshot.rng.seed,
            rng_draws: snapshot.rng.draws,
            rng,
        };

        if restored.snake.len() >= restored.bounds.total_cells() {
            restored.status = GameStatus::Victory;
            restored.death_reason = None;
        } else {
            restored.sync_food_count_to_density();
        }

        Ok(restored)
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

    /// Returns the current base point value of ordinary food.
    ///
    /// This reflects speed scaling only (`1 * speed_level`) and intentionally
    /// ignores board state and coverage bonus.
    #[must_use]
    pub fn ordinary_food_base_points(&self) -> u32 {
        self.speed_level
    }

    /// Returns projected ordinary-food points including the coverage bonus.
    ///
    /// This mirrors the runtime scoring order in `tick()`: the snake grows
    /// first, then score is awarded using post-growth coverage.
    #[must_use]
    pub fn ordinary_food_projected_points(&self) -> u32 {
        score_with_coverage_bonus(
            self.ordinary_food_base_points(),
            self.coverage_percent_after_growth(1),
        )
    }

    /// Returns projected ordinary-food score multiplier including coverage bonus.
    #[must_use]
    pub fn ordinary_food_projected_multiplier(&self) -> f64 {
        coverage_total_multiplier(self.coverage_percent_after_growth(1))
    }

    /// Returns how many foods are needed to reach the next speed level.
    #[must_use]
    pub fn foods_until_next_speed_level(&self) -> u32 {
        let mut level = self.base_speed_level;
        let mut remaining_food = self.snake.len().saturating_sub(2) as u32;

        loop {
            let required_for_next = Self::food_required_for_next_level(level);
            if required_for_next == 0 {
                return 0;
            }
            if remaining_food < required_for_next {
                return required_for_next - remaining_food;
            }

            remaining_food -= required_for_next;
            let next = level.saturating_add(1);
            if next == level {
                return 0;
            }
            level = next;
        }
    }

    fn coverage_percent_after_growth(&self, growth: usize) -> f64 {
        let total_cells = self.bounds.total_cells();
        if total_cells == 0 {
            return 0.0;
        }

        let projected_len = self.snake.len().saturating_add(growth).min(total_cells);
        (projected_len as f64 / total_cells as f64) * 100.0
    }

    /// Returns the snake coverage of the full play area as a percentage.
    #[must_use]
    pub fn play_area_coverage_percent(&self) -> f64 {
        let total_cells = self.bounds.total_cells();
        if total_cells == 0 {
            return 0.0;
        }

        (self.snake.len() as f64 / total_cells as f64) * 100.0
    }

    fn sync_food_count_to_density(&mut self) {
        let target_count = desired_food_count(self.bounds, self.snake.len(), self.food_density);

        if self.foods.len() > target_count {
            self.foods.truncate(target_count);
        }

        while self.foods.len() < target_count {
            let candidates = available_food_positions(self.bounds, &self.snake, &self.foods);
            if candidates.is_empty() {
                break;
            }

            let mut food = Food::new(candidates[self.random_usize_below(candidates.len())]);

            // 30% chance to upgrade newly spawned food to super food
            // (only after the game has started — initial food is always normal).
            if self.tick_count > 0 && self.random_u32_below(100) < 30 {
                let head = self.snake.head();
                let distance = (head.x - food.position.x).unsigned_abs()
                    + (head.y - food.position.y).unsigned_abs();
                food = Food::new_super(food.position, distance + 10);
            }

            self.foods.push(food);
        }
    }

    fn random_usize_below(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        self.rng_draws = self.rng_draws.saturating_add(1);
        (self.rng.next_u64() % upper as u64) as usize
    }

    fn random_u32_below(&mut self, upper: u32) -> u32 {
        debug_assert!(upper > 0);
        self.rng_draws = self.rng_draws.saturating_add(1);
        (self.rng.next_u64() % u64::from(upper)) as u32
    }
}

fn score_with_coverage_bonus(base_points: u32, coverage_percent: f64) -> u32 {
    let total = (base_points as f64) * coverage_total_multiplier(coverage_percent);
    total.floor() as u32
}

fn coverage_total_multiplier(coverage_percent: f64) -> f64 {
    let bonus_multiplier = (coverage_percent * COVERAGE_BONUS_RATE).min(COVERAGE_BONUS_CAP);
    1.0 + bonus_multiplier
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

fn available_food_positions(
    bounds: GridSize,
    snake: &Snake,
    existing_foods: &[Food],
) -> Vec<Position> {
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

    candidates
}

fn advance_rng_draws(rng: &mut StdRng, draws: u64) {
    for _ in 0..draws {
        let _ = rng.next_u64();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::config::{GridSize, MAX_START_SPEED_LEVEL};
    use crate::food::Food;
    use crate::input::Direction;

    use super::{FoodDensity, GameState, GameStatus, SnapshotRestoreError};
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
        )
        .expect("test snake segments should be valid");

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
    fn starting_speed_level_is_clamped_to_max() {
        let state = GameState::new_with_options(
            GridSize {
                width: 10,
                height: 10,
            },
            MAX_START_SPEED_LEVEL + 5,
        );
        assert_eq!(state.speed_level, MAX_START_SPEED_LEVEL);
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

    #[test]
    fn score_multiplied_by_speed_level() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 20,
                height: 20,
            },
            42,
        );
        state.set_base_speed_level(3);
        state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
        state.foods = vec![Food::new(Position { x: 6, y: 5 })];

        state.tick();

        assert_eq!(state.score, 3, "score should be 1 * speed_level(3)");
    }

    #[test]
    fn coverage_percent_uses_snake_length_over_total_cells() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 20,
                height: 20,
            },
            12,
        );
        state.snake = Snake::new(Position { x: 2, y: 2 }, Direction::Right);

        // Initial snake length is 2 on a 400-cell board: 0.50%
        assert!((state.play_area_coverage_percent() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn coverage_bonus_increases_points() {
        let base_points = 10;
        let points = super::score_with_coverage_bonus(base_points, 10.0);
        assert_eq!(points, 20, "10 base with 10% coverage should be 2x");
    }

    #[test]
    fn coverage_bonus_is_capped() {
        let base_points = 10;
        let points = super::score_with_coverage_bonus(base_points, 100.0);
        assert_eq!(
            points, 100,
            "bonus cap of 9.0 should limit total to 10x base"
        );
    }

    #[test]
    fn level_progression_uses_tiered_food_thresholds() {
        let bounds = GridSize {
            width: 500,
            height: 200,
        };
        let mut state = GameState::new_with_seed_speed_and_food(
            bounds,
            55,
            1,
            FoodDensity {
                foods_per: 1,
                cells_per: usize::MAX,
            },
        );

        let cases = [
            (5, 1),
            (6, 2),
            (13, 3),
            (31, 5),
            (41, 6),
            (58, 7),
            (150, 11),
            (205, 12),
        ];

        for (food_eaten, expected_level) in cases {
            let len = (food_eaten + 2) as i32;
            let segments = (0..len)
                .map(|i| Position { x: 300 - i, y: 10 })
                .collect::<Vec<_>>();
            state.snake = Snake::from_segments(segments, Direction::Right)
                .expect("snake segments should be valid");

            state.update_speed_level();

            assert_eq!(
                state.speed_level, expected_level,
                "food_eaten={food_eaten} should yield level {expected_level}"
            );
        }
    }

    #[test]
    fn speed_level_continues_past_starting_speed_cap() {
        let mut state = GameState::new_with_seed_speed_and_food(
            GridSize {
                width: 200,
                height: 200,
            },
            99,
            MAX_START_SPEED_LEVEL,
            FoodDensity {
                foods_per: 1,
                cells_per: usize::MAX,
            },
        );
        let segments = (0..77)
            .map(|i| Position { x: 90 - i, y: 10 })
            .collect::<Vec<_>>();
        state.snake = Snake::from_segments(segments, Direction::Right)
            .expect("snake segments should be valid");
        state.foods = vec![Food::new(Position { x: 91, y: 10 })];

        state.tick();

        assert_eq!(state.speed_level, MAX_START_SPEED_LEVEL + 1);
        assert_eq!(state.score, MAX_START_SPEED_LEVEL);
        assert_eq!(
            state.active_glow().map(|g| g.trigger),
            Some(super::GlowTrigger::SpeedLevelUp)
        );
    }

    #[test]
    fn scoring_uses_levels_beyond_fifteen() {
        let mut state = GameState::new_with_seed_speed_and_food(
            GridSize {
                width: 200,
                height: 200,
            },
            101,
            MAX_START_SPEED_LEVEL,
            FoodDensity {
                foods_per: 1,
                cells_per: usize::MAX,
            },
        );
        let segments = (0..77)
            .map(|i| Position { x: 90 - i, y: 10 })
            .collect::<Vec<_>>();
        state.snake = Snake::from_segments(segments, Direction::Right)
            .expect("snake segments should be valid");
        state.foods = vec![Food::new(Position { x: 91, y: 10 })];

        state.tick();
        assert_eq!(state.speed_level, MAX_START_SPEED_LEVEL + 1);
        assert_eq!(state.score, MAX_START_SPEED_LEVEL);

        state.foods = vec![Food::new(Position { x: 92, y: 10 })];
        state.tick();

        assert_eq!(state.speed_level, MAX_START_SPEED_LEVEL + 1);
        assert_eq!(state.score, (MAX_START_SPEED_LEVEL * 2) + 1);
    }

    #[test]
    fn super_food_ripple_duration_has_minimum() {
        let duration = super::GlowEffect::super_food_duration(1, 100);
        assert!(duration >= Duration::from_millis(440));
    }

    #[test]
    fn super_food_ripple_duration_has_maximum() {
        let duration = super::GlowEffect::super_food_duration(2_000, 1);
        assert!(duration <= Duration::from_millis(1_800));
    }

    #[test]
    fn ordinary_food_base_points_tracks_speed_level() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 8,
                height: 8,
            },
            17,
        );
        state.set_base_speed_level(4);

        assert_eq!(state.ordinary_food_base_points(), 4);
    }

    #[test]
    fn ordinary_food_projected_points_include_coverage_bonus() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 4,
                height: 4,
            },
            22,
        );
        state.set_base_speed_level(4);

        // Length 2 on 16 cells; ordinary eat grows to 3 => 18.75% coverage.
        // base=4, bonus=min(18.75 * 0.10, 2.0)=1.875
        // projected=floor(4 * (1 + 1.875))=11
        assert_eq!(state.ordinary_food_projected_points(), 11);
    }

    #[test]
    fn ordinary_food_projected_multiplier_matches_formula() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 4,
                height: 4,
            },
            31,
        );
        state.set_base_speed_level(2);

        // Post-growth coverage is 18.75%, so multiplier is 1 + min(18.75 * 0.10, 2.0) = 2.875.
        assert!((state.ordinary_food_projected_multiplier() - 2.875).abs() < f64::EPSILON);
    }

    #[test]
    fn foods_until_next_speed_level_tracks_progress_within_current_level() {
        let bounds = GridSize {
            width: 200,
            height: 200,
        };
        let mut state = GameState::new_with_seed_speed_and_food(
            bounds,
            55,
            1,
            FoodDensity {
                foods_per: 1,
                cells_per: usize::MAX,
            },
        );
        // 8 foods eaten means snake length 10.
        // Level progression from start:
        // - 6 foods to reach level 2
        // - then 2 foods toward next threshold (7)
        // => 5 foods remaining to level 3.
        let segments = (0..10)
            .map(|i| Position { x: 150 - i, y: 10 })
            .collect::<Vec<_>>();
        state.snake = Snake::from_segments(segments, Direction::Right)
            .expect("snake segments should be valid");
        state.update_speed_level();

        assert_eq!(state.speed_level, 2);
        assert_eq!(state.foods_until_next_speed_level(), 5);
    }

    #[test]
    fn snapshot_round_trip_restores_core_state() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 20,
                height: 20,
            },
            123,
        );
        state.snake = Snake::new(Position { x: 4, y: 4 }, Direction::Right);
        state.foods = vec![Food::new(Position { x: 8, y: 4 })];
        state.tick();
        state.record_tick_duration(Duration::from_millis(200));

        let snapshot = state.to_snapshot_v1();
        let restored = GameState::from_snapshot_v1(snapshot).expect("snapshot should restore");

        assert_eq!(restored.score, state.score);
        assert_eq!(restored.tick_count, state.tick_count);
        assert_eq!(restored.bounds(), state.bounds());
        assert_eq!(restored.snake.head(), state.snake.head());
        assert_eq!(restored.status, GameStatus::Paused);
    }

    #[test]
    fn snapshot_rejects_food_overlapping_snake() {
        let mut state = GameState::new_with_seed(
            GridSize {
                width: 20,
                height: 20,
            },
            99,
        );
        state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
        let mut snapshot = state.to_snapshot_v1();
        snapshot.foods = vec![Food::new(state.snake.head())];

        let result = GameState::from_snapshot_v1(snapshot);
        assert!(matches!(
            result,
            Err(SnapshotRestoreError::FoodOverlapsSnake)
        ));
    }
}
