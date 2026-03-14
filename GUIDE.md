# Volt Terminal — Feature Guide

## Getting Started

```bash
cargo build --release -p volt
cargo run -p volt
```

Config file: `~/.config/volt/config.toml`

---

## Keyboard Shortcuts

### Tab Management

| Shortcut | Action |
|----------|--------|
| Cmd+T | New tab |
| Cmd+W | Close current tab/split |
| Cmd+1-9 | Jump to tab N (Cmd+9 = last tab) |
| Cmd+Shift+] | Next tab |
| Cmd+Shift+[ | Previous tab |
| Double-click tab | Rename tab |
| Cmd+Shift+R | Rename current tab |
| Click tab | Switch to tab |
| Scroll on tab bar | Scroll tabs horizontally |

### Split Panes

| Shortcut | Action |
|----------|--------|
| Cmd+D | Split right |
| Cmd+Shift+D | Split down |
| Cmd+Shift+Enter | Toggle zoom on current pane |
| Cmd+Shift+B | Broadcast input to all panes |
| Ctrl+Cmd+Arrow | Resize pane (keyboard) |
| Drag divider | Resize pane (mouse) |

### General

| Shortcut | Action |
|----------|--------|
| Cmd+, | Open settings viewer |
| Cmd+K | Clear scrollback |
| Cmd+L | Clear screen |
| Cmd+=/-/0 | Zoom in/out/reset |
| Cmd+Enter | Toggle fullscreen |
| Cmd+N | New window |
| Cmd+Q | Quit |
| Cmd+V | Paste |
| Cmd+C | Copy / Send SIGINT |

---

## Features

### Color-Coded Tabs

Every new tab gets a unique random accent color. The full tab bar is visible at all times — each tab is a solid color block so you can instantly identify tabs by color from anywhere. The active tab has a white bottom indicator strip.

Tabs auto-size based on their label — short number labels are compact, renamed tabs expand to fit (max 20 characters with ".." truncation).

### Command Consequences Preview

Volt automatically detects destructive commands before execution:

- `rm -rf` — shows file count and total size
- `git push --force` — warns about overwriting remote history
- `kubectl delete` — warns about Kubernetes resource deletion
- `chmod -R` — warns about recursive permission changes
- `docker prune/rm` — warns about container/image removal
- SQL `DROP TABLE/DATABASE/TRUNCATE` — warns about data destruction

When detected, a native macOS dialog appears with severity level, description, and details. Choose **"Execute Anyway"** or **"Cancel"**.

**Bypass:** Prefix any command with `!` to skip the check: `!rm -rf /tmp/junk`

### Settings Viewer (Cmd+,)

Opens a dedicated settings view showing your current configuration organized by category:

- **Font** — family, size
- **Window** — opacity, background
- **Navigation** — mode, hide if single
- **Colors** — background, foreground, cursor
- **Shell** — program, args
- **Developer** — log level, log file

Press **Escape** to return to the terminal. Edit `~/.config/volt/config.toml` to change settings.

### Pane Zoom

Press `Cmd+Shift+Enter` to zoom the current pane to fill the entire window, hiding all other panes. Press again to restore the split layout. Useful for focusing on one pane temporarily.

### Broadcast Input

Press `Cmd+Shift+B` to toggle broadcast mode. When active, everything you type is sent to ALL panes simultaneously. Press again to disable. Great for running the same command on multiple servers.

### Mouse Divider Dragging

When you have split panes, hover over the divider between panes — the cursor changes to a resize indicator. Click and drag to resize panes freely.

### Tab Rename

Double-click any tab or press `Cmd+Shift+R` to rename it. A native dialog appears where you can type a custom name. Custom names persist across title updates.

---

## Scaffolded Features (Data Models Ready)

These features have their core logic implemented and tested. They need UI wiring to become fully interactive.

### OSC 133 Shell Integration (blocks.rs)

Block model for tracking individual commands:

- Detects prompt start, command start, output start, command finish
- Tracks exit code and duration per command
- Foundation for block-based UI (collapsible command blocks)

### Structured Output Detection (structured_output.rs)

Detects and parses:

- **JSON** — full recursive descent parser, produces collapsible tree
- **Tables** — detects aligned columns (kubectl, docker, ps output)
- **Key-Value** — detects `key: value` and `key = value` patterns

### Retroactive Piping (retroactive_pipe.rs)

Pipe any previous command's captured output through a filter:

```
/pipe grep error   # Search old output for "error"
/pipe jq '.name'   # Extract field from JSON output
/pipe wc -l        # Count lines
```

Smart filter suggestions based on output content.

### Session Time-Travel (time_travel.rs)

Records all commands with timestamp, exit code, duration, and output preview:

- Search history: find commands matching a query
- Find failed commands (exit code != 0)
- Export session as text

### Shell Intelligence (shell_intelligence.rs)

- **Project detection**: Rust (Cargo.toml), Node (package.json), Python, Go, Ruby, Java, Docker, Terraform
- **Git branch detection**: reads `.git/HEAD`
- **Secret scanning**: detects API keys (AWS, GitHub, Stripe, Slack, OpenAI) before execution
- **Smart suggestions**: context-aware commands per project type

### Slash Command System (slash_commands.rs)

20 built-in commands with fuzzy matching:

- `/split`, `/zoom`, `/tab`, `/close` — Navigation
- `/theme`, `/font`, `/opacity`, `/settings` — Appearance
- `/undo`, `/pipe`, `/test`, `/debug` — Tools
- `/search`, `/history`, `/bookmark`, `/share`, `/notify` — Session
- `/sandbox`, `/ai`, `/layout` — Debug

### Command-Level Undo (undo.rs)

APFS clonefile checkpoints for zero-cost filesystem snapshots:

- Creates snapshot before destructive commands
- `volt undo` or `/undo` restores files
- Named checkpoints: `/undo name "before migration"`
- Rolling window: 50 checkpoints max

### Tmux CC Mode Integration (tmux_cc.rs)

Connect to tmux via Control Center mode:

- tmux windows map to Volt tabs
- tmux panes map to Volt splits
- Session browser: list/attach/detach
- Notification parsing for window/pane lifecycle

---

## Inherited from Rio Terminal

Volt is built on [Rio Terminal](https://github.com/raphamorim/rio) v0.2.37 (MIT license), inheriting:

- **WGPU rendering** — GPU-accelerated via Metal (macOS), Vulkan, DirectX
- **Font ligatures** — full OpenType support via skrifa
- **True color** — 24-bit color (16 million colors)
- **Image protocols** — Sixel, iTerm2, Kitty
- **TOML configuration** — with hot-reload on file change
- **Cross-platform** — macOS, Linux (X11/Wayland), Windows
- **Kitty keyboard protocol** — full mode stack support
- **Vi mode** — scrollback navigation
- **URL detection** — clickable hyperlinks
- **Color automation** — per-program tab colors

---

## Configuration

Config file: `~/.config/volt/config.toml`

Example:

```toml
[fonts]
family = "JetBrains Mono"
size = 14

[window]
opacity = 1.0

[navigation]
mode = "TopTab"
hide-if-single = false

[colors]
background = "#0F0D0E"
foreground = "#F9F4DA"

[shell]
program = "/bin/zsh"
args = ["--login"]
```

---

## Building

```bash
# Development
cargo run -p volt

# Release build
cargo build --release -p volt

# Run tests
cargo test -p volt
```

---

## Architecture

```
frontends/rioterm/     — Main application (binary: volt)
  src/
    main.rs            — Entry point, config loading
    application.rs     — Event loop, window management
    screen/            — Terminal screen rendering
    context/           — Pane/tab/grid management, blocks, titles
    renderer/          — Navigation bar, text rendering
    consequences.rs    — Destructive command detection
    structured_output.rs — JSON/table/KV parsing
    retroactive_pipe.rs  — Output re-piping
    time_travel.rs     — Session recording
    shell_intelligence.rs — Project/git/secret detection
    slash_commands.rs   — Built-in command system
    undo.rs            — APFS checkpoint undo
    tmux_cc.rs         — Tmux CC mode integration

sugarloaf/             — GPU rendering engine (WGPU)
rio-backend/           — Terminal logic, ANSI parsing, config
rio-window/            — Window management (Winit fork)
teletypewriter/        — PTY/shell management
```
