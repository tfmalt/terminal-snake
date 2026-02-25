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
    assert_eq!(state.score, 2);
    assert_eq!(state.snake.len(), 3);
    assert_eq!(state.snake.head(), Position { x: 2, y: 1 });

    state.apply_input(GameInput::Direction(Direction::Up));
    state.tick();
    assert_eq!(state.status, GameStatus::Playing);
    assert_eq!(state.snake.head(), Position { x: 2, y: 0 });

    state.tick();
    assert_eq!(state.status, GameStatus::GameOver);
}
