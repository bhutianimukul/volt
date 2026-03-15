# Volt Terminal — Feature Guide

## Getting Started

```bash
cargo build --release -p volt
cargo run -p volt
```

Config file: `~/.config/volt/config.toml`

---

## What Works RIGHT NOW (try these!)

### 1. Color-Coded Tabs
Open the terminal and press **Cmd+T** a few times. Each new tab gets a unique random color. You can instantly identify which tab is which by its color.

### 2. Click Tabs to Switch
**Click any tab** in the top bar to switch to it. No keyboard shortcut needed.

### 3. Rename Tabs
**Double-click** any tab to rename it. A dialog pops up — type a name and hit OK. Or press **Cmd+Shift+R**.

### 4. Jump to Tab by Number
Press **Cmd+1** through **Cmd+9** to jump directly to that tab. Cmd+9 goes to the last tab.

### 5. Split Panes
- **Cmd+D** — split the current pane to the right
- **Cmd+Shift+D** — split the current pane downward (check your config for this binding)
- **Drag the divider** between panes with your mouse to resize them
- **Cmd+Shift+Enter** — zoom the current pane to fill the whole window (press again to unzoom)

### 6. Broadcast Input
Press **Cmd+Shift+B** to toggle broadcast mode. When active, everything you type goes to ALL panes at once. Great for running the same command on multiple servers. Press again to disable.

### 7. Destructive Command Warning
Try typing `rm -rf /tmp/test` and press Enter. Volt will show a warning dialog:
- Shows what the command will do
- Shows severity level (Warning/Danger)
- You can choose "Execute Anyway" or "Cancel"

Also catches: `git push --force`, `kubectl delete`, `chmod -R`, `docker prune`, `DROP TABLE`, `terraform destroy`, `git reset --hard`, `dd`, `mkfs`, `kill -9`, `sudo rm`, and more (20 patterns total).

**Bypass:** Prefix with `!` to skip: `!rm -rf /tmp/junk`

### 8. Settings Viewer
Press **Cmd+,** to open the settings viewer. Shows all your current config values organized by category. Press **Escape** to go back to the terminal.

### 9. Tab Scrolling
When you have lots of tabs, **scroll your mouse wheel/trackpad** over the tab bar to scroll horizontally through them.

### 10. Block Navigation
Press **Cmd+Up** to jump to the previous command, **Cmd+Down** to jump to the next command. (Requires shell integration — see below.)

---

## Keyboard Shortcuts Reference

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
| Cmd+Shift+Enter | Toggle zoom on current pane |
| Cmd+Shift+B | Broadcast input to all panes |
| Ctrl+Cmd+Arrow | Resize pane (keyboard) |
| Drag divider | Resize pane (mouse) |

### Navigation
| Shortcut | Action |
|----------|--------|
| Cmd+Up | Jump to previous command block |
| Cmd+Down | Jump to next command block |
| Cmd+, | Open settings viewer |
| Cmd+K | Clear scrollback |
| Cmd+L | Clear screen |
| Cmd+=/-/0 | Zoom in/out/reset |
| Cmd+Enter | Toggle fullscreen |
| Cmd+N | New window |
| Cmd+Q | Quit |

---

## Shell Integration Setup

To enable block tracking (Cmd+Up/Down navigation, command duration, exit codes), add this to your shell config:

### For Zsh (~/.zshrc)
```bash
[ -f ~/.config/volt/shell/volt-integration.zsh ] && source ~/.config/volt/shell/volt-integration.zsh
```

### For Bash (~/.bashrc)
```bash
[ -f ~/.config/volt/shell/volt-integration.bash ] && source ~/.config/volt/shell/volt-integration.bash
```

### For Fish (~/.config/fish/config.fish)
```fish
test -f ~/.config/volt/shell/volt-integration.fish && source ~/.config/volt/shell/volt-integration.fish
```

The integration scripts emit OSC 133 sequences that tell Volt where each command starts and ends.

---

## Configuration

Config file: `~/.config/volt/config.toml`

### Example Config
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

[developer]
log-level = "OFF"
```

### Connection Manager
Create `~/.config/volt/connections.toml` to save frequently used connections:

```toml
[connections.prod-server]
type = "ssh"
host = "prod.example.com"
user = "deploy"

[connections.staging-db]
type = "mysql"
host = "staging-db.example.com"
user = "app"
database = "myapp_staging"

[connections.cache]
type = "redis"
host = "redis.example.com"
port = 6379
```

---

## Feature Modules (Scaffolded — Coming Soon)

These features have their core logic built and tested. They need UI integration to become interactive:

| Module | What It Does | Tests |
|--------|-------------|-------|
| **Structured Output** | Detect JSON/table/CSV in command output, render as collapsible tree | 3 |
| **Retroactive Piping** | Pipe old command output through grep/jq/awk without re-running | 5 |
| **Session Time-Travel** | Record all commands with timestamps, search history | - |
| **Slash Commands** | Type `/split`, `/undo`, `/test` etc. for quick actions (20 commands) | 3 |
| **APFS Undo** | Snapshot files before destructive commands, restore with `/undo` | - |
| **Tmux CC Mode** | Connect to tmux, map tmux windows→tabs and panes→splits | 7 |
| **Test Runner** | `/test` auto-detects cargo/npm/pytest/go and runs tests | 3 |
| **Notifications** | macOS notification when long commands (>10s) finish | 5 |
| **Triggers** | Watch output for patterns (errors, warnings) and highlight/notify | 6 |
| **Bookmarks** | Save important commands for quick reference later | 6 |
| **Layout Presets** | Save/load pane arrangements (`dev`, `quad`, `monitoring`) | 3 |
| **Session Export** | Export sessions as asciinema, text, HTML, or JSON | 5 |
| **Shell Intelligence** | Detect project type, git branch, suggest commands | 8 |
| **Env Inspector** | Categorized env var viewer with secret masking | 8 |
| **Config Import** | Import settings from Alacritty, Ghostty, Kitty | 5 |
| **Quake Mode** | Ctrl+` dropdown terminal (animated slide-down) | 4 |
| **Block UI** | Visual decorations for command blocks (exit badges, duration) | 3 |
| **Dock Badge** | Badge on dock icon when bell rings in unfocused window | 4 |
| **Window State** | Save/restore window positions and tabs across restarts | 4 |
| **Audit Log** | Structured security log of all terminal events | 4 |

---

## Architecture

```
frontends/rioterm/     — Main application (binary: volt)
sugarloaf/             — GPU rendering engine (WGPU)
rio-backend/           — Terminal logic, ANSI parsing, config
rio-window/            — Window management (Winit fork)
teletypewriter/        — PTY/shell management
```

## Building

```bash
cargo run -p volt          # Development build + run
cargo build --release      # Release build
cargo test -p volt         # Run all tests (245+)
cargo clippy --workspace   # Lint
```

---

*Volt is forked from [Rio Terminal](https://github.com/raphamorim/rio) v0.2.37 (MIT license).*
