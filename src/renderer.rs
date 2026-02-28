use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::config::{GridSize, PLAY_AREA_MARGIN_X, PLAY_AREA_MARGIN_Y, Theme, glyphs};
use crate::game::{GameState, GameStatus, GlowEffect, GlowTrigger};
use crate::platform::Platform;
use crate::ui::hud::{HudInfo, render_hud};
use crate::ui::menu::{
    ThemeSelectView, render_game_over_menu, render_pause_menu, render_start_menu,
};

pub struct MenuUiState<'a> {
    pub start_selected_idx: usize,
    pub start_settings_open: bool,
    pub start_settings_selected_idx: usize,
    pub start_speed_level: u32,
    /// Whether the speed-adjust sub-mode is active (Up/Down changes speed value).
    pub start_speed_adjust_mode: bool,
    pub checkerboard_enabled: bool,
    pub pause_selected_idx: usize,
    pub game_over_selected_idx: usize,
    pub start_theme_select: Option<ThemeSelectView<'a>>,
    pub pause_theme_select: Option<ThemeSelectView<'a>>,
}

/// What occupies a single logical game cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    Empty,
    SnakeHead,
    /// Carries body segment index (excluding head/tail) for color banding.
    SnakeBody(usize),
    SnakeTail,
    Food,
    SuperFood,
}

/// Renders the full game frame from immutable state.
pub fn render(
    frame: &mut Frame<'_>,
    state: &GameState,
    platform: Platform,
    hud_info: HudInfo<'_>,
    menu_ui: MenuUiState<'_>,
) {
    let area = frame.area();

    let theme = hud_info.theme;
    frame.render_widget(
        Block::default().style(Style::new().bg(theme.terminal_bg)),
        area,
    );

    let play_area = render_hud(frame, area, state, platform, &hud_info);

    let gameplay_area = inset_play_area(play_area);
    frame.render_widget(
        Block::default().style(Style::new().bg(theme.field_bg)),
        gameplay_area,
    );
    render_play_area_hud_margin(frame, play_area, gameplay_area, theme);

    render_play_area(
        frame,
        gameplay_area,
        state,
        theme,
        menu_ui.checkerboard_enabled,
    );

    if state.is_start_screen() {
        render_start_menu(
            frame,
            play_area,
            hud_info.high_score,
            hud_info.theme,
            menu_ui.start_selected_idx,
            menu_ui.start_settings_open,
            menu_ui.start_settings_selected_idx,
            menu_ui.start_speed_level,
            menu_ui.start_speed_adjust_mode,
            menu_ui.checkerboard_enabled,
            menu_ui.start_theme_select,
        );
        return;
    }

    match state.status {
        GameStatus::Paused => render_pause_menu(
            frame,
            play_area,
            hud_info.theme,
            menu_ui.pause_selected_idx,
            menu_ui.pause_theme_select,
        ),
        GameStatus::GameOver => render_game_over_menu(
            frame,
            play_area,
            state.score,
            hud_info.game_over_reference_high_score,
            state.snake.len(),
            state.death_reason,
            state.elapsed_duration(),
            hud_info.theme,
            menu_ui.game_over_selected_idx,
        ),
        GameStatus::Victory => render_game_over_menu(
            frame,
            play_area,
            state.score,
            hud_info.game_over_reference_high_score,
            state.snake.len(),
            state.death_reason,
            state.elapsed_duration(),
            hud_info.theme,
            menu_ui.game_over_selected_idx,
        ),
        _ => {}
    }
}

fn inset_play_area(area: Rect) -> Rect {
    let horizontal_margin = PLAY_AREA_MARGIN_X.saturating_mul(2);
    let vertical_margin = PLAY_AREA_MARGIN_Y.saturating_mul(2);

    Rect {
        x: area.x.saturating_add(PLAY_AREA_MARGIN_X),
        y: area.y.saturating_add(PLAY_AREA_MARGIN_Y),
        width: area.width.saturating_sub(horizontal_margin),
        height: area.height.saturating_sub(vertical_margin),
    }
}

fn render_play_area_hud_margin(
    frame: &mut Frame<'_>,
    play_area: Rect,
    gameplay_area: Rect,
    theme: &Theme,
) {
    if gameplay_area.bottom() >= play_area.bottom() {
        return;
    }

    let y = gameplay_area.bottom();
    let style = Style::new().fg(theme.terminal_bg).bg(theme.field_bg);
    let half_upper = glyphs().half_upper;
    let buffer = frame.buffer_mut();

    for x in gameplay_area.x..gameplay_area.right() {
        buffer.set_string(x, y, half_upper, style);
    }
}

/// Returns the checkerboard background color for a given game-grid cell.
fn checker_bg(col: usize, game_row: usize, theme: &Theme) -> ratatui::style::Color {
    let tile_x = col / 6;
    let tile_y = game_row / 6;
    if (tile_x + tile_y).is_multiple_of(2) {
        theme.field_bg
    } else {
        theme.field_bg_alt
    }
}

/// Builds a color grid from game state and composites half-block row-pairs.
fn render_play_area(
    frame: &mut Frame<'_>,
    inner: Rect,
    state: &GameState,
    theme: &Theme,
    checkerboard_enabled: bool,
) {
    let bounds = state.bounds();
    let grid = build_cell_grid(state, bounds);
    let glow = state.active_glow();

    let buffer = frame.buffer_mut();
    let game_h = usize::from(bounds.height);
    // Each terminal row composites two game rows.
    let term_rows = game_h.div_ceil(2);

    for term_row in 0..term_rows {
        let top_game_row = term_row * 2;
        let bot_game_row = term_row * 2 + 1;
        let y = inner.y.saturating_add(term_row as u16);
        if y >= inner.bottom() {
            break;
        }

        for col in 0..usize::from(bounds.width) {
            let x = inner.x.saturating_add(col as u16);
            if x >= inner.right() {
                break;
            }

            let top_kind = grid[top_game_row * usize::from(bounds.width) + col];
            let bot_kind = if bot_game_row < game_h {
                grid[bot_game_row * usize::from(bounds.width) + col]
            } else {
                CellKind::Empty
            };

            let top_bg = if checkerboard_enabled {
                checker_bg(col, top_game_row, theme)
            } else {
                theme.field_bg
            };
            let bot_bg = if checkerboard_enabled {
                checker_bg(col, bot_game_row, theme)
            } else {
                theme.field_bg
            };
            let (glyph, fg, bg) =
                composite_half_block(top_kind, bot_kind, top_bg, bot_bg, theme, glow);
            buffer.set_string(x, y, glyph, Style::new().fg(fg).bg(bg));
        }
    }
}

/// Populates a flat grid of `CellKind` values indexed by `row * width + col`.
fn build_cell_grid(state: &GameState, bounds: GridSize) -> Vec<CellKind> {
    let w = usize::from(bounds.width);
    let h = usize::from(bounds.height);
    let mut grid = vec![CellKind::Empty; w * h];

    // Food
    for food in &state.foods {
        let fp = food.position;
        if fp.is_within_bounds(bounds) {
            let kind = if food.is_super() {
                CellKind::SuperFood
            } else {
                CellKind::Food
            };
            grid[fp.y as usize * w + fp.x as usize] = kind;
        }
    }

    // Snake segments — index 0 is the head.
    let snake_len = state.snake.len();
    for (idx, seg) in state.snake.segments().enumerate() {
        if !seg.is_within_bounds(bounds) {
            continue;
        }
        let kind = if idx == 0 {
            CellKind::SnakeHead
        } else if idx + 1 == snake_len {
            CellKind::SnakeTail
        } else {
            CellKind::SnakeBody(idx - 1)
        };
        grid[seg.y as usize * w + seg.x as usize] = kind;
    }

    grid
}

/// Returns (glyph, fg_color, bg_color) for a terminal cell compositing two game rows.
fn composite_half_block(
    top: CellKind,
    bot: CellKind,
    top_bg: ratatui::style::Color,
    bot_bg: ratatui::style::Color,
    theme: &Theme,
    glow: Option<&GlowEffect>,
) -> (&'static str, ratatui::style::Color, ratatui::style::Color) {
    let palette = glyphs();

    match (top, bot) {
        (CellKind::Empty, CellKind::Empty) => (palette.half_upper, top_bg, bot_bg),
        (top_kind, CellKind::Empty) => (
            palette.half_upper,
            cell_color(top_kind, theme, glow),
            bot_bg,
        ),
        (CellKind::Empty, bot_kind) => (
            palette.half_lower,
            cell_color(bot_kind, theme, glow),
            top_bg,
        ),
        (top_kind, bot_kind) => (
            palette.half_upper,
            cell_color(top_kind, theme, glow),
            cell_color(bot_kind, theme, glow),
        ),
    }
}

/// Maps a non-empty `CellKind` to its theme color, with optional glow blending.
///
/// Snake body uses alternating 3-segment bands: even bands use the base
/// `snake_body` color; odd bands have the red channel boosted by 10%.
/// When a glow effect is active, snake cells are blended toward the glow color.
fn cell_color(kind: CellKind, theme: &Theme, glow: Option<&GlowEffect>) -> ratatui::style::Color {
    match kind {
        CellKind::SnakeHead => {
            let base = theme.snake_head;
            if let Some(effect) = glow {
                let glow_color = glow_target_color(effect.trigger);
                lerp_color(base, glow_color, effect.intensity())
            } else {
                base
            }
        }
        CellKind::SnakeBody(idx) => {
            let band = idx / 3;
            let base = if band % 2 == 0 {
                theme.snake_body
            } else {
                redden_color(theme.snake_body, 0.8)
            };
            if let Some(effect) = glow {
                let glow_color = glow_target_color(effect.trigger);
                lerp_color(base, glow_color, effect.intensity())
            } else {
                base
            }
        }
        CellKind::SnakeTail => {
            let base = theme.snake_tail;
            if let Some(effect) = glow {
                let glow_color = glow_target_color(effect.trigger);
                lerp_color(base, glow_color, effect.intensity())
            } else {
                base
            }
        }
        CellKind::Food => theme.food,
        CellKind::SuperFood => theme.super_food,
        CellKind::Empty => theme.field_bg,
    }
}

/// Returns the glow target color for a given trigger type.
fn glow_target_color(trigger: GlowTrigger) -> ratatui::style::Color {
    use ratatui::style::Color;
    match trigger {
        GlowTrigger::SpeedLevelUp => Color::Rgb(200, 255, 255),
        GlowTrigger::SuperFoodEaten => Color::Rgb(255, 215, 0),
    }
}

/// Linearly interpolates between two `Rgb` colors at factor `t` (0.0–1.0).
/// Returns `from` unchanged when either color is a named (non-RGB) color.
fn lerp_color(
    from: ratatui::style::Color,
    to: ratatui::style::Color,
    t: f32,
) -> ratatui::style::Color {
    use ratatui::style::Color;
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let t = t.clamp(0.0, 1.0);
            let lerp = |a: u8, b: u8| -> u8 {
                (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8
            };
            Color::Rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
        }
        _ => from,
    }
}

/// Makes an `Rgb` color appear redder by reducing the green and blue channels
/// by `factor` (e.g. 0.9 = 10% reduction). Named colors are returned unchanged.
fn redden_color(color: ratatui::style::Color, factor: f32) -> ratatui::style::Color {
    use ratatui::style::Color;
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r,
            (f32::from(g) * factor).round() as u8,
            (f32::from(b) * factor).round() as u8,
        ),
        other => other,
    }
}
