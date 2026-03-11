# Volt

A fast, lightweight terminal emulator with Metal rendering for macOS.

**Status:** Early development (v0.1.0-alpha)

## Features (Planned)

- Native Metal rendering with 120Hz ProMotion support
- Direct AppKit integration (native tabs, fullscreen, IME)
- Command consequences preview
- Structured output rendering
- Command-level undo via APFS snapshots
- Built-in AI agent pane
- Block-based terminal model with shell integration

## Building

Requires Rust 1.85+ and macOS 14+.

```bash
cargo build --release
cargo run -p volt-app
```

## License

MIT
