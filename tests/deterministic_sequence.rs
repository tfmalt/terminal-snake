use terminal_snake::config::GridSize;
use terminal_snake::food::Food;
use terminal_snake::game::{GameState, GameStatus};
use terminal_snake::input::{Direction, GameInput};
use terminal_snake::snake::{Position, Snake};

#[test]
fn stepwise_food_collection_and_wall_collision() {
    let mut state = GameState::new_with_seed(
        GridSize {
            width: 6,
            height: 4,
        },
        42,
    );

    state.set_base_speed_level(2);
    state.snake = Snake::new(Position { x: 1, y: 1 }, Direction::Right);
    state.foods = vec![Food::new(Position { x: 2, y: 1 })];

    state.tick();
    assert_eq!(state.status, GameStatus::Playing);
    // Base score (2) plus coverage bonus at 12.5% coverage: floor(2 * (1 + 1.25)) = 4.
    assert_eq!(state.score, 4);
    assert_eq!(state.snake.len(), 3);
    assert_eq!(state.snake.head(), Position { x: 2, y: 1 });

    state.apply_input(GameInput::Direction(Direction::Up));
    state.tick();
    assert_eq!(state.status, GameStatus::Playing);
    assert_eq!(state.snake.head(), Position { x: 2, y: 0 });

    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);
}

#[test]
fn self_collision_triggers_game_over() {
    // Build a coiled snake so the head immediately moves into its own body.
    // Shape: head at (2,2) going left, body spirals around.
    //   (3,2) ← (3,3) ← (2,3) ← (1,3) ← (1,2) = tail
    //     ↓
    //   (2,2) = head going Left
    //
    // Next move: head goes to (1,2) which is occupied by the tail.
    // But the tail moves, so we need the body to wrap tighter.
    //
    // Use a shape where the head moves directly into an interior body segment:
    //   head(2,2) going Down → next position is (2,3) which is body.
    let mut state = GameState::new_with_seed(
        GridSize {
            width: 6,
            height: 6,
        },
        123,
    );

    let segments = vec![
        Position { x: 2, y: 2 }, // head
        Position { x: 1, y: 2 },
        Position { x: 1, y: 3 },
        Position { x: 2, y: 3 }, // body occupies (2,3) — head's next position
        Position { x: 3, y: 3 },
        Position { x: 3, y: 2 },
    ];
    state.snake = Snake::from_segments(segments, Direction::Down).unwrap();
    state.foods.clear();

    // Head moves down to (2,3) which is occupied by body segment.
    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);
    assert_eq!(
        state.death_reason,
        Some(terminal_snake::game::DeathReason::SelfCollision)
    );
}

#[test]
fn speed_level_progression_through_food_collection() {
    let mut state = GameState::new_with_seed(
        GridSize {
            width: 40,
            height: 20,
        },
        999,
    );

    // Place the snake heading right with plenty of room.
    state.snake = Snake::new(Position { x: 1, y: 10 }, Direction::Right);
    assert_eq!(state.speed_level, 1);

    // Feed the snake enough food to level up.
    // Level 1→2 requires FOOD_PER_SPEED_LEVEL + level = 5 + 1 = 6 food items.
    for i in 0..6 {
        state.foods = vec![Food::new(Position { x: 2 + i, y: 10 })];
        state.tick();
        assert_eq!(state.status, GameStatus::Playing);
    }

    assert!(
        state.speed_level >= 2,
        "Expected speed level >= 2 after eating 6 food, got {}",
        state.speed_level,
    );
}

#[test]
fn pause_and_resume_preserves_state() {
    let mut state = GameState::new_with_seed(
        GridSize {
            width: 20,
            height: 20,
        },
        42,
    );

    state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
    state.foods = vec![Food::new(Position { x: 8, y: 5 })];

    // Play a couple ticks.
    state.tick();
    state.tick();
    let pos_before_pause = state.snake.head();
    let score_before = state.score;

    // Pause.
    state.apply_input(GameInput::Pause);
    assert_eq!(state.status, GameStatus::Paused);

    // Tick while paused should not advance.
    state.tick();
    assert_eq!(state.snake.head(), pos_before_pause);
    assert_eq!(state.score, score_before);

    // Resume.
    state.apply_input(GameInput::Pause);
    assert_eq!(state.status, GameStatus::Playing);

    // Tick should advance now.
    state.tick();
    assert_ne!(state.snake.head(), pos_before_pause);
}

#[test]
fn restart_resets_state() {
    let mut state = GameState::new_with_seed(
        GridSize {
            width: 20,
            height: 20,
        },
        42,
    );

    state.snake = Snake::new(Position { x: 5, y: 5 }, Direction::Right);
    state.foods = vec![Food::new(Position { x: 6, y: 5 })];

    // Eat food and accumulate score.
    state.tick();
    assert!(state.score > 0);
    assert!(state.snake.len() > 2);

    // Restart.
    let new_state = state.restart();
    assert_eq!(new_state.score, 0);
    assert_eq!(new_state.tick_count, 0);
    assert_eq!(new_state.snake.len(), 2);
    assert_eq!(new_state.status, GameStatus::Playing);
    assert_eq!(new_state.death_reason, None);
}

#[test]
fn wall_collision_in_all_four_directions() {
    let bounds = GridSize {
        width: 10,
        height: 10,
    };

    // Hit top wall.
    let mut state = GameState::new_with_seed(bounds, 1);
    state.snake = Snake::new(Position { x: 5, y: 0 }, Direction::Up);
    state.foods.clear();
    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);
    assert_eq!(
        state.death_reason,
        Some(terminal_snake::game::DeathReason::WallCollision)
    );

    // Hit bottom wall.
    let mut state = GameState::new_with_seed(bounds, 2);
    state.snake = Snake::new(Position { x: 5, y: 9 }, Direction::Down);
    state.foods.clear();
    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);

    // Hit left wall.
    let mut state = GameState::new_with_seed(bounds, 3);
    state.snake = Snake::new(Position { x: 0, y: 5 }, Direction::Left);
    state.foods.clear();
    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);

    // Hit right wall.
    let mut state = GameState::new_with_seed(bounds, 4);
    state.snake = Snake::new(Position { x: 9, y: 5 }, Direction::Right);
    state.foods.clear();
    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);
}

#[test]
fn snapshot_resume_continues_deterministically() {
    let bounds = GridSize {
        width: 16,
        height: 10,
    };

    let mut baseline = GameState::new_with_seed(bounds, 777);
    baseline.set_base_speed_level(3);
    baseline.tick();
    baseline.tick();

    let mut resumed = GameState::from_snapshot_v1(baseline.to_snapshot_v1())
        .expect("snapshot should restore successfully");
    resumed.status = GameStatus::Playing;

    // Apply the same inputs then step the same number of ticks.
    baseline.apply_input(GameInput::Direction(Direction::Down));
    resumed.apply_input(GameInput::Direction(Direction::Down));

    for _ in 0..8 {
        baseline.tick();
        resumed.tick();
    }

    assert_eq!(resumed.score, baseline.score);
    assert_eq!(resumed.tick_count, baseline.tick_count);
    assert_eq!(resumed.speed_level, baseline.speed_level);
    assert_eq!(resumed.status, baseline.status);
    assert_eq!(resumed.snake.head(), baseline.snake.head());
    assert_eq!(resumed.foods, baseline.foods);
}
