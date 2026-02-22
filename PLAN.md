# PLAN.md — Snake Game Development Plan

Active roadmap after completing phases 1-8.

---

## Phase 9: Native Windows Terminal Support [ ]

Make native Windows (`x86_64-pc-windows-msvc`) a first-class runtime target for
PowerShell + Windows Terminal.

- [ ] Add robust terminal lifecycle guard in `main.rs` so cleanup always runs
  on normal exit, early `Err` returns, and panic
- [ ] Verify alternate screen/raw mode/cursor restoration on all exit paths in
  PowerShell (including Ctrl+C and startup failures)
- [ ] Add resize-event handling policy for Windows Terminal
- [ ] Validate glyph rendering in Windows Terminal (Cascadia Mono/Code): `▀`,
  `▄`, `█`, directional/menu glyphs
- [ ] Add fallback strategy for poor glyph environments and document behavior
- [ ] Confirm color and style behavior in PowerShell + Windows Terminal
- [ ] Validate persistence paths on Windows: score/theme file creation, parent
  directory creation, malformed/missing-file recovery
- [ ] Update docs to explicitly state Windows support scope, expected fonts,
  known limitations, and persistence path (`LOCALAPPDATA`)
- [ ] Add manual Windows smoke-test checklist to docs

---

## Phase 10: CI and Release Pipeline [ ]

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

## Phase 11: Polish and Extras [ ]

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
6. For Windows-targeted phases, behavior is verified in PowerShell on Windows
   Terminal
