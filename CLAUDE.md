# Volt — GPU-Accelerated Terminal Emulator

## Project Overview
Volt is a fast, GPU-accelerated terminal emulator built in Rust. Forked from Rio Terminal (MIT),
rebranded and customized. macOS-focused but cross-platform capable.

## Architecture
- **8 workspace crates**: `sugarloaf`, `teletypewriter`, `corcovado`, `copa`, `rio-proc-macros`, `rio-backend`, `rio-window`, `frontends/rioterm`
- Internal crate names retain `rio-*` prefix to avoid breaking 178K lines of imports
- Binary name: `volt` (package name in frontends/rioterm/Cargo.toml)

## Key Technology
| Component | Library |
|---|---|
| GPU rendering | `wgpu` (Metal/Vulkan/DX/GL) via `sugarloaf` crate |
| Windowing | `rio-window` (Winit fork with macOS NSWindowTabGroup) |
| Text shaping | `skrifa` + `font-kit` |
| PTY | `teletypewriter` crate (forkpty on Unix) |
| VT parsing | Alacritty-derived parser in `rio-backend` |
| Config | TOML via `serde` + `toml` (~/.config/volt/config.toml) |
| Logging | `tracing` |

## Build & Run
```bash
cargo check --workspace       # Quick compile check
cargo clippy --workspace      # Lint
cargo test --workspace        # Run all tests
cargo build --release -p volt # Release build
cargo run -p volt             # Run the app
make dev                      # Run with Metal HUD
make dev-debug                # Run with debug logging
```

## Features (inherited from Rio)
- Tabs (native macOS tabs via NSWindowTabGroup)
- Split panes
- TOML config with hot-reload
- Themes
- Font ligatures
- Sixel / iTerm2 / Kitty image protocols
- RetroArch shader support
- True color (24-bit)
- Navigation modes: CollapsedTab, Breadcrumb, TopTab, BottomTab, NativeTab

## File Layout
```
volt/
  Cargo.toml              # Workspace root
  CLAUDE.md               # This file
  frontends/rioterm/      # Main application binary (builds as "volt")
  sugarloaf/              # GPU rendering engine (WGPU)
  teletypewriter/         # PTY/shell management
  rio-backend/            # Terminal logic, ANSI parsing, grid, config
  rio-window/             # Window management (Winit fork)
  rio-proc-macros/        # Procedural macros
  copa/                   # Clipboard and utilities
  corcovado/              # Event loop utilities
  misc/osx/Volt.app/      # macOS .app bundle template
```

## Code Conventions
- Rust edition 2021 (workspace default)
- `cargo clippy` must pass
- `cargo fmt` before every commit
- Logging: `tracing` macros, never `println!`
- Config lives at `~/.config/volt/config.toml`

## Origin
Forked from [Rio Terminal](https://github.com/raphamorim/rio) v0.2.37 (MIT license).
