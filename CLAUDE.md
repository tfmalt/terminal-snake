# CLAUDE.md — Snake Game (Rust)

This file provides context and guidance for Claude Code when working on this
project.

## Project Overview

A cross-platform, terminal-based Snake game written in Rust. The game targets
modern terminal emulators on Linux, macOS, and Windows (including WSL). Input
is supported via keyboard and optionally via game controller. Graphics use
Unicode block elements and Nerd Font glyphs — no ASCII fallback, no color
emoji dependency.

## Technology Stack

- `ratatui`: terminal UI rendering framework.
- `crossterm`: cross-platform raw mode, keyboard input, cursor control.
- `gilrs`: game controller input (disabled automatically under WSL).
- `unicode-width`: correct cell-width calculation for Unicode glyphs.
- `serde` + `serde_json`: high score persistence.
- `clap`: CLI argument parsing (for flags like `--speed`).
- `rand`: food placement randomization.

## Project Structure

```text
snake/
├── CLAUDE.md
├── PLAN.md
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .github/
│   └── workflows/
│       ├── ci.yml          # Build + test on all platforms
│       └── release.yml     # Cross-compile and publish binaries on tag
├── assets/
│   └── (none required — all graphics are Unicode glyphs in source)
└── src/
    ├── main.rs             # Entry point, CLI parsing, top-level game loop
    ├── game.rs             # Game state, tick logic, collision detection
    ├── snake.rs            # Snake data structure and movement
    ├── food.rs             # Food spawning logic
    ├── input.rs            # Unified input handler (keyboard + controller)
    ├── renderer.rs         # Ratatui rendering: grid, HUD, menus
    ├── ui/
    │   ├── mod.rs
    │   ├── menu.rs         # Start screen, pause screen, game over screen
    │   └── hud.rs          # Score, speed level, lives display
    ├── config.rs           # Game configuration, constants, glyph definitions
    ├── score.rs            # High score loading/saving (JSON, ~/.local/share)
    └── platform.rs         # Runtime platform detection (WSL, terminal caps)
```

## Architecture: Key Principles

### Game Loop

The main loop runs at a fixed tick rate driven by `std::thread::sleep`. The
tick rate increases as the score grows (speed levels). Each tick:

1. Poll input (keyboard via crossterm, controller via gilrs if available)
2. Update game state (move snake, check collisions, spawn food)
3. Render frame (ratatui)

Input is decoupled from the tick rate — direction changes are buffered and
only one is consumed per tick to prevent 180° reversals from rapid input.

### Input Abstraction

All input sources (keyboard, D-pad, analog stick) funnel into a single
`GameInput` enum:

```rust
pub enum GameInput {
    Direction(Direction),
    Pause,
    Quit,
    Confirm,
}
```

The `input.rs` module is the only place that knows about `crossterm` or
`gilrs`.

### Grid Coordinate System

The game grid uses a logical coordinate system independent of terminal cell
size. Each logical cell maps to **2 terminal columns × 1 row** to account for
the typical 1:2 aspect ratio of terminal characters, ensuring the grid appears
square. All glyphs used for the grid must be single-width Unicode (not
double-width emoji).

### Glyph Palette

Defined in `config.rs` as constants. Do not scatter glyph literals across the
codebase.

```rust
pub const GLYPH_SNAKE_HEAD_UP: &str    = "▲";
pub const GLYPH_SNAKE_HEAD_DOWN: &str  = "▼";
pub const GLYPH_SNAKE_HEAD_LEFT: &str  = "◀";
pub const GLYPH_SNAKE_HEAD_RIGHT: &str = "▶";
pub const GLYPH_SNAKE_BODY: &str       = "█";
pub const GLYPH_SNAKE_TAIL: &str       = "▓";
pub const GLYPH_FOOD: &str             = "●";
pub const GLYPH_FOOD_BONUS: &str       = "★";
pub const GLYPH_EMPTY: &str            = " ";
// Border: ╔ ═ ╗ ║ ╝ ╚
```

### Platform Detection (WSL)

`platform.rs` detects WSL at startup by checking `/proc/version` for
"microsoft" (case-insensitive). If WSL is detected, controller support is
disabled and a note is shown in the HUD. No panics, no user-facing errors — it
degrades silently.

### High Score Persistence

Scores are saved to a platform-appropriate path:
- Linux/WSL: `~/.local/share/terminal-snake/scores.json`
- macOS: `~/Library/Application Support/terminal-snake/scores.json`
- Windows: `%APPDATA%\terminal-snake\scores.json`

Use the `dirs` crate to resolve these paths at runtime.

## Versioning Policy

Follow [Semantic Versioning](https://semver.org/) with the scheme `MAJOR.MINOR.PATCH`.
The project is pre-1.0, so MAJOR stays at `0`.

- **Patch** (`0.y.Z`): bump for every bug fix, refactor, test addition, internal
  cleanup, or dependency update. Update `version` in `Cargo.toml` and include the
  bump in the same commit as the change.
- **Minor** (`0.Y.0`): bump whenever a change is visible to the player — new or
  removed game mechanics, scoring changes, UI layout changes, new screens, new
  glyphs, or altered controls. Reset patch to `0`.
- **Major** (`X.0.0`): reserved for a stable 1.0 release.

When in doubt, prefer a minor bump. Always update `Cargo.toml` in the same
commit as the change that triggered it — never as a separate "version bump"
commit.

## Coding Conventions

- Use `thiserror` for error types; no `unwrap()` in library code paths.
- Prefer `Result<T, E>` returns throughout; only `unwrap()` in `main()` at
  startup after clear diagnostic messages.
- All public types and functions must have doc comments.
- Run `cargo clippy -- -D warnings` before committing — CI enforces this.
- Format with `cargo fmt` (rustfmt defaults).
- Keep `renderer.rs` free of game logic — it only reads state, never mutates
  it.

## Building

```bash
cargo build --release
cargo run
cargo test
cargo clippy -- -D warnings
```

## Distribution

Binaries are published via GitHub Actions on version tags (`v*`). Targets:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Users with Rust installed can also use:

```bash
cargo install --git https://github.com/YOUR_ORG/snake
```

Once published to crates.io:

```bash
cargo install snake
```

## Testing Strategy

- Unit tests in each module for game logic (collision, direction reversal
  prevention, food spawn, score calculation).
- Integration test that runs a deterministic sequence of inputs against the
  game state and asserts expected output.
- No tests for rendering — visual output is verified manually.
- CI runs on `ubuntu-latest`, `macos-latest`, `windows-latest`.
