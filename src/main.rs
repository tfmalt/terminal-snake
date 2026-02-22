use std::io;
use std::panic;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Size;
use ratatui::Terminal;
use snake::config::{DEFAULT_TICK_INTERVAL_MS, MIN_TICK_INTERVAL_MS, GridSize, THEMES};
use snake::game::{GameState, GameStatus};
use snake::input::{GameInput, InputConfig, InputHandler};
use snake::platform::Platform;
use snake::renderer;
use snake::score::{load_high_score, load_theme_name, save_high_score, save_theme_name};
use snake::ui::hud::HudInfo;

#[derive(Debug, Parser)]
struct Cli {
    /// Starting speed level.
    #[arg(long, default_value_t = 1)]
    speed: u32,

    /// Grid width in logical cells (defaults to terminal width).
    #[arg(long)]
    width: Option<u16>,

    /// Grid height in logical cells (defaults to terminal height).
    #[arg(long)]
    height: Option<u16>,

    /// Disable controller input even when available.
    #[arg(long = "no-controller")]
    no_controller: bool,

    /// Disable colored rendering (forces Mono theme, disables theme cycling).
    #[arg(long = "no-color")]
    no_color: bool,

    /// Show diagnostic debug line at the bottom of the screen.
    #[arg(long)]
    debug: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let platform = Platform::detect();

    install_panic_hook();

    run(cli, platform)?;
    cleanup_terminal()?;
    Ok(())
}

fn run(cli: Cli, platform: Platform) -> io::Result<()> {
    // Load before entering raw mode so any warning prints to a clean terminal.
    let mut high_score = load_high_score().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load high score: {e}");
        0
    });

    let mut selected_theme_idx = if cli.no_color {
        THEMES
            .iter()
            .position(|t| t.name == "Mono")
            .unwrap_or(THEMES.len() - 1)
    } else {
        let saved_name = load_theme_name().unwrap_or(None);
        saved_name
            .as_deref()
            .and_then(|name| THEMES.iter().position(|t| t.name == name))
            .unwrap_or(0)
    };

    let mut terminal = setup_terminal()?;

    // Derive grid bounds from ratatui's own size so the logical grid
    // matches the exact frame area the renderer will use.
    let frame_area = terminal.size()?;
    let bounds = grid_bounds_from_frame(frame_area, &cli)?;
    let mut input = InputHandler::new(InputConfig {
        enable_controller: !cli.no_controller,
        is_wsl: platform.is_wsl(),
    });
    let mut state = GameState::new_with_options(bounds, cli.speed);
    state.status = GameStatus::Paused;
    let mut game_over_reference_high_score = high_score;

    let controller_enabled = !cli.no_controller && !platform.is_wsl();
    let mut last_tick = Instant::now();
    let mut last_status = state.status;
    let mut last_input: Option<GameInput> = None;
    let mut last_input_tick: Option<u64> = None;

    loop {
        terminal.draw(|frame| {
            renderer::render(
                frame,
                &state,
                platform,
                HudInfo {
                    high_score,
                    game_over_reference_high_score,
                    controller_enabled,
                    theme: &THEMES[selected_theme_idx],
                    debug: cli.debug,
                    debug_line: if cli.debug {
                        format_debug_line(&state, last_input, last_input_tick)
                    } else {
                        String::new()
                    },
                },
            )
        })?;

        if let Some(game_input) = input.poll_input()? {
            last_input = Some(game_input);
            last_input_tick = Some(state.tick_count);

            if matches!(game_input, GameInput::Quit) {
                break;
            }

            match game_input {
                GameInput::CycleTheme if state.is_start_screen() && !cli.no_color => {
                    selected_theme_idx = (selected_theme_idx + 1) % THEMES.len();
                    if let Err(e) = save_theme_name(THEMES[selected_theme_idx].name) {
                        eprintln!("Failed to save theme: {e}");
                    }
                }
                GameInput::CycleTheme => {}
                other => handle_input(&mut state, other),
            }
        }

        let tick_interval = tick_interval_for_speed(state.speed_level);
        if last_tick.elapsed() >= tick_interval {
            state.tick();
            last_tick = Instant::now();
        }

        if state.status != last_status {
            if matches!(state.status, GameStatus::GameOver | GameStatus::Victory) {
                game_over_reference_high_score = high_score;

                if state.score > high_score {
                    high_score = state.score;
                    if let Err(error) = save_high_score(high_score) {
                        eprintln!("Failed to save high score: {error}");
                    }
                }
            }

            last_status = state.status;
        }

        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
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
/// possible mismatch between the logical grid and the visual border.
fn grid_bounds_from_frame(size: Size, cli: &Cli) -> io::Result<GridSize> {
    let hud_rows: u16 = 2 + u16::from(cli.debug);

    let min_w: u16 = 5;
    let min_h: u16 = 4 + hud_rows;
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
    // inner = play_area minus 2 for the border (1 each side).
    let inner_w = size.width.saturating_sub(2);
    let inner_h = size.height.saturating_sub(2 + hud_rows);

    let width = cli.width.unwrap_or(inner_w).min(inner_w);
    let height = cli.height.unwrap_or(inner_h).min(inner_h);

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

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn cleanup_terminal() -> io::Result<()> {
    disable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, Show, LeaveAlternateScreen)?;

    Ok(())
}

fn install_panic_hook() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal_after_panic();
        default_hook(panic_info);
    }));
}

fn restore_terminal_after_panic() {
    let _ = disable_raw_mode();

    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show, LeaveAlternateScreen);
}

fn tick_interval_for_speed(speed_level: u32) -> Duration {
    let speed_penalty_ms = u64::from(speed_level.saturating_sub(1)) * 10;
    let clamped_ms = DEFAULT_TICK_INTERVAL_MS
        .saturating_sub(speed_penalty_ms)
        .max(MIN_TICK_INTERVAL_MS);
    Duration::from_millis(clamped_ms)
}
