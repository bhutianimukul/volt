# Volt — Rust Terminal Emulator

## Project Overview
Volt is a fast, lightweight terminal emulator in Rust with native Metal rendering on macOS.
macOS-only. No cross-platform abstractions.

## Architecture
- **4 crates** (v0.1): `volt-core`, `volt-pty`, `volt-renderer`, `volt-app`
- Dependency flow: `volt-app` → `volt-renderer` → `volt-core`, `volt-app` → `volt-pty`
- `volt-core` has zero platform dependencies (pure terminal state machine)

## Key Technology Decisions — DO NOT DEVIATE
| Component | Use | Do NOT use |
|---|---|---|
| Metal bindings | `objc2-metal` | `metal-rs` (deprecated Dec 2025) |
| Windowing | `objc2-app-kit` (direct AppKit) | `winit` (Ghostty abandoned it) |
| Text pipeline | `cosmic-text` (fontdb + harfrust + swash) | Separate rustybuzz/swash/font-kit |
| PTY | Raw `forkpty(3)` via `libc` + `mio` | `portable-pty` |
| Inter-thread | `crossbeam-channel` (unbounded for PTY→parser) | `std::sync::mpsc` |
| VT parsing | `vte` crate (we implement `Perform`) | Custom parser |
| Config format | TOML via `serde` + `toml` | YAML, JSON, or Lua config |

## Threading Model
- **Main thread**: macOS event loop, input dispatch, Metal rendering
- **Parser thread** (per pane): VT parsing + grid mutation on dedicated thread
  - Double-buffered grid: parser writes "back", renderer reads "front", atomic swap at vsync
- **PTY reader thread** (per pane): blocks on `read()`, posts to parser via crossbeam
- **CAMetalDisplayLink**: fires at vsync, triggers grid swap + render on main
- **Worker pool**: consequence analysis, structured output — NEVER on main thread

## Performance Rules
- Parse eagerly, render lazily. Drain entire PTY buffer before rendering.
- When PTY buffer > 64KB in one drain: fast-forward (skip intermediate grid states)
- Glyph atlas starts at 2048x2048 (NOT 1024). LRU eviction when full.
- Damage tracking is scroll-aware: scroll shifts instance buffer via memcpy, only new rows rebuild
- Target: <5ms input latency, <50ms cold start, >500 MB/s `cat` throughput, 120Hz ProMotion

## Code Conventions
- Rust edition 2024
- `cargo clippy` must pass with no warnings
- `cargo fmt` before every commit
- Error types: `thiserror` for library crates, `anyhow` only in `volt-app` if needed
- Logging: `tracing` macros (`info!`, `debug!`, `warn!`, `error!`), never `println!`
- Unsafe code: minimize, always document safety invariants in `// SAFETY:` comments
- Tests: unit tests in each module, integration tests in `tests/` directories

## Build & Test
```bash
cargo check                    # Quick compile check
cargo clippy --workspace       # Lint (must be clean)
cargo test --workspace         # Run all tests
cargo build --release          # Release build
cargo run -p volt-app          # Run the app
```

## CI
GitHub Actions runs on every PR: `cargo check`, `clippy`, `test`, `cargo audit`, `cargo deny`.

## File Layout
```
volt/
  Cargo.toml                   # Workspace root
  CLAUDE.md                    # This file
  crates/
    volt-core/                 # Grid, cells, VT parser, scrollback, selection, damage
    volt-pty/                  # PTY via forkpty, reader thread, signals
    volt-renderer/             # Metal pipeline, glyph atlas, text shaping
    volt-app/                  # AppKit shell, event loop, config, window management
  resources/
    Volt.app/                  # .app bundle skeleton
  .github/workflows/ci.yml    # CI pipeline
```

## Reference Codebases
- **Alacritty**: renderer, grid model, VT handling, atlas packing
- **Ghostty**: libghostty architecture, native macOS integration, shell integration
- **iTerm2**: shell integration (OSC 133), tmux CC mode, session management
- **Kitty**: keyboard protocol spec, graphics protocol
- **WezTerm**: multiplexer, font handling, portable-pty patterns
