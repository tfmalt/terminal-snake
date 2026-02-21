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
}

#[derive(Debug, Clone, Copy)]
struct AppConfig {
    bounds: (u16, u16),
    starting_speed: u32,
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
    let app_config = AppConfig {
        bounds: (cli.width, cli.height),
        starting_speed: cli.speed,
    };
    validate_terminal_size(app_config.bounds)?;

    let mut terminal = setup_terminal()?;
    let mut input = InputHandler::new(InputConfig {
        enable_controller: !cli.no_controller,
        is_wsl: platform.is_wsl(),
    });
    let mut state = GameState::new_with_options(app_config.bounds, app_config.starting_speed);
    state.status = GameStatus::Paused;
    let mut high_score = load_high_score();
    let mut game_over_reference_high_score = high_score;

    let controller_enabled = !cli.no_controller && !platform.is_wsl();
    let mut last_tick = Instant::now();
    let mut last_status = state.status;

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
                },
            )
        })?;

        if let Some(game_input) = input.poll_input()? {
            if matches!(game_input, GameInput::Quit) {
                break;
            }

            handle_input(&mut state, game_input, app_config);
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

fn handle_input(state: &mut GameState, input: GameInput, app_config: AppConfig) {
    match input {
        GameInput::Confirm if is_start_screen(state) => {
            state.status = GameStatus::Playing;
        }
        GameInput::Confirm
            if matches!(state.status, GameStatus::GameOver | GameStatus::Victory) =>
        {
            *state = GameState::new_with_options(app_config.bounds, app_config.starting_speed);
            state.status = GameStatus::Paused;
        }
        other => state.apply_input(other),
    }
}

fn validate_terminal_size(bounds: (u16, u16)) -> io::Result<()> {
    let (terminal_width, terminal_height) = terminal::size()?;

    let required_width = bounds.0.saturating_mul(2).saturating_add(2);
    let required_height = bounds.1.saturating_add(3);

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

fn is_start_screen(state: &GameState) -> bool {
    state.status == GameStatus::Paused && state.tick_count == 0 && state.score == 0
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
