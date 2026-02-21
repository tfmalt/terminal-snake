# term-snake

A cross-platform, terminal-based Snake game in Rust.

This project is currently in the planning/scaffolding stage. The architecture,
module layout, and phased implementation plan are documented in:

- `CLAUDE.md` (architecture + runtime design)
- `PLAN.md` (incremental implementation phases)
- `AGENTS.md` (agent workflow + coding standards)

## Goals

- Build a polished terminal Snake game for Linux, macOS, Windows, and WSL.
- Support keyboard input and optional game controller input.
- Use Unicode glyph rendering in terminal (no color emoji dependency).
- Keep core game logic deterministic and thoroughly unit-tested.
- Use this codebase as a Rust learning project while shipping a real app.

## Planned Tech Stack

- `ratatui` for terminal UI rendering
- `crossterm` for terminal input/raw mode
- `gilrs` for game controller support
- `clap` for CLI argument parsing
- `serde` + `serde_json` for high score persistence
- `thiserror` for error types

## Planned Module Layout

```text
src/
  main.rs
  game.rs
  snake.rs
  food.rs
  input.rs
  renderer.rs
  config.rs
  score.rs
  platform.rs
  ui/
    mod.rs
    menu.rs
    hud.rs
```

## Getting Started (Once Scaffold Exists)

From the repository root:

```bash
cargo build
cargo run
```

Useful quality checks:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Run a single test by name:

```bash
cargo test direction_buffer_rejects_reverse -- --exact
```

## Install (Placeholder)

Installation instructions will be added once the first playable release is
available.

## Screenshot (Placeholder)

Screenshot/GIF to be added after renderer and gameplay phases are complete.

## Development Notes

- Keep rendering and gameplay logic separate (`renderer` reads state only).
- Keep input backend details isolated in `input.rs`.
- Centralize glyph constants in `config.rs`.
- Prefer small, incremental changes with focused tests.

## License

TBD.
