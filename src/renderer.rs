use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::config::{
    DEFAULT_TICK_INTERVAL_MS, GridSize, MIN_TICK_INTERVAL_MS, PLAY_AREA_MARGIN_X,
    PLAY_AREA_MARGIN_Y, Theme, glyphs,
};
use crate::game::{GameState, GameStatus, GlowEffect, GlowTrigger};
use crate::platform::Platform;
use crate::snake::Position;
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
    pub game_border_enabled: bool,
    pub play_area_too_small: bool,
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

#[derive(Debug, Clone, Copy)]
struct CellRender {
    kind: CellKind,
    bg: ratatui::style::Color,
    bg_flash_amount: f32,
    snake_body_flash_amount: f32,
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
    if menu_ui.game_border_enabled {
        render_play_area_border(frame, play_area, gameplay_area, theme);
    } else {
        render_play_area_hud_margin(frame, play_area, gameplay_area, theme);
    }

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
            menu_ui.play_area_too_small,
            menu_ui.start_selected_idx,
            menu_ui.start_settings_open,
            menu_ui.start_settings_selected_idx,
            menu_ui.start_speed_level,
            menu_ui.start_speed_adjust_mode,
            menu_ui.checkerboard_enabled,
            menu_ui.game_border_enabled,
            menu_ui.start_theme_select,
        );
        return;
    }

    match state.status {
        GameStatus::Paused => render_pause_menu(
            frame,
            play_area,
            hud_info.theme,
            menu_ui.play_area_too_small,
            menu_ui.pause_selected_idx,
            menu_ui.pause_theme_select,
        ),
        GameStatus::GameOver => render_game_over_menu(
            frame,
            play_area,
            state.score,
            hud_info.game_over_reference_high_score,
            state.snake.len(),
            state.play_area_coverage_percent(),
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
            state.play_area_coverage_percent(),
            state.death_reason,
            state.elapsed_duration(),
            hud_info.theme,
            menu_ui.game_over_selected_idx,
        ),
        _ => {}
    }
}

fn render_play_area_border(
    frame: &mut Frame<'_>,
    play_area: Rect,
    gameplay_area: Rect,
    theme: &Theme,
) {
    let style = Style::new().fg(theme.ui_bright).bg(theme.terminal_bg);
    let buffer = frame.buffer_mut();

    if gameplay_area.y > play_area.y {
        let top_y = gameplay_area.y - 1;
        for x in gameplay_area.x..gameplay_area.right() {
            buffer.set_string(x, top_y, "▁", style);
        }
    }

    if gameplay_area.bottom() < play_area.bottom() {
        let bottom_y = gameplay_area.bottom();
        for x in gameplay_area.x..gameplay_area.right() {
            buffer.set_string(x, bottom_y, "▔", style);
        }
    }

    if gameplay_area.x > play_area.x {
        let left_x = gameplay_area.x - 1;
        for y in gameplay_area.y..gameplay_area.bottom() {
            buffer.set_string(left_x, y, "▕", style);
        }
    }

    if gameplay_area.right() < play_area.right() {
        let right_x = gameplay_area.right();
        for y in gameplay_area.y..gameplay_area.bottom() {
            buffer.set_string(right_x, y, "▏", style);
        }
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
    let snake_cells = build_snake_cell_mask(state, bounds);
    let level_up_neighbor_flash = glow.and_then(level_up_neighbor_flash_amount).unwrap_or(0.0);
    let neighbor_flash_mask = if level_up_neighbor_flash > 0.0 {
        Some(build_snake_neighbor_mask(state, bounds, &snake_cells))
    } else {
        None
    };
    let super_food_ripple_flash = glow.and_then(super_food_ripple_flash_amount).unwrap_or(0.0);
    let super_food_ripple_center = super_food_ripple_center_position(state, glow);
    let super_food_ripple_center_idx = super_food_ripple_center.and_then(|center| {
        if center.is_within_bounds(bounds) {
            Some(center.y as usize * usize::from(bounds.width) + center.x as usize)
        } else {
            None
        }
    });
    let super_food_body_expansion_mask = if super_food_ripple_flash > 0.0 {
        build_super_food_body_expansion_mask(bounds, super_food_ripple_center)
    } else {
        None
    };
    let super_food_ripple_mask = if super_food_ripple_flash > 0.0 {
        build_super_food_ripple_mask(bounds, &snake_cells, super_food_ripple_center)
    } else {
        None
    };

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

            let mut top_kind = grid[top_game_row * usize::from(bounds.width) + col];
            let top_idx = top_game_row * usize::from(bounds.width) + col;
            let mut bot_kind = if bot_game_row < game_h {
                grid[bot_game_row * usize::from(bounds.width) + col]
            } else {
                CellKind::Empty
            };
            let bot_idx = bot_game_row * usize::from(bounds.width) + col;

            if super_food_body_expansion_mask
                .as_ref()
                .is_some_and(|mask| mask[top_idx])
                && matches!(top_kind, CellKind::Empty)
            {
                top_kind = CellKind::SnakeBody(0);
            }
            if bot_game_row < game_h
                && super_food_body_expansion_mask
                    .as_ref()
                    .is_some_and(|mask| mask[bot_idx])
                && matches!(bot_kind, CellKind::Empty)
            {
                bot_kind = CellKind::SnakeBody(0);
            }

            let top_neighbor_flash = neighbor_flash_mask
                .as_ref()
                .is_some_and(|mask| mask[top_idx]);
            let bot_neighbor_flash = bot_game_row < game_h
                && neighbor_flash_mask
                    .as_ref()
                    .is_some_and(|mask| mask[bot_idx]);
            let top_super_flash = super_food_ripple_mask
                .as_ref()
                .is_some_and(|mask| mask[top_idx]);
            let bot_super_flash = bot_game_row < game_h
                && super_food_ripple_mask
                    .as_ref()
                    .is_some_and(|mask| mask[bot_idx]);
            let top_flash_amount = if top_super_flash {
                super_food_ripple_flash
            } else if top_neighbor_flash {
                level_up_neighbor_flash
            } else {
                0.0
            };
            let bot_flash_amount = if bot_super_flash {
                super_food_ripple_flash
            } else if bot_neighbor_flash {
                level_up_neighbor_flash
            } else {
                0.0
            };
            let top_body_flash_amount = if matches!(top_kind, CellKind::SnakeBody(_))
                && super_food_ripple_center_idx == Some(top_idx)
            {
                super_food_ripple_flash
            } else {
                0.0
            };
            let bot_body_flash_amount = if bot_game_row < game_h
                && matches!(bot_kind, CellKind::SnakeBody(_))
                && super_food_ripple_center_idx == Some(bot_idx)
            {
                super_food_ripple_flash
            } else {
                0.0
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
            let top = CellRender {
                kind: top_kind,
                bg: top_bg,
                bg_flash_amount: top_flash_amount,
                snake_body_flash_amount: top_body_flash_amount,
            };
            let bot = CellRender {
                kind: bot_kind,
                bg: bot_bg,
                bg_flash_amount: bot_flash_amount,
                snake_body_flash_amount: bot_body_flash_amount,
            };
            let (glyph, fg, bg) = composite_half_block(top, bot, theme, glow);
            buffer.set_string(x, y, glyph, Style::new().fg(fg).bg(bg));
        }
    }
}

fn build_snake_cell_mask(state: &GameState, bounds: GridSize) -> Vec<bool> {
    let width = usize::from(bounds.width);
    let height = usize::from(bounds.height);
    let mut snake_cells = vec![false; width * height];

    for segment in state.snake.segments() {
        if segment.is_within_bounds(bounds) {
            let idx = segment.y as usize * width + segment.x as usize;
            snake_cells[idx] = true;
        }
    }

    snake_cells
}

fn build_snake_neighbor_mask(
    state: &GameState,
    bounds: GridSize,
    snake_cells: &[bool],
) -> Vec<bool> {
    let width = usize::from(bounds.width);
    let height = usize::from(bounds.height);
    let mut neighbors = vec![false; width * height];

    for segment in state.snake.segments() {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = segment.x + dx;
                let ny = segment.y + dy;
                if nx < 0
                    || ny < 0
                    || nx >= i32::from(bounds.width)
                    || ny >= i32::from(bounds.height)
                {
                    continue;
                }

                let idx = ny as usize * width + nx as usize;
                if !snake_cells[idx] {
                    neighbors[idx] = true;
                }
            }
        }
    }

    neighbors
}

fn build_super_food_ripple_mask(
    bounds: GridSize,
    snake_cells: &[bool],
    center: Option<Position>,
) -> Option<Vec<bool>> {
    let center = center?;

    let width = usize::from(bounds.width);
    let mut mask = vec![false; width * usize::from(bounds.height)];

    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }

            let nx = center.x + dx;
            let ny = center.y + dy;
            if nx < 0 || ny < 0 || nx >= i32::from(bounds.width) || ny >= i32::from(bounds.height) {
                continue;
            }

            let idx = ny as usize * width + nx as usize;
            if !snake_cells[idx] {
                mask[idx] = true;
            }
        }
    }

    Some(mask)
}

fn build_super_food_body_expansion_mask(
    bounds: GridSize,
    center: Option<Position>,
) -> Option<Vec<bool>> {
    let center = center?;
    let width = usize::from(bounds.width);
    let mut mask = vec![false; width * usize::from(bounds.height)];

    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = center.x + dx;
            let ny = center.y + dy;
            if nx < 0 || ny < 0 || nx >= i32::from(bounds.width) || ny >= i32::from(bounds.height) {
                continue;
            }

            let idx = ny as usize * width + nx as usize;
            mask[idx] = true;
        }
    }

    Some(mask)
}

fn super_food_ripple_center_position(
    state: &GameState,
    glow: Option<&GlowEffect>,
) -> Option<Position> {
    let effect = glow?;
    if effect.trigger != GlowTrigger::SuperFoodEaten {
        return None;
    }

    let tick_interval = tick_interval_for_speed(state.speed_level).as_secs_f32();
    if tick_interval <= 0.0 {
        return None;
    }

    let snake_cells_per_second = 1.0 / tick_interval;
    let ripple_speed = snake_cells_per_second * GlowEffect::SUPER_FOOD_RIPPLE_SPEED_MULTIPLIER;
    let segment_idx = (effect.elapsed().as_secs_f32() * ripple_speed).floor() as usize;
    let segment_idx = segment_idx.min(state.snake.len().saturating_sub(1));
    state.snake.segments().nth(segment_idx).copied()
}

fn tick_interval_for_speed(speed_level: u32) -> std::time::Duration {
    let speed_penalty_ms = u64::from(speed_level.saturating_sub(1)) * 10;
    let clamped_ms = DEFAULT_TICK_INTERVAL_MS
        .saturating_sub(speed_penalty_ms)
        .max(MIN_TICK_INTERVAL_MS);
    std::time::Duration::from_millis(clamped_ms)
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
    top: CellRender,
    bot: CellRender,
    theme: &Theme,
    glow: Option<&GlowEffect>,
) -> (&'static str, ratatui::style::Color, ratatui::style::Color) {
    let palette = glyphs();
    let top_bg = apply_neighbor_flash(top.bg, top.bg_flash_amount);
    let bot_bg = apply_neighbor_flash(bot.bg, bot.bg_flash_amount);

    match (top.kind, bot.kind) {
        (CellKind::Empty, CellKind::Empty) => (palette.half_upper, top_bg, bot_bg),
        (top_kind, CellKind::Empty) => (
            palette.half_upper,
            cell_color(top_kind, theme, glow, top.snake_body_flash_amount),
            bot_bg,
        ),
        (CellKind::Empty, bot_kind) => (
            palette.half_lower,
            cell_color(bot_kind, theme, glow, bot.snake_body_flash_amount),
            top_bg,
        ),
        (top_kind, bot_kind) => (
            palette.half_upper,
            cell_color(top_kind, theme, glow, top.snake_body_flash_amount),
            cell_color(bot_kind, theme, glow, bot.snake_body_flash_amount),
        ),
    }
}

/// Maps a non-empty `CellKind` to its theme color, with optional glow blending.
///
/// Snake body uses alternating 3-segment bands: even bands use the base
/// `snake_body` color; odd bands have the red channel boosted by 10%.
/// When a glow effect is active, snake cells are blended toward the glow color.
fn cell_color(
    kind: CellKind,
    theme: &Theme,
    glow: Option<&GlowEffect>,
    snake_body_flash_amount: f32,
) -> ratatui::style::Color {
    match kind {
        CellKind::SnakeHead => {
            let base = theme.snake_head;
            if let Some(effect) = glow {
                match effect.trigger {
                    GlowTrigger::SpeedLevelUp => {
                        let glow_color = glow_target_color(effect.trigger, theme);
                        lerp_color(base, glow_color, effect.intensity())
                    }
                    GlowTrigger::SuperFoodEaten => base,
                }
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
                match effect.trigger {
                    GlowTrigger::SpeedLevelUp => {
                        let glow_color = glow_target_color(effect.trigger, theme);
                        lerp_color(base, glow_color, effect.intensity())
                    }
                    GlowTrigger::SuperFoodEaten => {
                        apply_neighbor_flash(base, snake_body_flash_amount)
                    }
                }
            } else {
                base
            }
        }
        CellKind::SnakeTail => {
            let base = theme.snake_tail;
            if let Some(effect) = glow {
                match effect.trigger {
                    GlowTrigger::SpeedLevelUp => {
                        let glow_color = glow_target_color(effect.trigger, theme);
                        lerp_color(base, glow_color, effect.intensity())
                    }
                    GlowTrigger::SuperFoodEaten => base,
                }
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
fn glow_target_color(trigger: GlowTrigger, theme: &Theme) -> ratatui::style::Color {
    use ratatui::style::Color;
    match trigger {
        GlowTrigger::SpeedLevelUp => brighten_color(theme.snake_body, 0.3),
        GlowTrigger::SuperFoodEaten => Color::Rgb(255, 215, 0),
    }
}

fn level_up_neighbor_flash_amount(effect: &GlowEffect) -> Option<f32> {
    if effect.trigger != GlowTrigger::SpeedLevelUp {
        return None;
    }

    let t = effect.progress();
    Some(0.5 * (1.0 - ease_out_cubic(t)))
}

fn super_food_ripple_flash_amount(effect: &GlowEffect) -> Option<f32> {
    if effect.trigger != GlowTrigger::SuperFoodEaten {
        return None;
    }

    Some(0.3)
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn apply_neighbor_flash(color: ratatui::style::Color, amount: f32) -> ratatui::style::Color {
    if amount <= 0.0 {
        return color;
    }

    brighten_color(color, amount)
}

fn brighten_color(color: ratatui::style::Color, amount: f32) -> ratatui::style::Color {
    use ratatui::style::Color;

    let Some((r, g, b)) = color_to_rgb(color) else {
        return color;
    };

    let amount = amount.clamp(0.0, 1.0);
    let brighten_channel = |channel: u8| -> u8 {
        let remaining = 255.0 - f32::from(channel);
        (f32::from(channel) + (remaining * amount)).round() as u8
    };

    Color::Rgb(
        brighten_channel(r),
        brighten_channel(g),
        brighten_channel(b),
    )
}

fn color_to_rgb(color: ratatui::style::Color) -> Option<(u8, u8, u8)> {
    use ratatui::style::Color;

    let rgb = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (229, 229, 229),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
        Color::Indexed(index) => xterm_index_to_rgb(index),
        Color::Reset => return None,
    };

    Some(rgb)
}

fn xterm_index_to_rgb(index: u8) -> (u8, u8, u8) {
    if index < 16 {
        const ANSI16: [(u8, u8, u8); 16] = [
            (0, 0, 0),
            (128, 0, 0),
            (0, 128, 0),
            (128, 128, 0),
            (0, 0, 128),
            (128, 0, 128),
            (0, 128, 128),
            (192, 192, 192),
            (128, 128, 128),
            (255, 0, 0),
            (0, 255, 0),
            (255, 255, 0),
            (0, 0, 255),
            (255, 0, 255),
            (0, 255, 255),
            (255, 255, 255),
        ];
        return ANSI16[index as usize];
    }

    if index <= 231 {
        let i = index - 16;
        let r = i / 36;
        let g = (i % 36) / 6;
        let b = i % 6;

        let level = |v: u8| -> u8 { if v == 0 { 0 } else { 55 + v * 40 } };

        return (level(r), level(g), level(b));
    }

    let gray = 8 + (index - 232) * 10;
    (gray, gray, gray)
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
