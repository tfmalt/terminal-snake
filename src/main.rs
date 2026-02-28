use std::io;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::layout::Size;
use terminal_snake::config::{
    DEFAULT_TICK_INTERVAL_MS, GlyphMode, GridSize, HUD_BOTTOM_MARGIN_Y, MAX_START_SPEED_LEVEL,
    MIN_START_SPEED_LEVEL, MIN_TICK_INTERVAL_MS, PLAY_AREA_MARGIN_X, PLAY_AREA_MARGIN_Y,
    configure_glyphs,
};
use terminal_snake::game::{GameState, GameStatus};
use terminal_snake::input::{Direction, GameInput, InputHandler};
use terminal_snake::platform::Platform;
use terminal_snake::renderer::{self, MenuUiState};
use terminal_snake::score::{
    load_high_score, load_theme_selection, save_high_score, save_theme_selection,
};
use terminal_snake::terminal_runtime::{AppTerminal, TerminalSession};
use terminal_snake::theme::ThemeCatalog;
use terminal_snake::ui::hud::HudInfo;
use terminal_snake::ui::menu::ThemeSelectView;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ThemeSelectionMode {
    StartMenu,
    PauseMenu,
}

const START_MENU_ITEM_COUNT: usize = 3;
const START_MENU_START_IDX: usize = 0;
const START_MENU_SETTINGS_IDX: usize = 1;
const START_MENU_QUIT_IDX: usize = 2;

const START_SETTINGS_ITEM_COUNT: usize = 4;
const START_SETTINGS_SPEED_IDX: usize = 0;
const START_SETTINGS_THEME_IDX: usize = 1;
const START_SETTINGS_GRID_IDX: usize = 2;
const START_SETTINGS_BACK_IDX: usize = 3;

#[derive(Debug, Parser)]
struct Cli {
    /// Starting speed level.
    #[arg(long, default_value_t = 1)]
    speed: u32,

    /// Deprecated compatibility flag; controller support has been removed.
    #[arg(long = "no-controller", hide = true)]
    _no_controller: bool,

    /// Show diagnostic debug line at the bottom of the screen.
    #[arg(long)]
    debug: bool,

    /// Use an ASCII-safe glyph palette for poor font environments.
    #[arg(long)]
    ascii_glyphs: bool,

    /// Disable the checkerboard background pattern.
    #[arg(long)]
    no_checkerboard: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let platform = Platform::detect();
    configure_glyphs(GlyphMode::resolve(cli.ascii_glyphs));

    run(cli, platform)
}

fn run(cli: Cli, platform: Platform) -> io::Result<()> {
    // Load before entering raw mode so any warning prints to a clean terminal.
    let mut high_score = load_high_score().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load high score: {e}");
        0
    });

    let mut themes = ThemeCatalog::load();
    if let Some(saved_theme) = load_theme_selection().unwrap_or(None)
        && !themes.select_by_id(&saved_theme)
    {
        eprintln!("Warning: saved theme '{saved_theme}' is unavailable; using default.");
    }

    let mut terminal_session = TerminalSession::enter()?;
    let terminal = terminal_session.terminal_mut();

    // Derive grid bounds from ratatui's own size so the logical grid
    // matches the exact frame area the renderer will use.
    let frame_area = terminal.size()?;
    let mut bounds = grid_bounds_from_frame(frame_area, cli.debug)?;
    let mut input = InputHandler::new();
    let mut start_speed_level = cli.speed.clamp(1, MAX_START_SPEED_LEVEL);
    let mut state = GameState::new_with_options(bounds, start_speed_level);
    state.status = GameStatus::Paused;
    let mut game_over_reference_high_score = high_score;

    let mut last_tick = Instant::now();
    let mut last_status = state.status;
    let mut last_input: Option<GameInput> = None;
    let mut last_input_tick: Option<u64> = None;
    let mut start_menu_selected_idx = 0usize;
    let mut pause_menu_selected_idx = 0usize;
    let mut game_over_menu_selected_idx = 0usize;
    let mut start_settings_open = false;
    let mut start_settings_selected_idx = 0usize;
    let mut theme_selection_mode: Option<ThemeSelectionMode> = None;
    let mut checkerboard_enabled = !cli.no_checkerboard;
    let mut start_speed_adjust_mode = false;
    let mut theme_selection_dirty = false;
    let mut pending_resize_reconcile = false;
    let mut last_resize_reconcile = Instant::now();

    loop {
        if pending_resize_reconcile || last_resize_reconcile.elapsed() >= Duration::from_millis(250)
        {
            reconcile_resize_if_needed(terminal, cli.debug, &mut bounds, &mut state)?;
            pending_resize_reconcile = false;
            last_resize_reconcile = Instant::now();
        }

        terminal.draw(|frame| {
            let start_theme_select = if state.is_start_screen()
                && theme_selection_mode == Some(ThemeSelectionMode::StartMenu)
            {
                Some(ThemeSelectView {
                    selected_idx: themes.current_index(),
                    themes: themes.items(),
                })
            } else {
                None
            };

            let pause_theme_select = if state.status == GameStatus::Paused
                && !state.is_start_screen()
                && theme_selection_mode == Some(ThemeSelectionMode::PauseMenu)
            {
                Some(ThemeSelectView {
                    selected_idx: themes.current_index(),
                    themes: themes.items(),
                })
            } else {
                None
            };

            renderer::render(
                frame,
                &state,
                platform,
                HudInfo {
                    high_score,
                    game_over_reference_high_score,
                    theme: themes.current_theme(),
                    debug: cli.debug,
                    debug_line: if cli.debug {
                        format_debug_line(&state, last_input, last_input_tick)
                    } else {
                        String::new()
                    },
                },
                MenuUiState {
                    start_selected_idx: start_menu_selected_idx,
                    start_settings_open,
                    start_settings_selected_idx,
                    start_speed_level,
                    start_speed_adjust_mode,
                    checkerboard_enabled,
                    pause_selected_idx: pause_menu_selected_idx,
                    game_over_selected_idx: game_over_menu_selected_idx,
                    start_theme_select,
                    pause_theme_select,
                },
            )
        })?;

        if let Some(game_input) = input.poll_input()? {
            if matches!(game_input, GameInput::Resize) {
                pending_resize_reconcile = true;
                continue;
            }

            last_input = Some(game_input);
            last_input_tick = Some(state.tick_count);

            if matches!(game_input, GameInput::Quit) {
                persist_selected_theme_if_dirty(&themes, &mut theme_selection_dirty);
                break;
            }

            if state.is_start_screen() {
                if theme_selection_mode == Some(ThemeSelectionMode::StartMenu) {
                    match game_input {
                        GameInput::Direction(Direction::Up) => {
                            themes.select_previous();
                            theme_selection_dirty = true;
                        }
                        GameInput::Direction(Direction::Down) | GameInput::CycleTheme => {
                            themes.select_next();
                            theme_selection_dirty = true;
                        }
                        GameInput::Confirm
                        | GameInput::Direction(Direction::Right)
                        | GameInput::Pause
                        | GameInput::Direction(Direction::Left) => {
                            theme_selection_mode = None;
                            persist_selected_theme_if_dirty(&themes, &mut theme_selection_dirty);
                        }
                        _ => {}
                    }

                    continue;
                }

                if start_speed_adjust_mode {
                    match game_input {
                        GameInput::Direction(Direction::Up) => {
                            start_speed_level = start_speed_level
                                .saturating_add(1)
                                .min(MAX_START_SPEED_LEVEL);
                            state.set_base_speed_level(start_speed_level);
                        }
                        GameInput::Direction(Direction::Down) => {
                            start_speed_level = start_speed_level
                                .saturating_sub(1)
                                .max(MIN_START_SPEED_LEVEL);
                            state.set_base_speed_level(start_speed_level);
                        }
                        GameInput::Confirm
                        | GameInput::Direction(Direction::Right)
                        | GameInput::Direction(Direction::Left)
                        | GameInput::Pause => {
                            start_speed_adjust_mode = false;
                        }
                        _ => {}
                    }

                    continue;
                }

                if start_settings_open {
                    match game_input {
                        GameInput::Direction(Direction::Up) => {
                            start_settings_selected_idx =
                                wrap_prev(start_settings_selected_idx, START_SETTINGS_ITEM_COUNT);
                        }
                        GameInput::Direction(Direction::Down) => {
                            start_settings_selected_idx =
                                wrap_next(start_settings_selected_idx, START_SETTINGS_ITEM_COUNT);
                        }
                        GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                            match start_settings_selected_idx {
                                START_SETTINGS_SPEED_IDX => {
                                    start_speed_adjust_mode = true;
                                }
                                START_SETTINGS_THEME_IDX => {
                                    theme_selection_mode = Some(ThemeSelectionMode::StartMenu)
                                }
                                START_SETTINGS_GRID_IDX => {
                                    checkerboard_enabled = !checkerboard_enabled;
                                }
                                START_SETTINGS_BACK_IDX => {
                                    start_settings_open = false;
                                }
                                _ => {}
                            }
                        }
                        GameInput::Direction(Direction::Left) | GameInput::Pause => {
                            start_settings_open = false;
                        }
                        _ => {}
                    }

                    continue;
                }

                match game_input {
                    GameInput::Direction(Direction::Up) => {
                        start_menu_selected_idx =
                            wrap_prev(start_menu_selected_idx, START_MENU_ITEM_COUNT);
                    }
                    GameInput::Direction(Direction::Down) => {
                        start_menu_selected_idx =
                            wrap_next(start_menu_selected_idx, START_MENU_ITEM_COUNT);
                    }
                    GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                        match start_menu_selected_idx {
                            START_MENU_START_IDX => {
                                state = GameState::new_with_options(bounds, start_speed_level);
                                state.status = GameStatus::Playing;
                            }
                            START_MENU_SETTINGS_IDX => {
                                start_settings_open = true;
                                start_settings_selected_idx = 0;
                            }
                            START_MENU_QUIT_IDX => {
                                persist_selected_theme_if_dirty(
                                    &themes,
                                    &mut theme_selection_dirty,
                                );
                                break;
                            }
                            _ => {}
                        }
                    }
                    GameInput::Pause => {}
                    _ => {}
                }

                continue;
            }

            if state.status == GameStatus::Paused {
                if theme_selection_mode == Some(ThemeSelectionMode::PauseMenu) {
                    match game_input {
                        GameInput::Direction(Direction::Up) => {
                            themes.select_previous();
                            theme_selection_dirty = true;
                        }
                        GameInput::Direction(Direction::Down) | GameInput::CycleTheme => {
                            themes.select_next();
                            theme_selection_dirty = true;
                        }
                        GameInput::Confirm
                        | GameInput::Direction(Direction::Right)
                        | GameInput::Pause
                        | GameInput::Direction(Direction::Left) => {
                            theme_selection_mode = None;
                            persist_selected_theme_if_dirty(&themes, &mut theme_selection_dirty);
                        }
                        _ => {}
                    }

                    continue;
                }

                match game_input {
                    GameInput::Direction(Direction::Up) => {
                        pause_menu_selected_idx = wrap_prev(pause_menu_selected_idx, 3);
                    }
                    GameInput::Direction(Direction::Down) => {
                        pause_menu_selected_idx = wrap_next(pause_menu_selected_idx, 3);
                    }
                    GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                        match pause_menu_selected_idx {
                            0 => state.status = GameStatus::Playing,
                            1 => theme_selection_mode = Some(ThemeSelectionMode::PauseMenu),
                            2 => {
                                persist_selected_theme_if_dirty(
                                    &themes,
                                    &mut theme_selection_dirty,
                                );
                                break;
                            }
                            _ => {}
                        }
                    }
                    GameInput::Pause | GameInput::Direction(Direction::Left) => {
                        state.status = GameStatus::Playing
                    }
                    _ => {}
                }

                continue;
            }

            if matches!(state.status, GameStatus::GameOver | GameStatus::Victory) {
                match game_input {
                    GameInput::Direction(Direction::Up) => {
                        game_over_menu_selected_idx = wrap_prev(game_over_menu_selected_idx, 2);
                    }
                    GameInput::Direction(Direction::Down) => {
                        game_over_menu_selected_idx = wrap_next(game_over_menu_selected_idx, 2);
                    }
                    GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                        if game_over_menu_selected_idx == 0 {
                            state = state.restart();
                            state.status = GameStatus::Paused;
                        } else {
                            persist_selected_theme_if_dirty(&themes, &mut theme_selection_dirty);
                            break;
                        }
                    }
                    _ => {}
                }

                continue;
            }

            match game_input {
                GameInput::CycleTheme => {
                    themes.select_next();
                    theme_selection_dirty = true;
                }
                other => handle_input(&mut state, other),
            }
        }

        let tick_interval = tick_interval_for_speed(state.speed_level);
        if last_tick.elapsed() >= tick_interval {
            if state.status == GameStatus::Playing {
                state.record_tick_duration(tick_interval);
            }
            state.tick();
            last_tick = Instant::now();
        }

        if state.status != last_status {
            if matches!(state.status, GameStatus::GameOver | GameStatus::Victory) {
                game_over_reference_high_score = high_score;
                game_over_menu_selected_idx = 0;
                theme_selection_mode = None;
                start_speed_adjust_mode = false;
                start_settings_open = false;
                start_settings_selected_idx = 0;

                if state.score > high_score {
                    high_score = state.score;
                    if let Err(error) = save_high_score(high_score) {
                        eprintln!("Failed to save high score: {error}");
                    }
                }
            }

            if state.status == GameStatus::Paused && !state.is_start_screen() {
                pause_menu_selected_idx = 0;
                start_speed_adjust_mode = false;
                start_settings_open = false;
            }

            if state.status == GameStatus::Playing {
                theme_selection_mode = None;
                start_speed_adjust_mode = false;
                start_settings_open = false;
            }

            last_status = state.status;
        }

        thread::sleep(Duration::from_millis(16));
    }

    persist_selected_theme_if_dirty(&themes, &mut theme_selection_dirty);

    Ok(())
}

fn persist_selected_theme(catalog: &ThemeCatalog) {
    if let Err(e) = save_theme_selection(catalog.current_id(), &catalog.current_theme().name) {
        eprintln!("Failed to save theme: {e}");
    }
}

fn persist_selected_theme_if_dirty(catalog: &ThemeCatalog, dirty: &mut bool) {
    if *dirty {
        persist_selected_theme(catalog);
        *dirty = false;
    }
}

fn handle_input(state: &mut GameState, input: GameInput) {
    match input {
        GameInput::Confirm if state.is_start_screen() => {
            state.status = GameStatus::Playing;
        }
        GameInput::Confirm
            if matches!(state.status, GameStatus::GameOver | GameStatus::Victory) =>
        {
            *state = state.restart();
            state.status = GameStatus::Paused;
        }
        other => state.apply_input(other),
    }
}

/// Derives grid bounds from the ratatui frame area.
///
/// This uses the exact same dimensions as the renderer, eliminating any
/// possible mismatch between the logical grid and the gameplay viewport.
fn grid_bounds_from_frame(size: Size, debug_enabled: bool) -> io::Result<GridSize> {
    let hud_rows: u16 = 2 + u16::from(debug_enabled) + HUD_BOTTOM_MARGIN_Y;

    let min_w: u16 = PLAY_AREA_MARGIN_X.saturating_mul(2).saturating_add(1);
    let min_h: u16 = hud_rows
        .saturating_add(PLAY_AREA_MARGIN_Y.saturating_mul(2))
        .saturating_add(1);
    if size.width < min_w || size.height < min_h {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Terminal too small: need at least {min_w}x{min_h}, got {}x{}.",
                size.width, size.height,
            ),
        ));
    }

    // play_area = full width, full height minus HUD/debug rows.
    // gameplay_viewport = play_area inset by configured side/top/bottom margins.
    let viewport_w = size
        .width
        .saturating_sub(PLAY_AREA_MARGIN_X.saturating_mul(2));
    let viewport_h = size
        .height
        .saturating_sub(hud_rows)
        .saturating_sub(PLAY_AREA_MARGIN_Y.saturating_mul(2));

    // Each terminal row holds 2 game rows (half-block rendering),
    // so the logical game height is double the available terminal rows.
    let game_h = viewport_h.saturating_mul(2);

    let width = viewport_w;
    let height = game_h;

    Ok(GridSize { width, height })
}

fn format_debug_line(
    state: &GameState,
    last_input: Option<GameInput>,
    last_input_tick: Option<u64>,
) -> String {
    let head = state.snake.head();
    let next = state.snake.next_head_position();
    format!(
        "dbg tick={} status={:?} in={:?}@{:?} dir={:?} head=({}, {}) next=({}, {})",
        state.tick_count,
        state.status,
        last_input,
        last_input_tick,
        state.snake.direction(),
        head.x,
        head.y,
        next.x,
        next.y,
    )
}

fn reconcile_resize_if_needed(
    terminal: &mut AppTerminal,
    debug_enabled: bool,
    bounds: &mut GridSize,
    state: &mut GameState,
) -> io::Result<()> {
    let frame_area = terminal.size()?;
    let Ok(next_bounds) = grid_bounds_from_frame(frame_area, debug_enabled) else {
        return Ok(());
    };

    if next_bounds != *bounds {
        *bounds = next_bounds;
        state.resize_bounds(*bounds);
    }

    Ok(())
}

fn tick_interval_for_speed(speed_level: u32) -> Duration {
    let speed_penalty_ms = u64::from(speed_level.saturating_sub(1)) * 10;
    let clamped_ms = DEFAULT_TICK_INTERVAL_MS
        .saturating_sub(speed_penalty_ms)
        .max(MIN_TICK_INTERVAL_MS);
    Duration::from_millis(clamped_ms)
}

fn wrap_next(current: usize, len: usize) -> usize {
    (current + 1) % len
}

fn wrap_prev(current: usize, len: usize) -> usize {
    if current == 0 { len - 1 } else { current - 1 }
}
