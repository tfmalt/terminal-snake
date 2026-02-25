use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::config::{GridSize, PLAY_AREA_MARGIN_X, PLAY_AREA_MARGIN_Y, Theme, glyphs};
use crate::game::{GameState, GameStatus};
use crate::platform::Platform;
use crate::ui::hud::{HudInfo, render_hud};
use crate::ui::menu::{
    ThemeSelectView, render_game_over_menu, render_pause_menu, render_start_menu,
};

pub struct MenuUiState<'a> {
    pub start_selected_idx: usize,
    pub pause_selected_idx: usize,
    pub game_over_selected_idx: usize,
    pub start_theme_select: Option<ThemeSelectView<'a>>,
    pub pause_theme_select: Option<ThemeSelectView<'a>>,
}

/// What occupies a single logical game cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    Empty,
    /// Carries the segment index (0 = head) for color banding.
    Snake(usize),
    Food,
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

    render_play_area(frame, gameplay_area, state, theme);

    if state.is_start_screen() {
        render_start_menu(
            frame,
            play_area,
            hud_info.high_score,
            hud_info.controller_detected,
            hud_info.theme,
            menu_ui.start_selected_idx,
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
            state.death_reason,
            state.elapsed_duration(),
            hud_info.controller_detected,
            hud_info.theme,
            menu_ui.game_over_selected_idx,
        ),
        GameStatus::Victory => render_game_over_menu(
            frame,
            play_area,
            state.score,
            hud_info.game_over_reference_high_score,
            state.death_reason,
            state.elapsed_duration(),
            hud_info.controller_detected,
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

/// Builds a color grid from game state and composites half-block row-pairs.
fn render_play_area(frame: &mut Frame<'_>, inner: Rect, state: &GameState, theme: &Theme) {
    let bounds = state.bounds();
    let grid = build_cell_grid(state, bounds);

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

            let (glyph, fg, bg) = composite_half_block(top_kind, bot_kind, theme);
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
            grid[fp.y as usize * w + fp.x as usize] = CellKind::Food;
        }
    }

    // Snake segments — index 0 is the head.
    for (idx, seg) in state.snake.segments().enumerate() {
        if !seg.is_within_bounds(bounds) {
            continue;
        }
        grid[seg.y as usize * w + seg.x as usize] = CellKind::Snake(idx);
    }

    grid
}

/// Returns (glyph, fg_color, bg_color) for a terminal cell compositing two game rows.
fn composite_half_block(
    top: CellKind,
    bot: CellKind,
    theme: &Theme,
) -> (&'static str, ratatui::style::Color, ratatui::style::Color) {
    let bg = theme.field_bg;
    let palette = glyphs();

    match (top, bot) {
        (CellKind::Empty, CellKind::Empty) => (" ", bg, bg),
        (top_kind, CellKind::Empty) => {
            // Upper half-block: fg = top color, bg = empty
            (palette.half_upper, cell_color(top_kind, theme), bg)
        }
        (CellKind::Empty, bot_kind) => {
            // Lower half-block: fg = bottom color, bg = empty
            (palette.half_lower, cell_color(bot_kind, theme), bg)
        }
        (top_kind, bot_kind) => {
            // Upper half-block: fg = top color, bg = bottom color
            (
                palette.half_upper,
                cell_color(top_kind, theme),
                cell_color(bot_kind, theme),
            )
        }
    }
}

/// Maps a non-empty `CellKind` to its theme color.
///
/// Snake body uses alternating 3-segment bands: even bands use the base
/// `snake_body` color; odd bands have the red channel boosted by 10%.
fn cell_color(kind: CellKind, theme: &Theme) -> ratatui::style::Color {
    match kind {
        CellKind::Snake(idx) => {
            let band = idx / 3;
            if band % 2 == 0 {
                theme.snake_body
            } else {
                redden_color(theme.snake_body, 0.8)
            }
        }
        CellKind::Food => theme.food,
        CellKind::Empty => theme.field_bg,
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
