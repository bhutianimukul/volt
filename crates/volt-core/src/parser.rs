//! VT parser wrapping the `vte` crate.
//!
//! Implements `vte::Perform` to translate escape sequences into terminal state
//! mutations. The parser runs on a dedicated thread in the final architecture;
//! parsed actions are applied to the terminal state.
//!
//! The handler receives raw VT events and translates them into high-level
//! operations on the Terminal struct.

use crate::cell::CellFlags;
use crate::color::{Color, NamedColor, Rgb};
use crate::cursor::SavedCursor;
use crate::grid::Grid;
use crate::modes::{ScrollRegion, TerminalModes};
use crate::osc;

/// Events emitted by the parser that the host application should handle.
/// These are things the terminal can't handle internally (e.g., clipboard, title changes).
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Window title changed.
    TitleChanged(String),
    /// Working directory changed (OSC 7).
    WorkingDirectoryChanged(String),
    /// Clipboard write request (OSC 52). Requires user confirmation per security policy.
    ClipboardStore { clipboard: String, data: String },
    /// Clipboard read request (OSC 52).
    ClipboardLoad { clipboard: String },
    /// Bell (BEL character).
    Bell,
    /// Shell integration marker.
    ShellIntegration(osc::ShellIntegrationMark),
}

/// The VT handler that implements `vte::Perform`.
///
/// Holds mutable references to all terminal state and modifies it in response
/// to escape sequences. Events that need host handling are collected in `events`.
pub struct Handler<'a> {
    pub grid: &'a mut Grid,
    pub cursor: &'a mut crate::cursor::Cursor,
    pub modes: &'a mut TerminalModes,
    pub scroll_region: &'a mut ScrollRegion,
    pub saved_cursor: &'a mut SavedCursor,
    pub alt_saved_cursor: &'a mut SavedCursor,
    pub alt_grid: &'a mut Grid,
    pub scrollback_rows: &'a mut Vec<crate::grid::Row>,
    pub events: &'a mut Vec<TerminalEvent>,
    pub cols: usize,
    pub rows: usize,
}

impl Handler<'_> {
    /// Current cursor row.
    fn cursor_row(&self) -> usize {
        self.cursor.pos.row
    }

    /// Current cursor column.
    fn cursor_col(&self) -> usize {
        self.cursor.pos.col
    }

    /// Write a character at the cursor position and advance.
    fn write_char(&mut self, c: char) {
        // Handle pending wrap
        if self.cursor.pending_wrap {
            if self.modes.auto_wrap {
                // Set wrap flag on current row
                self.grid.row_mut(self.cursor_row()).set_wrapped(true);
                self.linefeed();
                self.cursor.pos.col = 0;
            }
            self.cursor.pending_wrap = false;
        }

        let row = self.cursor_row();
        let col = self.cursor_col();

        // Apply current SGR template to the cell
        let cell = self.grid.cell_mut(row, col);
        self.cursor.template.apply(cell, c);

        // Advance cursor
        if col + 1 >= self.cols {
            // At right margin: set pending wrap (don't wrap yet)
            self.cursor.pending_wrap = true;
        } else {
            self.cursor.pos.col += 1;
        }
    }

    /// Perform a linefeed: move cursor down, scrolling if at bottom of scroll region.
    fn linefeed(&mut self) {
        let row = self.cursor_row();
        let bottom = self.scroll_region.bottom;

        if row == bottom {
            // At bottom of scroll region: scroll up
            let bg = self.cursor.template.bg;
            let scrolled = self.grid.scroll_up(self.scroll_region.top, bottom, 1, bg);
            self.scrollback_rows.extend(scrolled);
        } else if row + 1 < self.rows {
            self.cursor.pos.row += 1;
        }
    }

    /// Perform a reverse linefeed: move cursor up, scrolling if at top of scroll region.
    fn reverse_linefeed(&mut self) {
        let row = self.cursor_row();
        let top = self.scroll_region.top;

        if row == top {
            let bg = self.cursor.template.bg;
            self.grid.scroll_down(top, self.scroll_region.bottom, 1, bg);
        } else if row > 0 {
            self.cursor.pos.row -= 1;
        }
    }

    /// Handle CSI SGR (Select Graphic Rendition) — set text attributes.
    fn handle_sgr(&mut self, params: &[Vec<u16>]) {
        let mut i = 0;
        let flat: Vec<u16> = params.iter().flat_map(|sub| sub.iter().copied()).collect();

        if flat.is_empty() {
            self.cursor.template.reset();
            return;
        }

        while i < flat.len() {
            match flat[i] {
                0 => self.cursor.template.reset(),
                1 => {
                    self.cursor.template.flags = self.cursor.template.flags.insert(CellFlags::BOLD)
                }
                2 => self.cursor.template.flags = self.cursor.template.flags.insert(CellFlags::DIM),
                3 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.insert(CellFlags::ITALIC)
                }
                4 => {
                    // Underline — check for subparameter for style
                    self.cursor.template.flags =
                        self.cursor.template.flags.remove(CellFlags::ALL_UNDERLINES);
                    if i + 1 < flat.len() && !params.is_empty() && params[0].len() > 1 {
                        // Subparameter style: 4:0 = none, 4:1 = single, 4:2 = double, 4:3 = curly, etc.
                        match flat[i + 1] {
                            0 => {} // No underline
                            1 => {
                                self.cursor.template.flags =
                                    self.cursor.template.flags.insert(CellFlags::UNDERLINE)
                            }
                            2 => {
                                self.cursor.template.flags = self
                                    .cursor
                                    .template
                                    .flags
                                    .insert(CellFlags::DOUBLE_UNDERLINE)
                            }
                            3 => {
                                self.cursor.template.flags = self
                                    .cursor
                                    .template
                                    .flags
                                    .insert(CellFlags::CURLY_UNDERLINE)
                            }
                            4 => {
                                self.cursor.template.flags = self
                                    .cursor
                                    .template
                                    .flags
                                    .insert(CellFlags::DOTTED_UNDERLINE)
                            }
                            5 => {
                                self.cursor.template.flags = self
                                    .cursor
                                    .template
                                    .flags
                                    .insert(CellFlags::DASHED_UNDERLINE)
                            }
                            _ => {
                                self.cursor.template.flags =
                                    self.cursor.template.flags.insert(CellFlags::UNDERLINE)
                            }
                        }
                        i += 1;
                    } else {
                        self.cursor.template.flags =
                            self.cursor.template.flags.insert(CellFlags::UNDERLINE);
                    }
                }
                5 => {
                    self.cursor.template.flags = self.cursor.template.flags.insert(CellFlags::BLINK)
                }
                7 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.insert(CellFlags::INVERSE)
                }
                8 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.insert(CellFlags::HIDDEN)
                }
                9 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.insert(CellFlags::STRIKETHROUGH)
                }
                21 => {
                    self.cursor.template.flags = self
                        .cursor
                        .template
                        .flags
                        .insert(CellFlags::DOUBLE_UNDERLINE)
                }
                22 => {
                    self.cursor.template.flags = self
                        .cursor
                        .template
                        .flags
                        .remove(CellFlags::BOLD)
                        .remove(CellFlags::DIM)
                }
                23 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.remove(CellFlags::ITALIC)
                }
                24 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.remove(CellFlags::ALL_UNDERLINES)
                }
                25 => {
                    self.cursor.template.flags = self.cursor.template.flags.remove(CellFlags::BLINK)
                }
                27 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.remove(CellFlags::INVERSE)
                }
                28 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.remove(CellFlags::HIDDEN)
                }
                29 => {
                    self.cursor.template.flags =
                        self.cursor.template.flags.remove(CellFlags::STRIKETHROUGH)
                }
                // Foreground colors
                30 => self.cursor.template.fg = Color::Named(NamedColor::Black),
                31 => self.cursor.template.fg = Color::Named(NamedColor::Red),
                32 => self.cursor.template.fg = Color::Named(NamedColor::Green),
                33 => self.cursor.template.fg = Color::Named(NamedColor::Yellow),
                34 => self.cursor.template.fg = Color::Named(NamedColor::Blue),
                35 => self.cursor.template.fg = Color::Named(NamedColor::Magenta),
                36 => self.cursor.template.fg = Color::Named(NamedColor::Cyan),
                37 => self.cursor.template.fg = Color::Named(NamedColor::White),
                38 => {
                    if let Some((color, consumed)) = parse_color(&flat[i + 1..]) {
                        self.cursor.template.fg = color;
                        i += consumed;
                    }
                }
                39 => self.cursor.template.fg = Color::Default,
                // Background colors
                40 => self.cursor.template.bg = Color::Named(NamedColor::Black),
                41 => self.cursor.template.bg = Color::Named(NamedColor::Red),
                42 => self.cursor.template.bg = Color::Named(NamedColor::Green),
                43 => self.cursor.template.bg = Color::Named(NamedColor::Yellow),
                44 => self.cursor.template.bg = Color::Named(NamedColor::Blue),
                45 => self.cursor.template.bg = Color::Named(NamedColor::Magenta),
                46 => self.cursor.template.bg = Color::Named(NamedColor::Cyan),
                47 => self.cursor.template.bg = Color::Named(NamedColor::White),
                48 => {
                    if let Some((color, consumed)) = parse_color(&flat[i + 1..]) {
                        self.cursor.template.bg = color;
                        i += consumed;
                    }
                }
                49 => self.cursor.template.bg = Color::Default,
                // Bright foreground colors
                90 => self.cursor.template.fg = Color::Named(NamedColor::BrightBlack),
                91 => self.cursor.template.fg = Color::Named(NamedColor::BrightRed),
                92 => self.cursor.template.fg = Color::Named(NamedColor::BrightGreen),
                93 => self.cursor.template.fg = Color::Named(NamedColor::BrightYellow),
                94 => self.cursor.template.fg = Color::Named(NamedColor::BrightBlue),
                95 => self.cursor.template.fg = Color::Named(NamedColor::BrightMagenta),
                96 => self.cursor.template.fg = Color::Named(NamedColor::BrightCyan),
                97 => self.cursor.template.fg = Color::Named(NamedColor::BrightWhite),
                // Bright background colors
                100 => self.cursor.template.bg = Color::Named(NamedColor::BrightBlack),
                101 => self.cursor.template.bg = Color::Named(NamedColor::BrightRed),
                102 => self.cursor.template.bg = Color::Named(NamedColor::BrightGreen),
                103 => self.cursor.template.bg = Color::Named(NamedColor::BrightYellow),
                104 => self.cursor.template.bg = Color::Named(NamedColor::BrightBlue),
                105 => self.cursor.template.bg = Color::Named(NamedColor::BrightMagenta),
                106 => self.cursor.template.bg = Color::Named(NamedColor::BrightCyan),
                107 => self.cursor.template.bg = Color::Named(NamedColor::BrightWhite),
                _ => {
                    tracing::debug!("Unhandled SGR parameter: {}", flat[i]);
                }
            }
            i += 1;
        }
    }
}

/// Parse an extended color (38/48 prefix already consumed).
/// Handles both `2;r;g;b` (true color) and `5;idx` (256-color).
/// Returns the color and how many additional params were consumed.
fn parse_color(params: &[u16]) -> Option<(Color, usize)> {
    match params.first() {
        Some(2) if params.len() >= 4 => {
            let r = params[1] as u8;
            let g = params[2] as u8;
            let b = params[3] as u8;
            Some((Color::Rgb(Rgb::new(r, g, b)), 4))
        }
        Some(5) if params.len() >= 2 => Some((Color::Indexed(params[1] as u8), 2)),
        _ => None,
    }
}

/// Extract a single numeric param from CSI parameters, with a default.
fn csi_param(params: &[Vec<u16>], index: usize, default: u16) -> u16 {
    params
        .get(index)
        .and_then(|sub| sub.first().copied())
        .map(|v| if v == 0 { default } else { v })
        .unwrap_or(default)
}

/// Collect vte::Params into a Vec of Vec for easier indexing.
fn collect_params(params: &vte::Params) -> Vec<Vec<u16>> {
    params.iter().map(|sub| sub.to_vec()).collect()
}

impl vte::Perform for Handler<'_> {
    /// Print a character to the terminal.
    fn print(&mut self, c: char) {
        self.write_char(c);
    }

    /// Execute a C0/C1 control character.
    fn execute(&mut self, byte: u8) {
        match byte {
            // BEL
            0x07 => {
                self.events.push(TerminalEvent::Bell);
            }
            // BS — Backspace
            0x08 => {
                if self.cursor.pos.col > 0 {
                    self.cursor.pos.col -= 1;
                    self.cursor.pending_wrap = false;
                }
            }
            // HT — Horizontal Tab (advance to next tab stop, default every 8 cols)
            0x09 => {
                let col = self.cursor_col();
                let next_tab = ((col / 8) + 1) * 8;
                self.cursor.pos.col = next_tab.min(self.cols - 1);
                self.cursor.pending_wrap = false;
            }
            // LF, VT, FF — Line Feed (and vertical tab, form feed treated same)
            0x0A..=0x0C => {
                self.linefeed();
            }
            // CR — Carriage Return
            0x0D => {
                self.cursor.pos.col = 0;
                self.cursor.pending_wrap = false;
            }
            // SI — Shift In (select G0 charset) — ignored for now
            0x0F => {}
            // SO — Shift Out (select G1 charset) — ignored for now
            0x0E => {}
            _ => {
                tracing::debug!("Unhandled C0 control: 0x{byte:02x}");
            }
        }
    }

    /// Handle a CSI (Control Sequence Introducer) dispatch.
    fn csi_dispatch(
        &mut self,
        raw_params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params = collect_params(raw_params);
        let is_private = intermediates.first() == Some(&b'?');

        match (action, is_private) {
            // --- Cursor Movement ---

            // CUU — Cursor Up
            ('A', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                self.cursor.move_up(n, self.scroll_region.top);
            }
            // CUD — Cursor Down
            ('B', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                self.cursor.move_down(n, self.scroll_region.bottom);
            }
            // CUF — Cursor Forward (Right)
            ('C', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                self.cursor.move_right(n, self.cols);
            }
            // CUB — Cursor Backward (Left)
            ('D', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                self.cursor.move_left(n);
            }
            // CUP / HVP — Cursor Position
            ('H' | 'f', false) => {
                let row = csi_param(&params, 0, 1) as usize - 1; // 1-based → 0-based
                let col = csi_param(&params, 1, 1) as usize - 1;
                self.cursor.goto(row, col, self.rows, self.cols);
            }

            // --- Erase ---

            // ED — Erase in Display
            ('J', false) => {
                let bg = self.cursor.template.bg;
                match csi_param(&params, 0, 0) {
                    // Erase below (from cursor to end of screen)
                    0 => {
                        let row = self.cursor_row();
                        let col = self.cursor_col();
                        self.grid.row_mut(row).clear_from(col, bg);
                        for r in row + 1..self.rows {
                            if bg == Color::Default {
                                self.grid.row_mut(r).clear();
                            } else {
                                self.grid.row_mut(r).clear_with_bg(bg);
                            }
                        }
                    }
                    // Erase above (from start to cursor)
                    1 => {
                        let row = self.cursor_row();
                        let col = self.cursor_col();
                        for r in 0..row {
                            if bg == Color::Default {
                                self.grid.row_mut(r).clear();
                            } else {
                                self.grid.row_mut(r).clear_with_bg(bg);
                            }
                        }
                        self.grid.row_mut(row).clear_to(col + 1, bg);
                    }
                    // Erase entire display
                    2 => {
                        self.grid.clear(bg);
                    }
                    // Erase scrollback (xterm extension)
                    3 => {
                        // Scrollback clearing is handled by Terminal
                        self.grid.clear(bg);
                    }
                    _ => {}
                }
            }
            // EL — Erase in Line
            ('K', false) => {
                let bg = self.cursor.template.bg;
                let row = self.cursor_row();
                let col = self.cursor_col();
                match csi_param(&params, 0, 0) {
                    0 => self.grid.row_mut(row).clear_from(col, bg),
                    1 => self.grid.row_mut(row).clear_to(col + 1, bg),
                    2 => {
                        if bg == Color::Default {
                            self.grid.row_mut(row).clear()
                        } else {
                            self.grid.row_mut(row).clear_with_bg(bg)
                        }
                    }
                    _ => {}
                }
            }

            // --- Scroll ---

            // SU — Scroll Up
            ('S', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let bg = self.cursor.template.bg;
                let scrolled =
                    self.grid
                        .scroll_up(self.scroll_region.top, self.scroll_region.bottom, n, bg);
                self.scrollback_rows.extend(scrolled);
            }
            // SD — Scroll Down
            ('T', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let bg = self.cursor.template.bg;
                self.grid
                    .scroll_down(self.scroll_region.top, self.scroll_region.bottom, n, bg);
            }

            // --- Insert/Delete ---

            // ICH — Insert Characters (shift right)
            ('@', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let row = self.cursor_row();
                let col = self.cursor_col();
                let grid_row = self.grid.row_mut(row);
                // Shift cells right
                for c in (col + n..grid_row.len()).rev() {
                    let prev = grid_row[c - n].clone();
                    *grid_row.cell_mut(c) = prev;
                }
                // Clear inserted cells
                let bg = self.cursor.template.bg;
                for c in col..col + n.min(grid_row.len() - col) {
                    grid_row.cell_mut(c).reset_with_bg(bg);
                }
            }
            // DCH — Delete Characters (shift left)
            ('P', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let row = self.cursor_row();
                let col = self.cursor_col();
                let grid_row = self.grid.row_mut(row);
                let len = grid_row.len();
                // Shift cells left
                for c in col..len.saturating_sub(n) {
                    let next = grid_row[c + n].clone();
                    *grid_row.cell_mut(c) = next;
                }
                // Clear vacated cells at end
                let bg = self.cursor.template.bg;
                for c in len.saturating_sub(n)..len {
                    grid_row.cell_mut(c).reset_with_bg(bg);
                }
            }
            // IL — Insert Lines
            ('L', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let bg = self.cursor.template.bg;
                let row = self.cursor_row();
                self.grid.scroll_down(row, self.scroll_region.bottom, n, bg);
            }
            // DL — Delete Lines
            ('M', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let bg = self.cursor.template.bg;
                let row = self.cursor_row();
                let scrolled = self.grid.scroll_up(row, self.scroll_region.bottom, n, bg);
                self.scrollback_rows.extend(scrolled);
            }
            // ECH — Erase Characters
            ('X', false) => {
                let n = csi_param(&params, 0, 1) as usize;
                let row = self.cursor_row();
                let col = self.cursor_col();
                let bg = self.cursor.template.bg;
                let end = (col + n).min(self.cols);
                for c in col..end {
                    self.grid.cell_mut(row, c).reset_with_bg(bg);
                }
            }

            // --- SGR ---
            ('m', false) => {
                self.handle_sgr(&params);
            }

            // --- DEC Private Modes (set) ---
            ('h', true) => {
                for param in &params {
                    if let Some(&mode) = param.first() {
                        if mode == 1049 {
                            // Save cursor, switch to alt screen, clear
                            *self.saved_cursor = SavedCursor::save(self.cursor);
                            std::mem::swap(self.grid, self.alt_grid);
                            self.grid.clear(Color::Default);
                        }
                        self.modes.set_dec_mode(mode, true);
                    }
                }
            }
            // DEC Private Modes (reset)
            ('l', true) => {
                for param in &params {
                    if let Some(&mode) = param.first() {
                        if mode == 1049 {
                            // Switch back to primary screen, restore cursor
                            std::mem::swap(self.grid, self.alt_grid);
                            self.saved_cursor.restore(self.cursor);
                        }
                        self.modes.set_dec_mode(mode, false);
                    }
                }
            }
            // ANSI modes (set/reset)
            ('h', false) | ('l', false) => {
                // Standard ANSI modes — not commonly used, log for now
                tracing::debug!("ANSI mode set/reset: params={params:?} action={action}");
            }

            // --- Scroll Region ---
            // DECSTBM — Set Top and Bottom Margins
            ('r', false) => {
                let top = csi_param(&params, 0, 1) as usize - 1;
                let bottom = csi_param(&params, 1, self.rows as u16) as usize - 1;
                let bottom = bottom.min(self.rows - 1);
                if top < bottom {
                    *self.scroll_region = ScrollRegion::new(top, bottom);
                    // DECSTBM moves cursor to home
                    self.cursor.goto(0, 0, self.rows, self.cols);
                }
            }

            // --- Cursor Save/Restore (ANSI) ---
            ('s', false) => {
                *self.saved_cursor = SavedCursor::save(self.cursor);
            }
            ('u', false) => {
                self.saved_cursor.restore(self.cursor);
            }

            // --- Device Status / Cursor Position Report ---
            // DSR — Device Status Report (we can't respond without PTY write access,
            // but we note it)
            ('n', false) => {
                tracing::debug!("DSR request: params={params:?}");
            }

            // --- Tab Stops ---
            // TBC — Tab Clear
            ('g', false) => {
                tracing::debug!("Tab clear: params={params:?}");
            }

            // --- Window manipulation (xterm) ---
            ('t', false) => {
                tracing::debug!("Window manipulation: params={params:?}");
            }

            _ => {
                tracing::debug!(
                    "Unhandled CSI: action={action} intermediates={intermediates:?} params={params:?}",
                );
            }
        }
    }

    /// Handle an ESC dispatch.
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (byte, intermediates) {
            // DECSC — Save Cursor
            (b'7', []) => {
                *self.saved_cursor = SavedCursor::save(self.cursor);
            }
            // DECRC — Restore Cursor
            (b'8', []) => {
                self.saved_cursor.restore(self.cursor);
            }
            // RI — Reverse Index (move cursor up, scroll if needed)
            (b'M', []) => {
                self.reverse_linefeed();
            }
            // IND — Index (move cursor down, scroll if needed)
            (b'D', []) => {
                self.linefeed();
            }
            // NEL — Next Line
            (b'E', []) => {
                self.linefeed();
                self.cursor.pos.col = 0;
                self.cursor.pending_wrap = false;
            }
            // DECALN — Screen Alignment Pattern (fill screen with 'E')
            (b'8', [b'#']) => {
                for row in 0..self.rows {
                    for col in 0..self.cols {
                        self.grid.cell_mut(row, col).c = 'E';
                    }
                }
            }
            _ => {
                tracing::debug!(
                    "Unhandled ESC: byte=0x{byte:02x} ({}) intermediates={intermediates:?}",
                    byte as char
                );
            }
        }
    }

    /// Handle an OSC dispatch.
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let cmd = osc::parse_osc(params);
        match cmd {
            osc::OscCommand::SetTitle(title) | osc::OscCommand::SetWindowTitle(title) => {
                self.events.push(TerminalEvent::TitleChanged(title));
            }
            osc::OscCommand::SetIconName(_) => {} // Ignored on macOS
            osc::OscCommand::SetWorkingDirectory(dir) => {
                self.events
                    .push(TerminalEvent::WorkingDirectoryChanged(dir));
            }
            osc::OscCommand::Clipboard { clipboard, data } => {
                if data == "?" {
                    self.events.push(TerminalEvent::ClipboardLoad { clipboard });
                } else {
                    self.events
                        .push(TerminalEvent::ClipboardStore { clipboard, data });
                }
            }
            osc::OscCommand::ShellIntegration(mark) => {
                self.events.push(TerminalEvent::ShellIntegration(mark));
            }
            osc::OscCommand::SetHyperlink { .. } => {
                // TODO: track hyperlink state
            }
            osc::OscCommand::DefaultForeground(_)
            | osc::OscCommand::DefaultBackground(_)
            | osc::OscCommand::CursorColor(_) => {
                // TODO: dynamic color changes
            }
            osc::OscCommand::Unknown(_) => {}
        }
    }

    /// DCS hook — start of a DCS sequence. Ignored for now.
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }

    /// DCS put — data within a DCS sequence. Ignored.
    fn put(&mut self, _byte: u8) {}

    /// DCS unhook — end of a DCS sequence. Ignored.
    fn unhook(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_256() {
        let params = [5u16, 196];
        let (color, consumed) = parse_color(&params).unwrap();
        assert_eq!(color, Color::Indexed(196));
        assert_eq!(consumed, 2);
    }

    #[test]
    fn parse_color_rgb() {
        let params = [2u16, 255, 128, 0];
        let (color, consumed) = parse_color(&params).unwrap();
        assert_eq!(color, Color::Rgb(Rgb::new(255, 128, 0)));
        assert_eq!(consumed, 4);
    }

    #[test]
    fn csi_param_default() {
        let params: Vec<Vec<u16>> = vec![vec![0], vec![5]];
        assert_eq!(csi_param(&params, 0, 1), 1); // 0 maps to default
        assert_eq!(csi_param(&params, 1, 1), 5);
        assert_eq!(csi_param(&params, 2, 1), 1); // Missing → default
    }
}
