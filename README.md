# Volt - GPU-Accelerated Terminal Emulator

A fast, modern terminal emulator built in Rust with native Metal/WGPU rendering. Designed for developers who want speed, power, and a beautiful interface.

## Features

### Core Terminal
- **GPU-accelerated rendering** via WGPU (Metal on macOS, Vulkan/DX on other platforms)
- **Color-coded tabs** with click, rename (double-click), scroll, Cmd+1-9 jump
- **Split panes** — vertical (Cmd+D) and horizontal splits with drag-to-resize dividers
- **Pane zoom** (Cmd+Shift+Enter) and **broadcast mode** (Cmd+Shift+B)
- **True color** (24-bit), font ligatures, Sixel/iTerm2/Kitty image protocols
- **TOML config** with hot-reload (~/.config/volt/config.toml)

### Interactive Settings (Cmd+,)
- 40+ configurable options organized by category
- Navigate with arrow keys, edit inline, toggle booleans with Enter
- Native image picker for background images with adjustable opacity
- Import config from Alacritty, Ghostty, or Kitty

### Help System (Cmd+?)
- 4 categories: **Shortcuts**, **Features**, **Actions**, **Commands**
- Actions launch features directly with Enter
- Commands insert slash commands into the terminal

### Session History (Cmd+Shift+H)
- Records all commands with exit codes, duration, timestamps
- Arrow keys to navigate, Enter to paste command into terminal
- `b` to bookmark, `e` to export session
- Persists across restarts (~/.config/volt/history.json)

### Environment Inspector (Cmd+Shift+E)
- All environment variables grouped by category
- Secrets auto-masked (API keys, tokens, passwords)
- Enter copies KEY=VALUE to clipboard and terminal

### Bookmarks (Cmd+Shift+K)
- Save important commands with tags and exit codes
- Enter to paste, `d` to delete
- Persistent storage (~/.config/volt/bookmarks.json)

### Connections Manager (status bar > Connect)
- SSH, MySQL, PostgreSQL, Redis, Kubernetes, Docker connections
- Create new connections with `n`, delete with `d`, edit config with `e`
- Config file: ~/.config/volt/connections.toml

### Tmux Integration (Cmd+Shift+M)
- Sidebar list of tmux sessions with attach/detach status
- Enter to attach, `d` to detach, `x` to kill, `n` to create, `r` to rename

### Layout Presets (Cmd+Shift+L)
- **Side by Side** — two equal vertical panes
- **Top / Bottom** — two equal horizontal panes
- **Dev** — editor left, two terminals right
- **Quad** — four equal panes (2x2 grid)
- **Three Column** — three equal vertical panes
- **Fullscreen** — single maximized pane

### Slash Commands (Cmd+Shift+P)
- 20+ built-in commands organized by category
- Navigate and select with arrow keys, Enter to insert
- Categories: Navigation, Appearance, Tools, Session, Debug

### Session Export (Cmd+Shift+X)
- Export as **Asciinema** (.cast) — playable recordings
- Export as **Plain Text** (.txt) — command history
- Export as **HTML** (.html) — styled terminal output with embedded player
- Export as **JSON** (.json) — structured session data

### Session Sharing (Cmd+Shift+S)
- Host a terminal session for others to view
- Connect to a shared session

### Time Travel (Cmd+Shift+T)
- Browse command timeline with sidebar navigation
- Detail panel shows command, status, duration, directory, output preview
- Enter to replay, `c` to copy command

### AI Assistant (Cmd+Shift+I)
- Opens Claude Code in a split pane
- Automatic tab rename to "AI"

### Safety Features
- **Destructive command detection** — warns on `rm -rf`, `git push -f`, etc. (20 patterns)
- **Multi-line paste confirmation**
- **Shell integration** auto-installs on launch
- macOS notifications on long-running commands
- Dock badge on terminal bell

### Background Images
- Set via Settings > Window > Background Image (native file picker)
- Default 40% opacity for readability (configurable)
- Remove with Backspace in settings

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+, | Settings |
| Cmd+? | Help |
| Cmd+Shift+H | Session History |
| Cmd+Shift+E | Environment Inspector |
| Cmd+Shift+K | Bookmarks |
| Cmd+Shift+P | Slash Commands |
| Cmd+Shift+L | Layout Presets |
| Cmd+Shift+X | Session Export |
| Cmd+Shift+S | Session Sharing |
| Cmd+Shift+T | Time Travel |
| Cmd+Shift+M | Tmux Picker |
| Cmd+Shift+I | AI Assistant |
| Cmd+T | New tab |
| Cmd+W | Close tab/pane |
| Cmd+D | Split right |
| Cmd+Shift+Enter | Zoom pane |
| Cmd+Shift+B | Broadcast to all panes |
| Cmd+1-9 | Jump to tab |
| Ctrl+` | Quake mode |

## Status Bar

The bottom status bar provides quick access to all features:

```
Volt                    AI | History | Env | Bookmarks | Connect | Layout | Export | Share  tmux
```

Each item is color-coded and clickable.

## Installation

### Build from source

```bash
git clone https://github.com/bhutianimukul/volt.git
cd volt
cargo build --release -p volt
```

### Run

```bash
cargo run -p volt
```

### macOS .app Bundle

```bash
# Copy to Applications
cp -r misc/osx/Volt.app /Applications/
# Copy the binary
cp target/release/volt /Applications/Volt.app/Contents/MacOS/
```

## Configuration

Config file: `~/.config/volt/config.toml`

```toml
[fonts]
size = 14.0

[window]
opacity = 1.0

[window.background-image]
path = "/path/to/image.png"
opacity = 0.4

[navigation]
mode = "TopTab"
```

Import from other terminals:
- Open Settings (Cmd+,) → press `i` to auto-import from Alacritty, Ghostty, or Kitty

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| GPU Rendering | WGPU (Metal/Vulkan/DX/GL) |
| Text Shaping | skrifa + font-kit |
| VT Parsing | Alacritty-derived parser |
| Config | TOML via serde |
| Logging | tracing |

## License

MIT
