use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::config::{
    GridSize, BORDER_HALF_BLOCK, GLYPH_FOOD, GLYPH_SNAKE_BODY, GLYPH_SNAKE_HEAD_DOWN,
    GLYPH_SNAKE_HEAD_LEFT, GLYPH_SNAKE_HEAD_RIGHT, GLYPH_SNAKE_HEAD_UP, GLYPH_SNAKE_TAIL,
};
use crate::game::{GameState, GameStatus};
use crate::input::Direction;
use crate::platform::Platform;
use crate::snake::Position;
use crate::ui::hud::{render_hud, HudInfo};
use crate::ui::menu::{render_game_over_menu, render_pause_menu, render_start_menu};

/// Renders the full game frame from immutable state.
pub fn render(frame: &mut Frame<'_>, state: &GameState, platform: Platform, hud_info: HudInfo) {
    let area = frame.area();
    let play_area = render_hud(frame, area, state, platform, &hud_info);

    let theme = hud_info.theme;
    let block = Block::bordered()
        .border_set(BORDER_HALF_BLOCK)
        .border_style(Style::new().fg(theme.border_fg).bg(theme.border_bg));

    let inner = block.inner(play_area);
    frame.render_widget(block, play_area);

    render_food(frame, inner, state, hud_info.theme);
    render_snake(frame, inner, state, hud_info.theme);

    if state.is_start_screen() {
        render_start_menu(frame, play_area, hud_info.high_score, hud_info.theme);
        return;
    }

    match state.status {
        GameStatus::Paused => render_pause_menu(frame, play_area, hud_info.theme),
        GameStatus::GameOver => render_game_over_menu(
            frame,
            play_area,
            state.score,
            hud_info.game_over_reference_high_score,
            state.death_reason,
            hud_info.theme,
        ),
        _ => {}
    }
}

fn render_food(
    frame: &mut Frame<'_>,
    inner: Rect,
    state: &GameState,
    theme: &crate::config::Theme,
) {
    let Some((x, y)) = logical_to_terminal(inner, state.bounds(), state.food.position) else {
        return;
    };

    let buffer = frame.buffer_mut();
    buffer.set_string(x, y, GLYPH_FOOD, Style::new().fg(theme.food));
}

fn render_snake(
    frame: &mut Frame<'_>,
    inner: Rect,
    state: &GameState,
    theme: &crate::config::Theme,
) {
    let head = state.snake.head();
    let tail = state.snake.segments().last().copied();

    let buffer = frame.buffer_mut();
    for segment in state.snake.segments() {
        let Some((x, y)) = logical_to_terminal(inner, state.bounds(), *segment) else {
            continue;
        };

        if *segment == head {
            let glyph = head_glyph(state.snake.direction());
            buffer.set_string(
                x,
                y,
                glyph,
                Style::new()
                    .fg(theme.snake_head)
                    .add_modifier(Modifier::BOLD),
            );
            continue;
        }

        if Some(*segment) == tail {
            buffer.set_string(
                x,
                y,
                GLYPH_SNAKE_TAIL,
                Style::new().fg(theme.snake_tail),
            );
            continue;
        }

        buffer.set_string(
            x,
            y,
            GLYPH_SNAKE_BODY,
            Style::new().fg(theme.snake_body),
        );
    }
}

fn head_glyph(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => GLYPH_SNAKE_HEAD_UP,
        Direction::Down => GLYPH_SNAKE_HEAD_DOWN,
        Direction::Left => GLYPH_SNAKE_HEAD_LEFT,
        Direction::Right => GLYPH_SNAKE_HEAD_RIGHT,
    }
}

fn logical_to_terminal(inner: Rect, bounds: GridSize, position: Position) -> Option<(u16, u16)> {
    if !position.is_within_bounds(bounds) {
        return None;
    }

    let x_offset = u16::try_from(position.x).ok()?;
    let y_offset = u16::try_from(position.y).ok()?;

    let x = inner.x.saturating_add(x_offset);
    let y = inner.y.saturating_add(y_offset);
    if x >= inner.right() || y >= inner.bottom() {
        return None;
    }

    Some((x, y))
}
