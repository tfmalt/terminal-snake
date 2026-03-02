use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::config::{
    CHECKER_TILE_SIZE, DEFAULT_TICK_INTERVAL_MS, GLYPH_BORDER_BOTTOM, GLYPH_BORDER_LEFT,
    GLYPH_BORDER_RIGHT, GLYPH_BORDER_TOP, GridSize, MIN_TICK_INTERVAL_MS, PLAY_AREA_MARGIN_X,
    PLAY_AREA_MARGIN_Y, Theme, glyphs,
};
use crate::game::{GameState, GameStatus, GlowEffect, GlowTrigger};
use crate::platform::Platform;
use crate::snake::Position;
use crate::ui::colors::{blend_color, brighten_color, ease_out_cubic, redden_color};
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
    pub has_active_session: bool,
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
    super_overlay_amount: f32,
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
            menu_ui.has_active_session,
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
            buffer.set_string(x, top_y, GLYPH_BORDER_TOP, style);
        }
    }

    if gameplay_area.bottom() < play_area.bottom() {
        let bottom_y = gameplay_area.bottom();
        for x in gameplay_area.x..gameplay_area.right() {
            buffer.set_string(x, bottom_y, GLYPH_BORDER_BOTTOM, style);
        }
    }

    if gameplay_area.x > play_area.x {
        let left_x = gameplay_area.x - 1;
        for y in gameplay_area.y..gameplay_area.bottom() {
            buffer.set_string(left_x, y, GLYPH_BORDER_LEFT, style);
        }
    }

    if gameplay_area.right() < play_area.right() {
        let right_x = gameplay_area.right();
        for y in gameplay_area.y..gameplay_area.bottom() {
            buffer.set_string(right_x, y, GLYPH_BORDER_RIGHT, style);
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
    let tile_x = col / CHECKER_TILE_SIZE;
    let tile_y = game_row / CHECKER_TILE_SIZE;
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
    let glow_trigger = glow.map(|effect| effect.trigger);
    let snake_cells = build_snake_cell_mask(state, bounds);
    let glow_flash = glow.and_then(glow_ripple_flash_amount).unwrap_or(0.0);
    let super_food_ripple_segment_pos = super_food_ripple_segment_position(state, glow);
    let super_food_ripple_centers = super_food_ripple_centers(state, glow);
    let super_food_ripple_size_scale =
        super_food_ripple_size_scale(state, super_food_ripple_segment_pos);
    let glow_overlay_field = if glow_flash > 0.0 {
        match glow_trigger {
            Some(GlowTrigger::SuperFoodEaten) => build_super_food_ripple_field(
                bounds,
                &snake_cells,
                super_food_ripple_centers.as_deref().unwrap_or(&[]),
                super_food_ripple_size_scale,
            ),
            Some(GlowTrigger::SpeedLevelUp) => {
                Some(build_level_up_aura_field(state, bounds, &snake_cells))
            }
            None => None,
        }
    } else {
        None
    };
    let glow_snake_pulse_field = if glow_flash > 0.0 {
        match glow_trigger {
            Some(GlowTrigger::SuperFoodEaten) => {
                build_super_food_snake_pulse_field(state, bounds, super_food_ripple_segment_pos)
            }
            Some(GlowTrigger::SpeedLevelUp) => build_level_up_snake_pulse_field(state, bounds),
            None => None,
        }
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

            let top_kind = grid[top_game_row * usize::from(bounds.width) + col];
            let top_idx = top_game_row * usize::from(bounds.width) + col;
            let bot_kind = if bot_game_row < game_h {
                grid[bot_game_row * usize::from(bounds.width) + col]
            } else {
                CellKind::Empty
            };
            let bot_idx = bot_game_row * usize::from(bounds.width) + col;

            let top_super_overlay_amount = glow_overlay_field
                .as_ref()
                .map_or(0.0, |field| field[top_idx] * glow_flash);
            let bot_super_overlay_amount = if bot_game_row < game_h {
                glow_overlay_field
                    .as_ref()
                    .map_or(0.0, |field| field[bot_idx] * glow_flash)
            } else {
                0.0
            };
            let top_flash_amount = (top_super_overlay_amount * 0.28).clamp(0.0, 1.0);
            let bot_flash_amount = (bot_super_overlay_amount * 0.28).clamp(0.0, 1.0);
            let top_body_flash_amount = if matches!(
                top_kind,
                CellKind::SnakeHead | CellKind::SnakeBody(_) | CellKind::SnakeTail
            ) {
                glow_snake_pulse_field
                    .as_ref()
                    .map_or(0.0, |field| field[top_idx] * glow_flash)
            } else {
                0.0
            };
            let bot_body_flash_amount = if bot_game_row < game_h
                && matches!(
                    bot_kind,
                    CellKind::SnakeHead | CellKind::SnakeBody(_) | CellKind::SnakeTail
                ) {
                glow_snake_pulse_field
                    .as_ref()
                    .map_or(0.0, |field| field[bot_idx] * glow_flash)
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
                super_overlay_amount: top_super_overlay_amount,
                snake_body_flash_amount: top_body_flash_amount,
            };
            let bot = CellRender {
                kind: bot_kind,
                bg: bot_bg,
                bg_flash_amount: bot_flash_amount,
                super_overlay_amount: bot_super_overlay_amount,
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

fn build_super_food_ripple_field(
    bounds: GridSize,
    snake_cells: &[bool],
    centers: &[(Position, f32)],
    size_scale: f32,
) -> Option<Vec<f32>> {
    if centers.is_empty() {
        return None;
    }

    let width = usize::from(bounds.width);
    let mut field = vec![0.0_f32; width * usize::from(bounds.height)];

    for (center, weight) in centers {
        apply_super_food_kernel(
            &mut field,
            bounds,
            snake_cells,
            *center,
            *weight,
            size_scale,
        );
    }

    Some(field)
}

fn apply_super_food_kernel(
    field: &mut [f32],
    bounds: GridSize,
    snake_cells: &[bool],
    center: Position,
    weight: f32,
    size_scale: f32,
) {
    if weight <= 0.0 {
        return;
    }

    let width = usize::from(bounds.width);
    let size_scale = size_scale.clamp(0.35, 1.0);

    for dy in -3..=3 {
        for dx in -3..=3 {
            let distance = ((dx * dx + dy * dy) as f32).sqrt();
            let scaled_distance = distance / size_scale;
            if scaled_distance > 3.2 {
                continue;
            }

            let ring = (1.0 - (scaled_distance - 1.6).abs() / 1.6).clamp(0.0, 1.0);
            let halo = (1.0 - scaled_distance / 3.2).clamp(0.0, 1.0) * 0.35;
            let core = (1.0 - scaled_distance / 0.95).clamp(0.0, 1.0) * 0.4;
            let intensity = (ring * 0.6 + halo + core) * weight;
            let intensity = intensity.clamp(0.0, 1.0);
            if intensity <= 0.0 {
                continue;
            }

            let nx = center.x + dx;
            let ny = center.y + dy;
            if nx < 0 || ny < 0 || nx >= i32::from(bounds.width) || ny >= i32::from(bounds.height) {
                continue;
            }

            let idx = ny as usize * width + nx as usize;
            if !snake_cells[idx] {
                field[idx] = field[idx].max(intensity);
            }
        }
    }
}

fn build_level_up_aura_field(
    state: &GameState,
    bounds: GridSize,
    snake_cells: &[bool],
) -> Vec<f32> {
    let width = usize::from(bounds.width);
    let mut field = vec![0.0_f32; width * usize::from(bounds.height)];

    for segment in state.snake.segments() {
        for dy in -2..=2 {
            for dx in -2..=2 {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                if distance > 2.35 {
                    continue;
                }

                let halo = (1.0 - distance / 2.35).clamp(0.0, 1.0).powf(1.8);
                let core = (1.0 - distance / 0.9).clamp(0.0, 1.0) * 0.9;
                let intensity = (halo * 0.35 + core).clamp(0.0, 1.0);
                if intensity <= 0.0 {
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
                    field[idx] = field[idx].max(intensity);
                }
            }
        }
    }

    field
}

fn super_food_ripple_centers(
    state: &GameState,
    glow: Option<&GlowEffect>,
) -> Option<Vec<(Position, f32)>> {
    let segment_position = super_food_ripple_segment_position(state, glow)?;
    let segments = state.snake.segments().copied().collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }

    let lower_idx = segment_position.floor() as usize;
    let upper_idx = (lower_idx + 1).min(segments.len().saturating_sub(1));
    let fraction = (segment_position - lower_idx as f32).clamp(0.0, 1.0);

    let mut centers = Vec::with_capacity(2);
    let lower = *segments.get(lower_idx)?;
    centers.push((lower, 1.0 - fraction));

    if upper_idx != lower_idx {
        let upper = *segments.get(upper_idx)?;
        centers.push((upper, fraction));
    }

    Some(centers)
}

fn super_food_ripple_size_scale(state: &GameState, segment_position: Option<f32>) -> f32 {
    let Some(segment_position) = segment_position else {
        return 1.0;
    };

    let max_idx = state.snake.len().saturating_sub(1) as f32;
    if max_idx <= 0.0 {
        return 1.0;
    }

    let travel_progress = (segment_position / max_idx).clamp(0.0, 1.0);
    (1.0 - 0.55 * ease_out_cubic(travel_progress)).clamp(0.35, 1.0)
}

fn build_super_food_snake_pulse_field(
    state: &GameState,
    bounds: GridSize,
    segment_position: Option<f32>,
) -> Option<Vec<f32>> {
    let segment_position = segment_position?;

    let width = usize::from(bounds.width);
    let mut field = vec![0.0_f32; width * usize::from(bounds.height)];

    for (idx, segment) in state.snake.segments().enumerate() {
        if !segment.is_within_bounds(bounds) {
            continue;
        }

        let distance = ((idx as f32) - segment_position).abs();
        let core = (1.0 - distance / 0.9).clamp(0.0, 1.0);
        let halo = (1.0 - distance / 2.75).clamp(0.0, 1.0) * 0.7;
        let intensity = (core * 0.75 + halo).clamp(0.0, 1.0);
        if intensity <= 0.0 {
            continue;
        }

        let cell_idx = segment.y as usize * width + segment.x as usize;
        field[cell_idx] = field[cell_idx].max(intensity);
    }

    Some(field)
}

fn build_level_up_snake_pulse_field(state: &GameState, bounds: GridSize) -> Option<Vec<f32>> {
    let width = usize::from(bounds.width);
    let mut field = vec![0.0_f32; width * usize::from(bounds.height)];

    let snake_len = state.snake.len();
    if snake_len == 0 {
        return None;
    }

    let denom = snake_len.saturating_sub(1) as f32;
    for (idx, segment) in state.snake.segments().enumerate() {
        if !segment.is_within_bounds(bounds) {
            continue;
        }

        let progress = if denom > 0.0 { idx as f32 / denom } else { 0.0 };
        let edge_distance = ((progress - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        let intensity = 1.0 - edge_distance * 0.18;
        let cell_idx = segment.y as usize * width + segment.x as usize;
        field[cell_idx] = field[cell_idx].max(intensity);
    }

    Some(field)
}

fn super_food_ripple_segment_position(state: &GameState, glow: Option<&GlowEffect>) -> Option<f32> {
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
    let max_idx = state.snake.len().saturating_sub(1) as f32;
    Some((effect.elapsed().as_secs_f32() * ripple_speed).clamp(0.0, max_idx))
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
    let top_overlay = top.super_overlay_amount.clamp(0.0, 1.0);
    let bot_overlay = bot.super_overlay_amount.clamp(0.0, 1.0);
    let top_bg_with_overlay = if top_overlay > 0.0 {
        super_overlay_color(top_bg, top_overlay, glow, theme)
    } else {
        top_bg
    };
    let bot_bg_with_overlay = if bot_overlay > 0.0 {
        super_overlay_color(bot_bg, bot_overlay, glow, theme)
    } else {
        bot_bg
    };

    match (top.kind, bot.kind) {
        (CellKind::Empty, CellKind::Empty) => {
            (palette.half_upper, top_bg_with_overlay, bot_bg_with_overlay)
        }
        (top_kind, CellKind::Empty) => (
            palette.half_upper,
            cell_color(top_kind, theme, glow, top.snake_body_flash_amount),
            bot_bg_with_overlay,
        ),
        (CellKind::Empty, bot_kind) => (
            palette.half_lower,
            cell_color(bot_kind, theme, glow, bot.snake_body_flash_amount),
            top_bg_with_overlay,
        ),
        (top_kind, bot_kind) => (
            palette.half_upper,
            cell_color(top_kind, theme, glow, top.snake_body_flash_amount),
            cell_color(bot_kind, theme, glow, bot.snake_body_flash_amount),
        ),
    }
}

fn super_overlay_color(
    base: ratatui::style::Color,
    intensity: f32,
    glow: Option<&GlowEffect>,
    theme: &Theme,
) -> ratatui::style::Color {
    let target = if let Some(effect) = glow {
        glow_target_color(effect.trigger, theme)
    } else {
        brighten_color(theme.super_food, 0.25)
    };
    blend_color(base, target, (0.25 + intensity * 0.55).clamp(0.0, 1.0))
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
            if glow.is_some() {
                apply_neighbor_flash(base, snake_body_flash_amount * 0.65)
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
            if glow.is_some() {
                apply_neighbor_flash(base, snake_body_flash_amount)
            } else {
                base
            }
        }
        CellKind::SnakeTail => {
            let base = theme.snake_tail;
            if glow.is_some() {
                apply_neighbor_flash(base, snake_body_flash_amount * 0.8)
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
    match trigger {
        GlowTrigger::SpeedLevelUp => brighten_color(theme.snake_body, 0.3),
        GlowTrigger::SuperFoodEaten => brighten_color(theme.super_food, 0.35),
    }
}

fn glow_ripple_flash_amount(effect: &GlowEffect) -> Option<f32> {
    if !matches!(
        effect.trigger,
        GlowTrigger::SuperFoodEaten | GlowTrigger::SpeedLevelUp
    ) {
        return None;
    }

    let t = effect.progress().clamp(0.0, 1.0);
    let intensity = match effect.trigger {
        GlowTrigger::SuperFoodEaten => {
            if t < 0.1 {
                0.45 + 0.45 * ease_out_cubic(t / 0.1)
            } else {
                let travel_t = ((t - 0.1) / 0.9).clamp(0.0, 1.0);
                0.05 + 0.75 * (1.0 - ease_out_cubic(travel_t))
            }
        }
        GlowTrigger::SpeedLevelUp => {
            if t < 0.06 {
                0.55 + 0.4 * ease_out_cubic(t / 0.06)
            } else {
                let fade_t = ((t - 0.06) / 0.3).clamp(0.0, 1.0);
                0.95 * (1.0 - ease_out_cubic(fade_t))
            }
        }
    };

    Some(intensity)
}

fn apply_neighbor_flash(color: ratatui::style::Color, amount: f32) -> ratatui::style::Color {
    if amount <= 0.0 {
        return color;
    }

    brighten_color(color, amount)
}

#[cfg(test)]
mod tests {
    use super::{GridSize, build_super_food_ripple_field};
    use crate::snake::Position;

    #[test]
    fn super_food_ripple_field_generates_falloff_values() {
        let bounds = GridSize {
            width: 7,
            height: 7,
        };
        let width = usize::from(bounds.width);
        let mut snake_cells = vec![false; width * usize::from(bounds.height)];

        let center = Position { x: 3, y: 3 };
        let center_idx = center.y as usize * width + center.x as usize;
        snake_cells[center_idx] = true;

        let field = build_super_food_ripple_field(bounds, &snake_cells, &[(center, 1.0)], 1.0)
            .expect("ripple field should be created when center exists");

        let near_idx = 3 * width + 4;
        let far_idx = 0;

        assert!(field[near_idx] > 0.0);
        assert_eq!(field[center_idx], 0.0);
        assert_eq!(field[far_idx], 0.0);
    }

    #[test]
    fn super_food_ripple_field_blends_multiple_centers() {
        let bounds = GridSize {
            width: 7,
            height: 7,
        };
        let width = usize::from(bounds.width);
        let snake_cells = vec![false; width * usize::from(bounds.height)];

        let left = Position { x: 2, y: 3 };
        let right = Position { x: 4, y: 3 };

        let field =
            build_super_food_ripple_field(bounds, &snake_cells, &[(left, 0.4), (right, 0.6)], 1.0)
                .expect("ripple field should be created when at least one center exists");

        let left_idx = left.y as usize * width + left.x as usize;
        let right_idx = right.y as usize * width + right.x as usize;

        assert!(field[left_idx] > 0.0);
        assert!(field[right_idx] > 0.0);
    }

    #[test]
    fn super_food_ripple_field_shrinks_with_size_scale() {
        let bounds = GridSize {
            width: 9,
            height: 9,
        };
        let width = usize::from(bounds.width);
        let snake_cells = vec![false; width * usize::from(bounds.height)];
        let center = Position { x: 4, y: 4 };

        let full = build_super_food_ripple_field(bounds, &snake_cells, &[(center, 1.0)], 1.0)
            .expect("full-scale ripple field should be created");
        let shrunk = build_super_food_ripple_field(bounds, &snake_cells, &[(center, 1.0)], 0.45)
            .expect("shrunk ripple field should be created");

        let probe_idx = 4 * width + 6;
        assert!(full[probe_idx] > shrunk[probe_idx]);
    }
}
