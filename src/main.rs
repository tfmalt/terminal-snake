use std::io;
use std::panic;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use snake::config::{
    DEFAULT_GRID_HEIGHT, DEFAULT_GRID_WIDTH, DEFAULT_TICK_INTERVAL_MS, MIN_TICK_INTERVAL_MS,
    GridSize,
};
use snake::game::{GameState, GameStatus};
use snake::input::{GameInput, InputConfig, InputHandler};
use snake::platform::Platform;
use snake::renderer;
use snake::score::{load_high_score, save_high_score};
use snake::ui::hud::HudInfo;

#[derive(Debug, Parser)]
struct Cli {
    /// Starting speed level.
    #[arg(long, default_value_t = 1)]
    speed: u32,

    /// Grid width in logical cells.
    #[arg(long, default_value_t = DEFAULT_GRID_WIDTH)]
    width: u16,

    /// Grid height in logical cells.
    #[arg(long, default_value_t = DEFAULT_GRID_HEIGHT)]
    height: u16,

    /// Disable controller input even when available.
    #[arg(long = "no-controller")]
    no_controller: bool,

    /// Disable colored rendering.
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
    let bounds = GridSize { width: cli.width, height: cli.height };
    validate_terminal_size(bounds)?;

    // Load before entering raw mode so any warning prints to a clean terminal.
    let mut high_score = load_high_score().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load high score: {e}");
        0
    });

    let mut terminal = setup_terminal()?;
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
                    monochrome: cli.no_color,
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

            handle_input(&mut state, game_input);
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

fn validate_terminal_size(bounds: GridSize) -> io::Result<()> {
    let (terminal_width, terminal_height) = terminal::size()?;

    let required_width = bounds.width.saturating_add(2);
    let required_height = bounds.height.saturating_add(4);

    if terminal_width < required_width || terminal_height < required_height {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Terminal too small: need at least {required_width}x{required_height}, got {terminal_width}x{terminal_height}. Try --width/--height or resize terminal."
            ),
        ));
    }

    Ok(())
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
