# Volt Terminal -- User Guide

Volt is a GPU-accelerated terminal emulator built in Rust, forked from Rio Terminal. This guide covers everything that is actually functional in the current build, and is honest about what exists only as scaffolded code.

---

## 1. Getting Started

### Build and Run

```bash
cargo run -p volt              # Development build + run
cargo build --release -p volt  # Optimized release build
cargo run -p volt --release    # Release build + run
make dev                       # Run with Metal HUD overlay
make dev-debug                 # Run with Metal HUD + debug logging
```

### Run Tests

```bash
cargo test -p volt             # Tests for the main binary
cargo test --workspace         # Tests across all crates
cargo clippy --workspace       # Lint
```

### Config File Location

Volt reads its configuration from:

```
~/.config/volt/config.toml
```

Changes to the config file are hot-reloaded -- you do not need to restart Volt.

---

## 2. UI Overview

When you launch Volt, the window is divided into three visual zones:

### Top Bar

A horizontal bar at the top of the window containing:

- **Colored tabs** -- each tab gets a unique random accent color. The active tab has a white underline indicator and full brightness; inactive tabs are dimmed. Tab labels show the tab number by default, or a custom name if you have renamed the tab. Tabs auto-scroll horizontally when there are too many to fit.
- **Help button** (right side) -- opens the keyboard shortcut reference overlay.
- **Settings button** (right side) -- opens the interactive settings editor.

The top bar is hidden when `navigation.hide-if-single` is `true` and only one tab exists.

### Terminal Area

The main terminal content area. Supports split panes with draggable dividers. When shell integration is active, command blocks are visually decorated with exit code badges (green checkmark for success, red X for failure) and duration labels.

### Bottom Status Bar

A 20-pixel bar along the bottom edge containing clickable buttons:

| Button | Color | What it opens |
|--------|-------|---------------|
| **AI** | Purple | Opens Claude Code in a split pane (requires `claude` CLI) |
| **History** | Purple-ish | Session history viewer overlay |
| **Env** | Cyan | Environment variable inspector overlay |
| **Bookmarks** | Orange | Bookmarks viewer overlay |
| **Connect** | Teal | Connection manager overlay (reads `connections.toml`) |
| **tmux** | Green (right side) | Tmux session picker overlay |

All six buttons are clickable. They open full-screen overlay panels that you dismiss with Escape.

---

## 3. Feature Reference

### Tabs

**What it does:** Create, close, rename, reorder, and switch between multiple terminal sessions. Each tab runs its own shell and gets a unique random accent color.

**How to access:**
- Cmd+T -- new tab
- Cmd+W -- close current tab (or split)
- Cmd+1 through Cmd+8 -- jump to tab by number
- Cmd+9 -- jump to last tab
- Cmd+Shift+] -- next tab
- Cmd+Shift+[ -- previous tab
- Double-click a tab -- rename it (opens a dialog)
- Cmd+Shift+R -- rename current tab
- Click a tab -- switch to it
- Scroll wheel on tab bar -- scroll tabs horizontally when they overflow

**What to expect:** Tabs appear instantly. Colors are randomly assigned on creation. Renamed tabs show the custom name (truncated to 20 characters). The active tab has a white underline and full color; inactive tabs are dimmed.

---

### Split Panes

**What it does:** Divide a tab into multiple terminal panes, each running its own shell.

**How to access:**
- Cmd+D -- split right
- Cmd+Shift+D -- split down
- Cmd+] -- select next split
- Cmd+[ -- select previous split
- Ctrl+Cmd+Arrow -- resize pane (keyboard)
- Drag the divider -- resize pane (mouse)
- Cmd+Shift+Enter -- toggle zoom on current pane (fills entire window; press again to restore)

**What to expect:** A new shell appears in the split. The divider between panes is draggable. Unfocused splits can be dimmed via the `navigation.unfocused-split-opacity` config option.

---

### Broadcast Mode

**What it does:** Sends every keystroke to ALL panes in the current tab simultaneously. Useful for running the same command on multiple servers.

**How to access:**
- Cmd+Shift+B -- toggle broadcast on/off

**What to expect:** When active, every character you type is echoed into all panes. Press Cmd+Shift+B again to disable. There is no visual indicator in the UI beyond seeing all panes react to your input.

---

### Destructive Command Warning

**What it does:** Intercepts commands before execution and shows a warning dialog if the command matches one of 20 destructive patterns. Reports severity (Danger, Warning, or Info) and shows details like file count and size for `rm -rf`.

**How to access:** Automatic -- just type a dangerous command and press Enter.

**Detected patterns:** `rm -rf`, `git push --force`, `chmod -R`, `docker prune`, `kubectl delete`, `DROP TABLE`, `terraform destroy`, `terraform apply` (without plan), `git reset --hard`, `git clean -f`, `dd`, `mkfs`, `kill -9`/`killall`, `systemctl stop`/`launchctl unload`, `pip install` (global), `npm install -g`, `sudo rm`, `mv /dev/null`, `truncate`, `chown -R`, and output redirect overwrite (`>`).

**What to expect:** A native macOS alert dialog appears with severity level, description, and details. You can choose "Execute Anyway" or "Cancel".

**Bypass:** Prefix the command with `!` to skip the check (e.g., `!rm -rf /tmp/junk`).

---

### Settings Editor

**What it does:** An interactive, VS Code-style settings panel that shows all current config values organized by category (Font, Window, Navigation, Colors, Shell, General, Cursor, Scroll, Renderer, Keyboard, Title, Bell, Hints, Developer). You can navigate with arrow keys, edit values, toggle booleans, and search. Changes are saved directly to `config.toml`.

**How to access:**
- Cmd+, -- open settings
- Click the "Settings" button in the top bar

**What to expect:** A full-screen overlay with categorized settings. Use Up/Down arrows to navigate, Enter to edit a value (or toggle a boolean), Escape to cancel an edit, and `/` to search/filter settings. Press Escape to return to the terminal. Edits are written to disk immediately.

---

### Help Overlay

**What it does:** Shows a formatted keyboard shortcut reference organized by category.

**How to access:**
- Cmd+Shift+/ -- open help
- F1 -- open help
- Click the "Help" button in the top bar

**What to expect:** A full-screen overlay with a gold accent bar showing all keyboard shortcuts. Press Escape to dismiss.

---

### AI Assistant

**What it does:** Opens Claude Code (the `claude` CLI) in a split pane alongside your terminal.

**How to access:**
- Cmd+Shift+I -- open AI assistant
- Click the "AI" button in the bottom status bar

**What to expect:** If the `claude` CLI is installed and on your PATH, Volt opens it in a new right-side split pane. If `claude` is not found, nothing happens. The assistant also detects `aider`, `copilot`, and `sgpt` as alternatives.

---

### Session History Viewer

**What it does:** Shows a scrollable list of all commands executed in the current session, with exit codes, duration, and working directory.

**How to access:**
- Cmd+Shift+H -- open history
- Click the "History" button in the bottom status bar

**What to expect:** A full-screen overlay listing commands from the current session (not persisted across restarts). Navigate with Up/Down arrows. Press Escape to dismiss. This requires shell integration to be active for block tracking data.

---

### Environment Variable Inspector

**What it does:** Displays all environment variables, categorized (Shell, Paths, Language/Runtime, Terminal, Editor, Git, Cloud/DevOps, Other) and sorted. Automatically detects and masks secrets (tokens, API keys, passwords) by showing only the first 4 characters.

**How to access:**
- Cmd+Shift+E -- open env viewer
- Click the "Env" button in the bottom status bar

**What to expect:** A full-screen overlay showing grouped environment variables. Secrets are masked (e.g., `sk-l****`). Scroll with Up/Down arrows. PATH entries are validated (existing vs missing directories). Press Escape to dismiss.

---

### Bookmarks Viewer

**What it does:** Shows saved bookmarks -- commands you have bookmarked along with their output preview, exit code, working directory, and tags.

**How to access:**
- Cmd+Shift+K -- open bookmarks
- Click the "Bookmarks" button in the bottom status bar

**What to expect:** A full-screen overlay listing bookmarks loaded from `~/.config/volt/bookmarks.json`. Bookmarks are sorted newest-first. Failed commands are highlighted in red, successful in green. Tags are shown in purple. Press Escape to dismiss.

Note: Adding bookmarks currently requires the slash command system, which is scaffolded but not yet wired to the UI.

---

### Connection Manager

**What it does:** Reads saved connections from `~/.config/volt/connections.toml` and lets you pick one to connect. Supports SSH, MySQL, PostgreSQL, Redis, Kubernetes, and Docker connection types. Generates the appropriate shell command and runs it in a new tab.

**How to access:**
- Click the "Connect" button in the bottom status bar

**What to expect:** A full-screen overlay listing your saved connections. Navigate with Up/Down arrows, press Enter to connect. The generated command (e.g., `ssh deploy@prod.example.com -p 2222`) is executed in a new tab. Press Escape to dismiss. If no `connections.toml` exists, the list is empty.

---

### Tmux CC Mode

**What it does:** Lists available tmux sessions and lets you attach via tmux Control Center (CC) mode. Parses tmux CC protocol notifications to map tmux windows to Volt tabs and tmux panes to Volt splits.

**How to access:**
- Ctrl+Shift+Cmd+T -- open tmux picker
- Click the "tmux" button in the bottom status bar

**What to expect:** A full-screen overlay listing tmux sessions (id, name, attached status). Select one and press Enter to run `tmux -CC attach -t <name>`. If no tmux sessions exist, the list is empty. The CC protocol parser handles `%session-changed`, `%window-add`, `%window-close`, `%window-renamed`, `%layout-change`, and `%exit` notifications.

---

### Block Navigation

**What it does:** Jumps the viewport to the previous or next command block. Requires shell integration (OSC 133 sequences) to know where commands start and end.

**How to access:**
- Cmd+Up -- jump to previous command block
- Cmd+Down -- jump to next command block

**What to expect:** The terminal scrolls so the target command block is at the top of the viewport. If there is no previous/next block, nothing happens. Each block tracks its start row, end row, exit code, and duration.

---

### Block Decorations

**What it does:** Renders visual overlays on command blocks: a thin horizontal separator line above each block, a colored exit code badge (green checkmark for 0, red X for non-zero, grey circle for running), and a duration label.

**How to access:** Automatic when shell integration is active.

**What to expect:** Small visual decorations appear above each command block. Duration is shown as `500ms`, `2.5s`, or `1m5s` depending on length.

---

### Notifications

**What it does:** Sends a macOS notification when a long-running command (default: more than 10 seconds) finishes. Reports the command name, status, duration, and exit code. Plays an error sound for failed commands.

**How to access:** Automatic. Uses NSUserNotificationCenter on macOS with an `osascript` fallback. On Linux, uses `notify-send`.

**What to expect:** If you run a command that takes more than 10 seconds and the window is open, a system notification appears when it completes.

---

### Search

**What it does:** Forward and backward text search within the terminal scrollback.

**How to access:**
- Cmd+F -- search forward
- Cmd+B -- search backward (from current position)
- Ctrl+C -- cancel search
- Ctrl+U -- clear search input
- Ctrl+W -- delete word in search
- Up/Down arrows -- cycle through search history

**What to expect:** A search bar appears. Matches are highlighted. Navigate between matches with the search history keys.

---

### Quake Mode

**What it does:** A dropdown terminal that slides down from the top of the screen with an animated transition (ease-out cubic, 200ms). Toggled with a global hotkey.

**How to access:**
- Ctrl+` -- toggle quake mode

**What to expect:** The terminal slides down occupying 40% of screen height. Press again to hide. Note: The animation and window positioning logic is implemented, but global hotkey registration from outside the app is not -- the hotkey only works when Volt is focused.

---

## 4. Keyboard Shortcuts

### macOS

#### General

| Shortcut | Action |
|----------|--------|
| Cmd+N | New window |
| Cmd+Q | Quit |
| Cmd+, | Open settings editor |
| Cmd+Shift+/ | Show help overlay |
| F1 | Show help overlay |
| Cmd+H | Hide window |
| Cmd+Alt+H | Hide other applications |
| Cmd+M | Minimize window |
| Ctrl+Cmd+F | Toggle fullscreen |
| Cmd+Enter | Toggle fullscreen |

#### Tabs

| Shortcut | Action |
|----------|--------|
| Cmd+T | New tab |
| Cmd+W | Close current split or tab |
| Cmd+1 through Cmd+8 | Jump to tab N |
| Cmd+9 | Jump to last tab |
| Cmd+Shift+] | Next tab |
| Cmd+Shift+[ | Previous tab |
| Ctrl+Tab | Next tab |
| Ctrl+Shift+Tab | Previous tab |
| Cmd+Shift+R | Rename current tab |
| Double-click tab | Rename tab |

#### Split Panes

| Shortcut | Action |
|----------|--------|
| Cmd+D | Split right |
| Cmd+Shift+D | Split down |
| Cmd+] | Select next split |
| Cmd+[ | Select previous split |
| Ctrl+Cmd+Up | Resize pane (move divider up) |
| Ctrl+Cmd+Down | Resize pane (move divider down) |
| Ctrl+Cmd+Left | Resize pane (move divider left) |
| Ctrl+Cmd+Right | Resize pane (move divider right) |
| Cmd+Shift+Enter | Toggle pane zoom |
| Cmd+Shift+B | Toggle broadcast mode |

#### Navigation and Tools

| Shortcut | Action |
|----------|--------|
| Cmd+Up | Jump to previous command block |
| Cmd+Down | Jump to next command block |
| Cmd+K | Clear scrollback |
| Cmd+F | Search forward |
| Cmd+B | Search backward |
| Cmd+Shift+I | Open AI assistant |
| Cmd+Shift+H | Open session history |
| Cmd+Shift+E | Open environment inspector |
| Cmd+Shift+K | Open bookmarks |
| Ctrl+Shift+Cmd+T | Open tmux picker |
| Ctrl+` | Toggle quake mode |

#### Font Size

| Shortcut | Action |
|----------|--------|
| Cmd+= | Zoom in |
| Cmd+- | Zoom out |
| Cmd+0 | Reset font size |

#### Copy/Paste

| Shortcut | Action |
|----------|--------|
| Cmd+C | Copy |
| Cmd+V | Paste |

### Linux/Other

On Linux, shortcuts use Ctrl+Shift instead of Cmd. Notable differences:

| Shortcut | Action |
|----------|--------|
| Ctrl+Shift+C | Copy |
| Ctrl+Shift+V | Paste |
| Ctrl+Shift+N | New window |
| Ctrl+Shift+T | New tab |
| Ctrl+Shift+W | Close tab/split |
| Ctrl+Shift+, | Open settings |
| Ctrl+Shift+/ | Show help |
| Ctrl+Shift+F | Search forward |
| Ctrl+Shift+R | Split right |
| Ctrl+Shift+D | Split down |
| Ctrl+F2 | Rename tab |
| Ctrl+0 | Reset font size |
| Ctrl+= | Zoom in |
| Ctrl+- | Zoom out |

---

## 5. Configuration

### Config File

```
~/.config/volt/config.toml
```

Volt watches this file and hot-reloads changes.

### Example Config

```toml
[fonts]
family = "JetBrains Mono"
size = 14
hinting = true

[window]
width = 800
height = 600
opacity = 1.0
blur = false
mode = "windowed"
decorations = "enabled"
macos-use-unified-titlebar = false
macos-use-shadow = true

[navigation]
mode = "TopTab"          # TopTab, BottomTab, Bookmark, Plain, NativeTab
hide-if-single = false
use-split = true
clickable = true
use-terminal-title = true
unfocused-split-opacity = 0.5

[colors]
background = "#0F0D0E"
foreground = "#F9F4DA"

[shell]
program = "/bin/zsh"
args = ["--login"]

[cursor]
shape = "Block"          # Block, Underline, Beam, Hidden
blinking = false
blinking-interval = 500

[scroll]
multiplier = 3.0
divider = 1.0

[renderer]
performance = "High"     # High, Low
backend = "Automatic"    # Automatic, Vulkan, GL, DX12, Metal
disable-unfocused-render = false

[developer]
log-level = "OFF"        # OFF, ERROR, WARN, INFO, DEBUG, TRACE
enable-log-file = false
enable-fps-counter = false

[bell]
visual = false
audio = false

confirm-before-quit = false
hide-mouse-cursor-when-typing = false
option-as-alt = "both"   # left, right, both
line-height = 1.0
padding-x = 0.0
```

### Connections File

Create `~/.config/volt/connections.toml` to define saved connections:

```toml
[connections.prod-server]
type = "ssh"
host = "prod.example.com"
user = "deploy"
port = 22
identity_file = "~/.ssh/id_ed25519"
# proxy_jump = "bastion.example.com"
# forward_agent = true

[connections.staging-db]
type = "mysql"
host = "staging-db.example.com"
user = "app"
database = "myapp_staging"

[connections.cache]
type = "redis"
host = "redis.example.com"
port = 6379

[connections.k8s-prod]
type = "kubectl"
context = "production"
namespace = "default"

[connections.analytics-db]
type = "postgres"
host = "analytics.example.com"
user = "readonly"
database = "analytics"

[connections.docker-remote]
type = "docker"
host = "tcp://docker.example.com:2376"
```

### Bookmarks Storage

Bookmarks are stored in JSON at:

```
~/.config/volt/bookmarks.json
```

This file is managed automatically by the bookmark system.

---

## 6. Shell Integration

Shell integration enables block tracking (Cmd+Up/Down navigation), command duration, exit code badges, and notification support. It works by emitting OSC 133 escape sequences that tell Volt where each command starts and ends.

Volt automatically installs integration scripts at startup to `~/.config/volt/shell/`. You need to source the appropriate one from your shell config.

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

### What the Integration Provides

The scripts install hooks for your shell's prompt/preexec lifecycle:

- **OSC 133;A** -- emitted before the prompt is drawn
- **OSC 133;B** -- emitted when the user starts typing a command
- **OSC 133;C** -- emitted when the command begins executing
- **OSC 133;D;{exit_code}** -- emitted when the command finishes

These enable: block navigation (Cmd+Up/Down), exit code badges, command duration display, and long-running command notifications.

---

## 7. Known Limitations

### What Works

These features are fully implemented, wired to the UI, and functional:

- Color-coded tabs with click, scroll, rename, and keyboard navigation
- Split panes with draggable dividers, keyboard resize, and zoom
- Broadcast mode (type into all panes)
- Destructive command warning with native macOS alert dialogs
- Interactive settings editor with search, edit, toggle, and save to disk
- Help overlay showing keyboard shortcuts
- Block navigation (Cmd+Up/Down) with shell integration
- Block decorations (exit code badges, duration labels)
- Session history viewer (current session only, not persisted)
- Environment variable inspector with categorization and secret masking
- Bookmarks viewer (reads from bookmarks.json)
- Connection manager (reads from connections.toml, opens connection in new tab)
- Tmux CC mode session picker
- AI assistant launcher (opens `claude` CLI in split pane)
- Search (forward and backward)
- macOS notifications for long-running commands
- Quake mode toggle (animation logic works, but no global hotkey when unfocused)
- Config hot-reload
- TOML configuration with full settings coverage
- Shell integration scripts for zsh, bash, and fish
- GPU rendering via wgpu (Metal on macOS, Vulkan/GL elsewhere)
- Sixel, iTerm2, and Kitty image protocols (inherited from Rio)
- Font ligatures and text shaping
- Vi mode
- True color (24-bit)
- Themes

### What Is Scaffolded (Code Exists, Not Yet Wired to UI)

These modules have their core logic implemented with unit tests, but lack the event-loop integration to make them interactive from the terminal:

| Module | Status |
|--------|--------|
| **Structured Output** | Data model and parsers for JSON/table/CSV exist. No detection or rendering on live output. |
| **Retroactive Piping** | `pipe_through()` function works in isolation. No UI to select previous output and pipe it. |
| **Slash Commands** | 20 commands defined with categories and usage text. Not intercepted at the prompt. |
| **APFS Undo** | Snapshot and restore logic scaffolded. No `/undo` command wired. |
| **Test Runner** | Auto-detection for cargo/npm/pytest/go test frameworks. No `/test` command wired. |
| **Triggers** | Rule engine with regex matching, notify/highlight/run/bell actions. Not connected to output stream. |
| **Layout Presets** | Save/load pane arrangements. Not connected to any UI or command. |
| **Session Export** | Text export function exists. No command to invoke it. |
| **Shell Intelligence** | Project type detection, git branch awareness. Not connected to prompt or UI. |
| **Config Import** | Parsers for Alacritty, Ghostty, Kitty config formats. No command to invoke. |
| **Dock Badge** | Badge-on-bell logic scaffolded. Not connected to bell events. |
| **Window State** | Save/restore logic scaffolded. Not connected to app lifecycle. |
| **Audit Log** | Structured event logging scaffolded. Not connected to event stream. |

### Other Limitations

- Quake mode only responds to the toggle hotkey when Volt is focused. No system-wide global hotkey registration.
- Session history is in-memory only and lost when Volt exits.
- Bookmarks can be viewed but there is no UI to add new bookmarks (the `BookmarkStore::add` API exists but is not exposed through a command or button).
- The AI assistant button does nothing if the `claude` CLI is not installed.
- Connection manager has no UI to create/edit connections -- you must edit `connections.toml` manually.
- macOS native tabs (NativeTab mode) use NSWindowTabGroup but do not show the colored tab bar.

---

*Volt is forked from [Rio Terminal](https://github.com/raphamorim/rio) v0.2.37 (MIT license).*
