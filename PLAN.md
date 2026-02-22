# PLAN.md — Snake Game Development Plan

Incremental development plan. Each phase produces a working, runnable game.
Complete phases in order — later phases build on earlier ones.

---

## Phase 1: Project Scaffold [x Completed]

Get a compilable project with the correct dependency graph and CI skeleton in
place before writing any game code.

- [ ] `cargo new snake --edition 2021`
- [ ] Add dependencies to `Cargo.toml`:
  - `ratatui = "0.29"`
  - `crossterm = "0.28"`
  - `gilrs = "0.11"`
  - `rand = "0.8"`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `clap = { version = "4", features = ["derive"] }`
  - `dirs = "5"`
  - `thiserror = "1"`
  - `unicode-width = "0.1"`
- [ ] Create the full `src/` directory structure as defined in `CLAUDE.md`
- [ ] Add stub `mod` declarations in `main.rs` so the project compiles with
  empty modules
- [ ] Write `config.rs` with all glyph constants, grid size defaults, tick
  rate defaults, speed levels
- [ ] Write `platform.rs` with WSL detection (`/proc/version` check) and a
  `Platform` struct
- [ ] Add `.gitignore` (Rust standard: `target/`, `*.lock` optional)
- [ ] Add `README.md` with project description, install instructions
  placeholder, and screenshot placeholder
- [ ] Verify `cargo build` succeeds with zero warnings

---

## Phase 2: Terminal Initialization and Teardown [x Completed]

Establish reliable terminal setup/teardown before drawing anything. This is
the foundation everything else builds on.

- [ ] Write `main.rs` startup sequence:
  - Enable raw mode via `crossterm`
  - Enter alternate screen
  - Hide cursor
  - Initialize ratatui `Terminal` with `CrosstermBackend`
- [ ] Implement a `cleanup()` function that restores terminal state (disable
  raw mode, leave alternate screen, show cursor)
- [ ] Register cleanup on panic via `std::panic::set_hook` so a crash never
  leaves the terminal broken
- [ ] Implement a simple render loop in `main.rs` that draws a placeholder
  frame (just the border) and exits cleanly on `q` or `Ctrl+C`
- [ ] Verify that after exiting, the terminal is fully restored (cursor
  visible, scrollback intact, no raw mode)

---

## Phase 3: Input Handling [x Completed]

Build the unified input abstraction before wiring up game logic.

- [ ] Define `GameInput` enum in `input.rs`
- [ ] Define `Direction` enum (`Up`, `Down`, `Left`, `Right`) with
  `opposite()` method
- [ ] Implement keyboard polling using `crossterm::event::poll` with zero
  timeout (non-blocking)
- [ ] Map keyboard events to `GameInput`:
  - Arrow keys + WASD → `Direction`
  - `p` / `Esc` → `Pause`
  - `q` / `Ctrl+C` → `Quit`
  - `Enter` / `Space` → `Confirm`
- [ ] Initialize `gilrs::Gilrs` in `input.rs`; detect and log connected
  gamepads
- [ ] Map D-pad events to `Direction`
- [ ] Map left analog stick to `Direction` with 0.5 deadzone threshold
- [ ] Map controller Start button → `Pause`, Select/Back → `Quit`, A/Cross →
  `Confirm`
- [ ] Add WSL guard: if `Platform::is_wsl()`, skip gilrs initialization
  entirely
- [ ] Add `--no-controller` CLI flag via `clap` that also skips gilrs
- [ ] Write unit tests for `Direction::opposite()` and direction-change
  validation (no 180° reversal)

---

## Phase 4: Game State and Logic [x Completed]

Pure logic with no rendering. This module must be fully unit-testable.

- [ ] Define `Position { x: i32, y: i32 }` with wrapping arithmetic helpers
- [ ] Implement `Snake` struct in `snake.rs`:
  - `VecDeque<Position>` body (front = head)
  - `Direction` current direction
  - `Direction` buffered next direction
  - `grow: bool` flag
  - `fn move_forward(&mut self, bounds: (u16, u16))`
  - `fn buffer_direction(&mut self, dir: Direction)` — rejects 180° reversals
  - `fn head(&self) -> Position`
  - `fn occupies(&self, pos: Position) -> bool`
- [ ] Implement `Food` struct in `food.rs`:
  - `fn spawn(rng, bounds, snake) -> Position` — guarantees no overlap with
    snake
  - Support for bonus food (different glyph, time-limited, higher score value)
- [ ] Implement `GameState` in `game.rs`:
  - Fields: `snake`, `food`, `score`, `speed_level`, `tick_count`, `status:
    GameStatus`
  - `GameStatus` enum: `Playing`, `Paused`, `GameOver`, `Victory`
  - `fn tick(&mut self)` — advance one game step: move snake, check
    collisions, check food, update score
  - `fn apply_input(&mut self, input: GameInput)` — handle direction and pause
    inputs
  - Collision detection: wall collision = game over, self collision = game
    over
  - Speed levels: tick interval decreases every N points (defined in
    `config.rs`)
- [ ] Write unit tests:
  - Snake grows after eating food
  - Snake collision with wall → `GameOver`
  - Snake collision with self → `GameOver`
  - Direction buffer rejects 180° reversal
  - Food never spawns on snake body
  - Score increments correctly

---

## Phase 5: Renderer [x Completed]

Translate game state to terminal output using ratatui. No game logic here.

- [ ] Implement `renderer.rs` with a single `fn render(frame, state,
  platform)` function
- [ ] Draw outer border using box-drawing characters `╔ ═ ╗ ║ ╚ ╝`
- [ ] Draw game grid — each logical cell is 2 terminal columns wide × 1 row
  tall
- [ ] Render snake body cells using `GLYPH_SNAKE_BODY` (`█`) with a green
  color style
- [ ] Render snake tail cell using `GLYPH_SNAKE_TAIL` (`▓`) with a slightly
  dimmer green
- [ ] Render snake head using directional glyph (`▲ ▼ ◀ ▶`) with bright green
  or yellow
- [ ] Render food using `GLYPH_FOOD` (`●`) in red
- [ ] Render bonus food using `GLYPH_FOOD_BONUS` (`★`) in gold with a blink
  style when time is running out
- [ ] Implement `hud.rs` — top or bottom bar showing:
  - Current score (left)
  - Speed level (center)
  - High score (right)
  - Controller connected indicator (Nerd Font gamepad glyph or `[PAD]`
    fallback)
  - WSL indicator if running under WSL
- [ ] Apply ratatui `Color` and `Modifier` (Bold, Dim) — use 256-color
  palette, not TrueColor, for broadest compatibility
- [ ] Verify rendering looks correct in: macOS Terminal, iTerm2, Ghostty,
  Windows Terminal

---

## Phase 6: Menus and Screens [x Completed]

- [ ] Implement `menu.rs` with the following screens, rendered as ratatui
  overlays/popups:
  - **Start screen**: game title (large Unicode block letters or a simple
    ASCII art title), `[Enter] Start`, `[Q] Quit`, high score display
  - **Pause screen**: `PAUSED` centered, `[P] Resume`, `[Q] Quit`
  - **Game over screen**: `GAME OVER`, final score, new high score indicator
    if applicable, `[Enter] Play Again`, `[Q] Quit`
- [ ] Wire screens into the main game loop via `GameStatus`
- [ ] Ensure all menu interactions work with both keyboard and controller

---

## Phase 7: High Score Persistence [x Completed]

- [ ] Implement `score.rs`:
  - `fn scores_path() -> PathBuf` — platform-correct path via `dirs` crate
  - `fn load_high_score() -> u32` — returns 0 if file missing or malformed
  - `fn save_high_score(score: u32)` — creates parent dirs if needed
- [ ] Integrate into game over flow: compare score, save if new high score,
  show indicator on game over screen
- [ ] Write unit test for score serialization round-trip

---

## Phase 8: CLI and Configuration [x Completed]

- [ ] Define `Cli` struct in `main.rs` using `clap` derive:
  - `--speed <1-5>` — starting speed level (default: 1)
  - `--width <N>` — grid width in logical cells (default: 40)
  - `--height <N>` — grid height in logical cells (default: 20)
  - `--no-controller` — disable gilrs initialization
  - `--no-color` — monochrome mode for terminals without color support
- [ ] Validate grid dimensions against current terminal size at startup; print
  helpful error and exit if terminal is too small
- [ ] Pass CLI config into `GameState` and `Renderer`

---

## Phase 9: CI and Release Pipeline [ ]

- [ ] Write `.github/workflows/ci.yml`:
  - Trigger on push and pull_request
  - Matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`
  - Steps: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
    `cargo build --release`
- [ ] Write `.github/workflows/release.yml`:
  - Trigger on tag push `v*`
  - Cross-compile using `cross` or GitHub-hosted runners:
    - `x86_64-unknown-linux-gnu`
    - `aarch64-unknown-linux-gnu`
    - `x86_64-apple-darwin`
    - `aarch64-apple-darwin`
    - `x86_64-pc-windows-msvc`
  - Package each binary as a `.tar.gz` (Linux/macOS) or `.zip` (Windows)
  - Create GitHub Release with binaries attached and auto-generated changelog
- [ ] Add `Cargo.toml` metadata: `description`, `license`, `homepage`,
  `repository`, `keywords`, `categories`
- [ ] Tag `v0.1.0` and verify the release pipeline produces downloadable
  binaries

---

## Phase 10: Polish and Extras [ ]

Nice-to-haves after the game is fully functional.

- [ ] Add sound? (Terminal bell on eat/die — `\x07`. Optional via `--bell`
  flag. Keep it subtle.)
- [ ] Bonus food that spawns periodically, blinks as its timer expires, and
  disappears if not eaten
- [ ] Speed ramp-up animation: brief flash on speed level increase
- [ ] Snake color gradient: head brightest, body dimming toward tail using
  256-color ramp
- [ ] Wall-wrap mode as an alternative to wall-collision death (`--wrap` flag)
- [ ] Publish to `crates.io`
- [ ] Add screenshot/demo GIF to `README.md` (record with `vhs` or
  `asciinema`)
- [ ] Write man page or `--help` text that is genuinely useful

---

## Definition of Done

A phase is complete when:

1. `cargo build --release` succeeds with zero warnings
2. `cargo clippy -- -D warnings` passes
3. `cargo test` passes
4. The feature works correctly in at least two of: macOS Terminal,
   iTerm2/Ghostty, Windows Terminal, Ubuntu terminal
5. No terminal state is leaked on exit or panic
