# AGENTS.md - Instructions for Agentic Coding Tools

This file is the operating manual for coding agents working in this
repository. It is based on the current project docs (`CLAUDE.md`, `PLAN.md`)
and Rust best practices for this codebase.

## Scope and Source of Truth

1. Follow this file for day-to-day implementation behavior.
2. Treat `CLAUDE.md` as the architecture and product spec.
3. Treat `PLAN.md` as the phased execution plan.
4. If these files conflict, prefer `CLAUDE.md` for architecture and runtime
   rules.
5. Keep changes aligned with the intended module layout in `src/`.

## Repository State (Important)

1. The repository currently contains planning/docs, not full Rust sources yet.
2. Implementations should follow the documented target structure.
3. Do not invent alternate architecture unless explicitly requested.

## Pedagogical Goal (Rust Learning First)

This repository has an explicit dual goal: ship the project and help the user
learn Rust deeply while building it.

When making changes, agents must act like a tutor pairing with a colleague who
is new to Rust and this stack.

- Prefer small, incremental steps over large opaque rewrites.
- Explain Rust-specific choices in plain language (ownership, borrowing,
  lifetimes, enums, pattern matching, error propagation).
- When introducing an idiom, briefly state why it is idiomatic in Rust and
  what alternative was considered.
- Tie explanations to concrete code locations and behavior, not abstract
  theory.
- Surface tradeoffs (readability, performance, safety, ergonomics) in 1-2
  lines.
- Encourage learning-oriented follow-ups (focused tests, tiny refactors,
  "try this next" tasks).
- For non-trivial edits, include a short "what changed" and "why this Rust
  approach" explanation in agent responses.
- Avoid assuming prior familiarity with crates like `ratatui`, `crossterm`,
  or common Rust patterns used by them.

Teaching style expectations:

1. Be precise but approachable; define jargon once before using it repeatedly.
2. Favor examples over long prose when clarifying a concept.
3. Keep momentum: teach through the current task instead of unrelated detours.
4. If a change is complex, propose a staged plan the learner can review
   stepwise.

## Cursor/Copilot Rule Files

- Checked `.cursor/rules/`: not present.
- Checked `.cursorrules`: not present.
- Checked `.github/copilot-instructions.md`: not present.
- If any of these are later added, merge their constraints into this file.

## Build, Lint, and Test Commands

Run from repository root.

### Build

- Debug build: `cargo build`
- Release build: `cargo build --release`
- Run game: `cargo run`
- Run with CLI args: `cargo run -- --speed 2 --debug`

### Format and Lint

- Format all code: `cargo fmt`
- Check formatting only: `cargo fmt --check`
- Lint (strict): `cargo clippy -- -D warnings`
- Lint tests too: `cargo clippy --tests -- -D warnings`

### Tests (Full and Single Test)

- Run all tests: `cargo test`
- Run tests with output: `cargo test -- --nocapture`
- Run a specific unit test by name substring:
  - `cargo test direction_buffer_rejects_reverse`
- Run a specific test exactly:
  - `cargo test direction_buffer_rejects_reverse -- --exact`
- Run tests in one module/file (name filter):
  - `cargo test snake::tests::`
- Run one integration test target file:
  - `cargo test --test deterministic_sequence`
- Run one test inside an integration target:
  - `cargo test --test deterministic_sequence stepwise_food_collection`
- Short backtrace for failures: `RUST_BACKTRACE=1 cargo test`

### CI-Parity Command Set

Use this before finalizing substantial changes:

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. `cargo build --release`

## Code Style Guidelines

### Formatting and Layout

- Always use rustfmt defaults (`cargo fmt`).
- Keep functions focused; split long functions by responsibility.
- Prefer early returns over deeply nested conditionals.
- Keep renderer code declarative and state-read-only.

### Imports

- Group imports by standard library, external crates, then local modules.
- Keep import lists stable and minimal; remove unused imports.
- Avoid wildcard imports (`use foo::*`) except in tests when clearly helpful.
- Use explicit paths for clarity in core logic modules.

### Naming Conventions

- Types/enums/traits: `PascalCase`.
- Functions/modules/variables: `snake_case`.
- Constants/statics: `SCREAMING_SNAKE_CASE`.
- Test names should describe behavior, not implementation detail.
- Prefer domain-specific names (`tick_interval_ms`) over generic names
  (`value`).

### Types and Ownership

- Prefer explicit domain types/structs over loose tuples in public APIs.
- Use `u16`/`u32`/`i32` intentionally for coordinates and scores as
  documented.
- Borrow where possible (`&T`, `&mut T`) and clone only when needed.
- Keep state mutations centralized in game-state methods.
- Derive traits intentionally (`Debug`, `Clone`, `Copy`, `Eq`, `PartialEq`)
  where useful.

### Error Handling

- Use `Result<T, E>` for fallible operations.
- Use `thiserror` for custom error enums.
- No `unwrap()` in library/game modules.
- `expect()` is acceptable only when failure is unrecoverable and message is
  clear.
- If panics are possible at startup, surface actionable diagnostics.
- Gracefully degrade optional features on platform differences when needed.

### Comments and Documentation

- Add doc comments for all public types/functions.
- Keep inline comments for non-obvious intent only.
- Do not restate what code already expresses clearly.
- Document invariants and coordinate assumptions near their definitions.

### Testing Standards

- Prefer unit tests for pure game logic and deterministic behavior.
- Add regression tests for bug fixes.
- Avoid testing rendering output snapshots unless explicitly requested.
- Keep tests deterministic; seed RNG or inject deterministic providers when
  needed.
- Validate direction buffering and collision behavior with focused tests.

### Architecture Constraints (From Spec)

- `crossterm` specifics must be isolated to `input.rs` (event mapping) and
  `terminal_runtime.rs` (raw mode + alternate screen lifecycle).
- `renderer.rs` must not mutate gameplay state.
- Define glyph constants in `config.rs`; do not scatter glyph literals.
- Respect logical grid mapping (2 terminal columns x 1 row per logical cell).
- Keep platform detection in `platform.rs` and degrade silently on WSL.

### Platform and Persistence Rules

- Use `dirs` crate for score file paths.
- Ensure score save creates parent directories when missing.
- Handle missing/malformed score files without crashing.
- Keep paths platform-correct (Linux/WSL, macOS, Windows).

## Change Management Expectations

- Keep diffs small and focused on requested behavior.
- Preserve existing conventions over personal preference.
- Do not add new dependencies without clear need.
- Update docs when behavior or CLI flags change.
- If introducing a new module, wire it via explicit `mod`/`use` structure.

## Pre-Completion Checklist for Agents

1. Code formatted (`cargo fmt` or `cargo fmt --check`).
2. Lint passes (`cargo clippy -- -D warnings`).
3. Relevant tests pass (at least focused tests, ideally full `cargo test`).
4. No forbidden `unwrap()` usage in non-startup paths.
5. Public APIs documented.
6. Architectural boundaries preserved (`input` vs `game` vs `renderer`).
