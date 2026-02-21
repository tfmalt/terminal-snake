use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::config::{
    GridSize, GLYPH_BORDER_BOTTOM_LEFT, GLYPH_BORDER_BOTTOM_RIGHT, GLYPH_BORDER_HORIZONTAL,
    GLYPH_BORDER_TOP_LEFT, GLYPH_BORDER_TOP_RIGHT, GLYPH_BORDER_VERTICAL, GLYPH_FOOD,
    GLYPH_FOOD_BONUS, GLYPH_SNAKE_BODY, GLYPH_SNAKE_HEAD_DOWN, GLYPH_SNAKE_HEAD_LEFT,
    GLYPH_SNAKE_HEAD_RIGHT, GLYPH_SNAKE_HEAD_UP, GLYPH_SNAKE_TAIL,
};
use crate::food::FoodKind;
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

    let block = Block::bordered()
        .title(status_title(state.status, platform))
        .border_set(border::Set {
            top_left: GLYPH_BORDER_TOP_LEFT,
            top_right: GLYPH_BORDER_TOP_RIGHT,
            bottom_left: GLYPH_BORDER_BOTTOM_LEFT,
            bottom_right: GLYPH_BORDER_BOTTOM_RIGHT,
            vertical_left: GLYPH_BORDER_VERTICAL,
            vertical_right: GLYPH_BORDER_VERTICAL,
            horizontal_top: GLYPH_BORDER_HORIZONTAL,
            horizontal_bottom: GLYPH_BORDER_HORIZONTAL,
        });

    let inner = block.inner(play_area);
    frame.render_widget(block, play_area);

    render_food(frame, inner, state, hud_info.monochrome);
    render_snake(frame, inner, state, hud_info.monochrome);

    if state.is_start_screen() {
        render_start_menu(frame, play_area, hud_info.high_score);
        return;
    }

    match state.status {
        GameStatus::Paused => render_pause_menu(frame, play_area),
        GameStatus::GameOver => render_game_over_menu(
            frame,
            play_area,
            state.score,
            hud_info.game_over_reference_high_score,
            state.death_reason,
        ),
        _ => {}
    }
}

fn status_title(status: GameStatus, platform: Platform) -> &'static str {
    if status == GameStatus::Paused {
        return " paused ";
    }
    if status == GameStatus::GameOver {
        return " game over ";
    }
    if status == GameStatus::Victory {
        return " victory ";
    }
    if platform.is_wsl() {
        " snake (wsl) "
    } else {
        " snake "
    }
}

fn render_food(frame: &mut Frame<'_>, inner: Rect, state: &GameState, monochrome: bool) {
    let Some((x, y)) = logical_to_terminal(inner, state.bounds(), state.food.position) else {
        return;
    };

    let (glyph, style) = match state.food.kind {
        FoodKind::Normal => (GLYPH_FOOD, style_with_color(monochrome, Color::Red, false)),
        FoodKind::Bonus { .. } => (
            GLYPH_FOOD_BONUS,
            style_with_color(monochrome, Color::Yellow, true),
        ),
    };

    let buffer = frame.buffer_mut();
    buffer.set_string(x, y, glyph, style);
}

fn render_snake(frame: &mut Frame<'_>, inner: Rect, state: &GameState, monochrome: bool) {
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
                style_with_color(monochrome, Color::Green, true),
            );
            continue;
        }

        if Some(*segment) == tail {
            buffer.set_string(
                x,
                y,
                GLYPH_SNAKE_TAIL,
                style_with_color(monochrome, Color::DarkGray, false),
            );
            continue;
        }

        buffer.set_string(
            x,
            y,
            GLYPH_SNAKE_BODY,
            style_with_color(monochrome, Color::Green, false),
        );
    }
}

fn style_with_color(monochrome: bool, color: Color, bold: bool) -> Style {
    let mut style = Style::default();
    if !monochrome {
        style = style.fg(color);
    }
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
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
