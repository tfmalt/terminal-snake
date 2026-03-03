use std::io;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use ratatui::layout::Size;
use terminal_snake::config::{
    DEFAULT_TICK_INTERVAL_MS, GlyphMode, GridSize, HUD_BOTTOM_MARGIN_Y, MAX_START_SPEED_LEVEL,
    MIN_TICK_INTERVAL_MS, PLAY_AREA_MARGIN_X, PLAY_AREA_MARGIN_Y, configure_glyphs,
};
use terminal_snake::game::{GameState, GameStatus};
use terminal_snake::input::{GameInput, InputHandler};
use terminal_snake::menu_state::{MenuAction, MenuScreen, MenuState};
use terminal_snake::platform::Platform;
use terminal_snake::renderer::{self, MenuUiState};
use terminal_snake::score::{
    load_high_score, load_theme_selection, save_high_score, save_theme_selection,
};
use terminal_snake::session::{
    SessionFile, find_session_by_hash, load_active_sessions, load_latest_session,
    remove_session_by_hash, save_active_session,
};
use terminal_snake::terminal_runtime::{AppTerminal, TerminalSession};
use terminal_snake::theme::ThemeCatalog;
use terminal_snake::ui::hud::{HudInfo, HudValueFlash};
use terminal_snake::ui::menu::ThemeSelectView;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HudTrackedValues {
    length: usize,
    level: u32,
    score: u32,
    high_score: u32,
    dimensions: (u16, u16),
    foods_to_next_level: u32,
    food_count: usize,
    next_points: u32,
    bonus_multiplier_hundredths: u32,
    coverage_hundredths: u32,
}

#[derive(Debug, Clone)]
struct SavedSessionNotice {
    resume_hash: String,
    score: u32,
    snake_length: usize,
    speed_level: u32,
    elapsed: Duration,
    coverage_percent: f64,
}

impl SavedSessionNotice {
    fn from_state_and_session(state: &GameState, session: &SessionFile) -> Self {
        Self {
            resume_hash: session.resume_hash.clone(),
            score: state.score,
            snake_length: state.snake.len(),
            speed_level: state.speed_level,
            elapsed: state.elapsed_duration(),
            coverage_percent: state.play_area_coverage_percent(),
        }
    }
}

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

    /// Resume a previously saved local session by hash.
    #[arg(long, conflicts_with = "resume_list")]
    resume: Option<String>,

    /// List saved resumable sessions and exit.
    #[arg(long = "resume-list")]
    resume_list: bool,
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

    let active_sessions = load_active_sessions().unwrap_or_else(|error| {
        eprintln!("Warning: failed to load active sessions: {error}");
        Vec::new()
    });

    if cli.resume_list {
        print_resume_list(&active_sessions);
        return Ok(());
    }

    let mut has_active_sessions = !active_sessions.is_empty();

    let mut terminal_session = TerminalSession::enter()?;
    let terminal = terminal_session.terminal_mut();

    let frame_area = terminal.size()?;
    let mut bounds = grid_bounds_from_frame(frame_area, cli.debug)?;
    let mut play_area_is_too_small = play_area_too_small(frame_area, cli.debug);
    let mut input = InputHandler::new();
    let start_speed_level = cli.speed.clamp(1, MAX_START_SPEED_LEVEL);
    let mut state = GameState::new_with_options(bounds, start_speed_level);
    if let Some(resume_hash) = cli.resume.as_deref() {
        match find_session_by_hash(resume_hash) {
            Ok(Some(session)) => match GameState::from_snapshot_v1(session.snapshot.clone()) {
                Ok(mut restored) => {
                    if apply_resume_bounds_or_warn(&mut restored, bounds) {
                        state = restored;
                        state.status = GameStatus::Playing;
                        if let Err(error) = remove_session_by_hash(&session.resume_hash) {
                            eprintln!("Warning: failed to remove resumed session: {error}");
                        }
                    }
                }
                Err(error) => {
                    eprintln!(
                        "Warning: failed to restore session for hash '{resume_hash}': {error}"
                    );
                }
            },
            Ok(None) => {
                eprintln!("Warning: no active session found for hash '{resume_hash}'.");
            }
            Err(error) => {
                eprintln!("Warning: failed to load session for hash '{resume_hash}': {error}");
            }
        }
        has_active_sessions = load_active_sessions()
            .map(|sessions| !sessions.is_empty())
            .unwrap_or(has_active_sessions);
    }

    if state.tick_count == 0 && state.score == 0 {
        state.status = GameStatus::Paused;
    } else if cli.resume.is_some() {
        state.status = GameStatus::Playing;
    }
    let mut game_over_reference_high_score = high_score;

    let mut last_tick = Instant::now();
    let mut last_status = state.status;
    let mut game_over_input_locked_until: Option<Instant> = None;
    let mut last_input: Option<GameInput> = None;
    let mut last_input_tick: Option<u64> = None;
    let mut pending_resize_reconcile = false;
    let mut last_resize_reconcile = Instant::now();
    let mut hud_value_flash = HudValueFlash::default();
    let mut last_hud_values: Option<HudTrackedValues> = None;
    let mut quit_notice: Option<SavedSessionNotice> = None;

    let mut menu = MenuState::new(start_speed_level, !cli.no_checkerboard);
    menu.set_has_active_session(has_active_sessions);

    if play_area_is_too_small && state.status == GameStatus::Playing {
        state.status = GameStatus::Paused;
    }

    loop {
        if pending_resize_reconcile || last_resize_reconcile.elapsed() >= Duration::from_millis(250)
        {
            reconcile_resize_if_needed(terminal, cli.debug, &mut bounds, &mut state)?;
            let frame_area = terminal.size()?;
            play_area_is_too_small = play_area_too_small(frame_area, cli.debug);
            if play_area_is_too_small && state.status == GameStatus::Playing {
                state.status = GameStatus::Paused;
                menu.enter_pause();
            }
            pending_resize_reconcile = false;
            last_resize_reconcile = Instant::now();
        }

        terminal.draw(|frame| {
            let now = Instant::now();
            let displayed_high_score = high_score.max(state.score);
            let hud_values = collect_hud_tracked_values(&state, displayed_high_score);
            update_hud_value_flash(&mut hud_value_flash, &mut last_hud_values, hud_values, now);

            let start_theme_select =
                if state.is_start_screen() && menu.screen == MenuScreen::StartThemeSelect {
                    Some(ThemeSelectView {
                        selected_idx: themes.current_index(),
                        themes: themes.items(),
                    })
                } else {
                    None
                };

            let pause_theme_select = if state.status == GameStatus::Paused
                && !state.is_start_screen()
                && menu.screen == MenuScreen::PauseThemeSelect
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
                    high_score: displayed_high_score,
                    game_over_reference_high_score,
                    theme: themes.current_theme(),
                    debug: cli.debug,
                    debug_line: if cli.debug {
                        format_debug_line(&state, last_input, last_input_tick)
                    } else {
                        String::new()
                    },
                    now,
                    value_flash: hud_value_flash,
                },
                MenuUiState {
                    start_selected_idx: menu.start_selected_idx,
                    start_settings_open: menu.is_settings_open(),
                    start_settings_selected_idx: menu.start_settings_selected_idx,
                    start_speed_level: menu.start_speed_level,
                    start_speed_adjust_mode: menu.is_speed_adjust(),
                    checkerboard_enabled: menu.checkerboard_enabled,
                    game_border_enabled: menu.game_border_enabled,
                    has_active_session: menu.has_active_session,
                    play_area_too_small: play_area_is_too_small,
                    pause_selected_idx: menu.pause_selected_idx,
                    game_over_selected_idx: menu.game_over_selected_idx,
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

            if state.status == GameStatus::GameOver
                && game_over_input_locked_until.is_some_and(|until| Instant::now() < until)
            {
                continue;
            }

            if matches!(game_input, GameInput::Quit) {
                if let Some(saved) = persist_active_session_if_resumable(&state) {
                    quit_notice = Some(SavedSessionNotice::from_state_and_session(&state, &saved));
                    menu.set_has_active_session(true);
                }
                persist_theme_if_dirty(&themes, &mut menu.theme_dirty);
                break;
            }

            let action = dispatch_input(
                &mut state,
                &mut menu,
                &mut themes,
                game_input,
                play_area_is_too_small,
            );

            match action {
                MenuAction::StartGame => {
                    state = GameState::new_with_options(bounds, menu.start_speed_level);
                    state.status = GameStatus::Playing;
                }
                MenuAction::Resume => {
                    state.status = GameStatus::Playing;
                }
                MenuAction::ResumeSession => match load_latest_session() {
                    Ok(Some(session_file)) => {
                        match GameState::from_snapshot_v1(session_file.snapshot.clone()) {
                            Ok(mut restored) => {
                                if apply_resume_bounds_or_warn(&mut restored, bounds) {
                                    state = restored;
                                    state.status = GameStatus::Playing;
                                    if let Err(error) =
                                        remove_session_by_hash(&session_file.resume_hash)
                                    {
                                        eprintln!(
                                            "Warning: failed to remove resumed session: {error}"
                                        );
                                    }
                                    has_active_sessions =
                                        refresh_has_active_sessions(has_active_sessions);
                                    menu.set_has_active_session(has_active_sessions);
                                }
                            }
                            Err(error) => {
                                eprintln!("Warning: failed to restore active session: {error}");
                            }
                        }
                    }
                    Ok(None) => {
                        has_active_sessions = false;
                        menu.set_has_active_session(false);
                    }
                    Err(error) => {
                        eprintln!("Warning: failed to load active sessions: {error}");
                    }
                },
                MenuAction::Restart => {
                    state = state.restart();
                    state.status = GameStatus::Paused;
                    menu.enter_start();
                    game_over_input_locked_until = None;
                }
                MenuAction::Quit => {
                    if let Some(saved) = persist_active_session_if_resumable(&state) {
                        quit_notice =
                            Some(SavedSessionNotice::from_state_and_session(&state, &saved));
                        menu.set_has_active_session(true);
                    }
                    persist_theme_if_dirty(&themes, &mut menu.theme_dirty);
                    break;
                }
                MenuAction::Consumed => {}
            }

            if matches!(
                action,
                MenuAction::StartGame
                    | MenuAction::Resume
                    | MenuAction::ResumeSession
                    | MenuAction::Restart
            ) {
                continue;
            }

            if state.is_start_screen()
                || state.status == GameStatus::Paused
                || matches!(state.status, GameStatus::GameOver | GameStatus::Victory)
            {
                continue;
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
            if state.status == GameStatus::GameOver {
                game_over_reference_high_score = high_score;
                menu.enter_game_over();
                game_over_input_locked_until = Some(Instant::now() + Duration::from_secs(1));

                if state.score > high_score {
                    high_score = state.score;
                    if let Err(error) = save_high_score(high_score) {
                        eprintln!("Failed to save high score: {error}");
                    }
                }
            }

            if state.status == GameStatus::Victory {
                game_over_reference_high_score = high_score;
                menu.enter_game_over();

                if state.score > high_score {
                    high_score = state.score;
                    if let Err(error) = save_high_score(high_score) {
                        eprintln!("Failed to save high score: {error}");
                    }
                }
            }

            if state.status == GameStatus::Paused && !state.is_start_screen() {
                menu.enter_pause();
            }

            if state.status == GameStatus::Playing {
                menu.enter_playing();
                game_over_input_locked_until = None;
            }

            last_status = state.status;
        }

        thread::sleep(Duration::from_millis(16));
    }

    persist_theme_if_dirty(&themes, &mut menu.theme_dirty);

    drop(terminal_session);
    if let Some(notice) = quit_notice {
        print_saved_session_notice(&notice);
    }

    Ok(())
}

/// Routes a game input to the appropriate menu or gameplay handler.
fn dispatch_input(
    state: &mut GameState,
    menu: &mut MenuState,
    themes: &mut ThemeCatalog,
    input: GameInput,
    play_area_too_small: bool,
) -> MenuAction {
    if state.is_start_screen() {
        let action = menu.handle_start_input(input, themes, play_area_too_small);
        // Sync speed level back to game state when adjusted.
        if menu.is_speed_adjust() || menu.screen == MenuScreen::StartSettings {
            state.set_base_speed_level(menu.start_speed_level);
        }
        if menu.theme_dirty
            && !matches!(
                menu.screen,
                MenuScreen::StartThemeSelect | MenuScreen::PauseThemeSelect
            )
        {
            persist_theme(themes);
            menu.theme_dirty = false;
        }
        return action;
    }

    if state.status == GameStatus::Paused {
        let action = menu.handle_pause_input(input, themes, play_area_too_small);
        if menu.theme_dirty
            && !matches!(
                menu.screen,
                MenuScreen::StartThemeSelect | MenuScreen::PauseThemeSelect
            )
        {
            persist_theme(themes);
            menu.theme_dirty = false;
        }
        return action;
    }

    if matches!(state.status, GameStatus::GameOver | GameStatus::Victory) {
        return menu.handle_game_over_input(input);
    }

    // Playing state — pass input to game.
    match input {
        GameInput::CycleTheme => {
            themes.select_next();
            menu.theme_dirty = true;
        }
        other => handle_gameplay_input(state, other),
    }

    MenuAction::Consumed
}

fn persist_theme(catalog: &ThemeCatalog) {
    if let Err(e) = save_theme_selection(catalog.current_id(), &catalog.current_theme().name) {
        eprintln!("Failed to save theme: {e}");
    }
}

fn persist_theme_if_dirty(catalog: &ThemeCatalog, dirty: &mut bool) {
    if *dirty {
        persist_theme(catalog);
        *dirty = false;
    }
}

fn persist_active_session_if_resumable(state: &GameState) -> Option<SessionFile> {
    let is_resumable = state.status == GameStatus::Playing
        || (state.status == GameStatus::Paused && !state.is_start_screen());

    if !is_resumable {
        return None;
    }

    match save_active_session(&state.to_snapshot_v1()) {
        Ok(saved) => Some(saved),
        Err(error) => {
            eprintln!("Warning: failed to save active session: {error}");
            None
        }
    }
}

fn refresh_has_active_sessions(current: bool) -> bool {
    load_active_sessions()
        .map(|sessions| !sessions.is_empty())
        .unwrap_or(current)
}

fn print_saved_session_notice(notice: &SavedSessionNotice) {
    eprintln!("Saved active session.");
    eprintln!(
        "  Score: {} | Length: {} | Speed: {} | Coverage: {:.2}% | Time: {}",
        notice.score,
        notice.snake_length,
        notice.speed_level,
        notice.coverage_percent,
        format_duration_mm_ss(notice.elapsed),
    );
    eprintln!("  Resume hash: {}", notice.resume_hash);
    eprintln!(
        "  Resume with: terminal-snake --resume {}",
        notice.resume_hash
    );
    eprintln!("  Or open the start menu and choose Resume.");
}

fn print_resume_list(sessions: &[SessionFile]) {
    if sessions.is_empty() {
        println!("No saved sessions.");
        return;
    }

    println!("Saved sessions: {}", sessions.len());
    for (index, session) in sessions.iter().enumerate() {
        let snake_len = session.snapshot.snake.segments.len();
        let total_cells = session.snapshot.bounds.total_cells();
        let coverage = if total_cells == 0 {
            0.0
        } else {
            (snake_len as f64 / total_cells as f64) * 100.0
        };

        println!(
            "{}. {} | score={} len={} lvl={} coverage={:.2}% time={} saved_unix_ms={}",
            index + 1,
            session.resume_hash,
            session.snapshot.score,
            snake_len,
            session.snapshot.speed_level,
            coverage,
            format_duration_mm_ss(Duration::from_millis(session.snapshot.elapsed_millis)),
            session.saved_at_unix_ms,
        );
        println!("   resume: terminal-snake --resume {}", session.resume_hash);
    }
}

fn apply_resume_bounds_or_warn(state: &mut GameState, current_bounds: GridSize) -> bool {
    let resumed_bounds = state.bounds();
    if resumed_bounds == current_bounds {
        return true;
    }

    if current_bounds.width < resumed_bounds.width || current_bounds.height < resumed_bounds.height
    {
        eprintln!(
            "Warning: terminal is {}x{}, but resumed game needs at least {}x{}. Resize the terminal to equal or larger size and resume again.",
            current_bounds.width,
            current_bounds.height,
            resumed_bounds.width,
            resumed_bounds.height,
        );
        return false;
    }

    state.resize_bounds(current_bounds);
    true
}

fn format_duration_mm_ss(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn handle_gameplay_input(state: &mut GameState, input: GameInput) {
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

    let viewport_w = size
        .width
        .saturating_sub(PLAY_AREA_MARGIN_X.saturating_mul(2));
    let viewport_h = size
        .height
        .saturating_sub(hud_rows)
        .saturating_sub(PLAY_AREA_MARGIN_Y.saturating_mul(2));

    let game_h = viewport_h.saturating_mul(2);

    let width = viewport_w;
    let height = game_h;

    Ok(GridSize { width, height })
}

fn play_area_too_small(size: Size, debug_enabled: bool) -> bool {
    const MIN_GAME_AREA_CELLS: u16 = 30;

    let hud_rows: u16 = 2 + u16::from(debug_enabled) + HUD_BOTTOM_MARGIN_Y;
    let viewport_w = size
        .width
        .saturating_sub(PLAY_AREA_MARGIN_X.saturating_mul(2));
    let viewport_h = size
        .height
        .saturating_sub(hud_rows)
        .saturating_sub(PLAY_AREA_MARGIN_Y.saturating_mul(2));
    let game_h = viewport_h.saturating_mul(2);

    viewport_w < MIN_GAME_AREA_CELLS || game_h < MIN_GAME_AREA_CELLS
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

fn collect_hud_tracked_values(state: &GameState, displayed_high_score: u32) -> HudTrackedValues {
    HudTrackedValues {
        length: state.snake.len(),
        level: state.speed_level,
        score: state.score,
        high_score: displayed_high_score,
        dimensions: (state.bounds().width, state.bounds().height),
        foods_to_next_level: state.foods_until_next_speed_level(),
        food_count: state.calculated_food_count(),
        next_points: state.ordinary_food_projected_points(),
        bonus_multiplier_hundredths: (state.ordinary_food_projected_multiplier() * 100.0).round()
            as u32,
        coverage_hundredths: (state.play_area_coverage_percent() * 100.0).round() as u32,
    }
}

fn update_hud_value_flash(
    flash: &mut HudValueFlash,
    previous: &mut Option<HudTrackedValues>,
    current: HudTrackedValues,
    now: Instant,
) {
    let Some(prev) = previous else {
        *previous = Some(current);
        return;
    };

    if prev.length != current.length {
        flash.length_changed_at = Some(now);
    }
    if prev.level != current.level {
        flash.level_changed_at = Some(now);
    }
    if prev.score != current.score {
        flash.score_changed_at = Some(now);
    }
    if prev.high_score != current.high_score {
        flash.high_score_changed_at = Some(now);
    }
    if prev.dimensions != current.dimensions {
        flash.dimensions_changed_at = Some(now);
    }
    if prev.foods_to_next_level != current.foods_to_next_level {
        flash.foods_to_next_level_changed_at = Some(now);
    }
    if prev.food_count != current.food_count {
        flash.food_count_changed_at = Some(now);
    }
    if prev.next_points != current.next_points {
        flash.next_points_changed_at = Some(now);
    }
    if prev.bonus_multiplier_hundredths != current.bonus_multiplier_hundredths {
        flash.bonus_multiplier_changed_at = Some(now);
    }
    if prev.coverage_hundredths != current.coverage_hundredths {
        flash.coverage_changed_at = Some(now);
    }

    *previous = Some(current);
}
