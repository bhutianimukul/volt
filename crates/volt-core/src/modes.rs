//! Terminal mode flags — DEC private modes, ANSI modes, mouse tracking, and
//! Kitty keyboard protocol mode stack.

/// DEC private and ANSI mode flags.
///
/// Stored as a flat struct of bools for clarity and fast access.
/// These are toggled via CSI ? Pm h (set) and CSI ? Pm l (reset).
#[derive(Debug, Clone)]
pub struct TerminalModes {
    // --- DEC Private Modes ---
    /// DECCKM: Application cursor keys (mode 1).
    pub application_cursor_keys: bool,

    /// DECOM: Origin mode (mode 6). Cursor addressing relative to scroll region.
    pub origin_mode: bool,

    /// DECAWM: Auto-wrap mode (mode 7). Wrap at right margin.
    pub auto_wrap: bool,

    /// DECTCEM: Text cursor enable (mode 25). Show/hide cursor.
    pub cursor_visible: bool,

    /// Alternate screen buffer (mode 1049). Also saves/restores cursor.
    pub alternate_screen: bool,

    // --- Mouse Modes (mutually exclusive) ---
    /// Current mouse tracking mode.
    pub mouse_mode: MouseMode,

    /// SGR mouse encoding (mode 1006). Extended coordinates.
    pub sgr_mouse: bool,

    // --- Keyboard & Input ---
    /// Bracketed paste mode (mode 2004).
    pub bracketed_paste: bool,

    /// Focus reporting (mode 1004). Send CSI I / CSI O on focus in/out.
    pub focus_reporting: bool,

    // --- Output Control ---
    /// Synchronized output (DEC mode 2026). Buffer output between
    /// BSU (Begin Synchronized Update) and ESU markers.
    pub synchronized_output: bool,

    // --- Application Keypad ---
    /// DECNKM / DECKPAM: Application keypad mode.
    pub application_keypad: bool,

    // --- Kitty Keyboard Protocol ---
    /// Mode stack for the Kitty keyboard protocol (CSI > u / CSI < u).
    /// Each entry is the flags value pushed onto the stack.
    pub kitty_keyboard_stack: Vec<u32>,
}

impl Default for TerminalModes {
    fn default() -> Self {
        Self {
            application_cursor_keys: false,
            origin_mode: false,
            auto_wrap: true, // DECAWM defaults to ON
            cursor_visible: true,
            alternate_screen: false,
            mouse_mode: MouseMode::None,
            sgr_mouse: false,
            bracketed_paste: false,
            focus_reporting: false,
            synchronized_output: false,
            application_keypad: false,
            kitty_keyboard_stack: Vec::new(),
        }
    }
}

impl TerminalModes {
    /// Set a DEC private mode by its numeric parameter.
    pub fn set_dec_mode(&mut self, mode: u16, enable: bool) {
        match mode {
            1 => self.application_cursor_keys = enable,
            6 => self.origin_mode = enable,
            7 => self.auto_wrap = enable,
            25 => self.cursor_visible = enable,
            1000 => self.set_mouse_mode(MouseMode::Press, enable),
            1002 => self.set_mouse_mode(MouseMode::ButtonMotion, enable),
            1003 => self.set_mouse_mode(MouseMode::AnyMotion, enable),
            1004 => self.focus_reporting = enable,
            1006 => self.sgr_mouse = enable,
            2004 => self.bracketed_paste = enable,
            2026 => self.synchronized_output = enable,
            1049 => self.alternate_screen = enable,
            _ => {
                tracing::debug!("Unhandled DEC private mode: {mode} = {enable}");
            }
        }
    }

    fn set_mouse_mode(&mut self, mode: MouseMode, enable: bool) {
        if enable {
            self.mouse_mode = mode;
        } else if self.mouse_mode == mode {
            self.mouse_mode = MouseMode::None;
        }
    }

    /// Push a Kitty keyboard flags value onto the stack.
    pub fn push_kitty_keyboard(&mut self, flags: u32) {
        self.kitty_keyboard_stack.push(flags);
    }

    /// Pop the top Kitty keyboard flags value. Returns the new active flags (0 if empty).
    pub fn pop_kitty_keyboard(&mut self) -> u32 {
        self.kitty_keyboard_stack.pop();
        self.kitty_keyboard_flags()
    }

    /// Current Kitty keyboard flags (top of stack, or 0).
    pub fn kitty_keyboard_flags(&self) -> u32 {
        self.kitty_keyboard_stack.last().copied().unwrap_or(0)
    }
}

/// Mouse tracking mode. These are mutually exclusive — enabling one disables others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// No mouse tracking.
    None,
    /// X10 compatibility mode (mode 9) — report button press only.
    X10,
    /// Normal tracking (mode 1000) — report press and release.
    Press,
    /// Button-event tracking (mode 1002) — report press, release, and motion while pressed.
    ButtonMotion,
    /// Any-event tracking (mode 1003) — report all motion.
    AnyMotion,
}

/// Scroll region (DECSTBM).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollRegion {
    /// Top row of scroll region (0-indexed, inclusive).
    pub top: usize,
    /// Bottom row of scroll region (0-indexed, inclusive).
    pub bottom: usize,
}

impl ScrollRegion {
    pub fn new(top: usize, bottom: usize) -> Self {
        Self { top, bottom }
    }

    /// Full-screen scroll region for the given number of rows.
    pub fn full(rows: usize) -> Self {
        Self {
            top: 0,
            bottom: rows.saturating_sub(1),
        }
    }

    /// Whether this region covers the entire screen.
    pub fn is_full(&self, rows: usize) -> bool {
        self.top == 0 && self.bottom == rows.saturating_sub(1)
    }

    /// Number of rows in the scroll region.
    pub fn height(&self) -> usize {
        self.bottom - self.top + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_modes() {
        let modes = TerminalModes::default();
        assert!(modes.auto_wrap);
        assert!(modes.cursor_visible);
        assert!(!modes.alternate_screen);
        assert_eq!(modes.mouse_mode, MouseMode::None);
    }

    #[test]
    fn dec_mode_toggle() {
        let mut modes = TerminalModes::default();
        modes.set_dec_mode(1, true);
        assert!(modes.application_cursor_keys);
        modes.set_dec_mode(1, false);
        assert!(!modes.application_cursor_keys);
    }

    #[test]
    fn mouse_modes_exclusive() {
        let mut modes = TerminalModes::default();
        modes.set_dec_mode(1000, true);
        assert_eq!(modes.mouse_mode, MouseMode::Press);
        modes.set_dec_mode(1003, true);
        assert_eq!(modes.mouse_mode, MouseMode::AnyMotion);
    }

    #[test]
    fn kitty_keyboard_stack() {
        let mut modes = TerminalModes::default();
        assert_eq!(modes.kitty_keyboard_flags(), 0);
        modes.push_kitty_keyboard(1);
        assert_eq!(modes.kitty_keyboard_flags(), 1);
        modes.push_kitty_keyboard(3);
        assert_eq!(modes.kitty_keyboard_flags(), 3);
        modes.pop_kitty_keyboard();
        assert_eq!(modes.kitty_keyboard_flags(), 1);
    }

    #[test]
    fn scroll_region() {
        let region = ScrollRegion::full(24);
        assert!(region.is_full(24));
        assert_eq!(region.height(), 24);

        let region = ScrollRegion::new(5, 20);
        assert!(!region.is_full(24));
        assert_eq!(region.height(), 16);
    }
}
