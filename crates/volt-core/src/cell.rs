//! Cell representation — character, foreground/background color, and attribute flags.
//!
//! Each cell in the grid holds a single character (or wide-char spacer), colors,
//! and styling flags. Following Alacritty's pattern, rare attributes (underline
//! color, hyperlinks) are stored in an optional `CellExtra` to keep the common
//! case small.
//!
//! Memory layout target: 16 bytes per cell for the common case.

use crate::color::Color;

/// Attribute flags for a terminal cell, stored as bitflags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellFlags(u32);

impl CellFlags {
    pub const NONE: Self = Self(0);
    pub const BOLD: Self = Self(1 << 0);
    pub const DIM: Self = Self(1 << 1);
    pub const ITALIC: Self = Self(1 << 2);
    pub const UNDERLINE: Self = Self(1 << 3);
    pub const DOUBLE_UNDERLINE: Self = Self(1 << 4);
    pub const CURLY_UNDERLINE: Self = Self(1 << 5);
    pub const DOTTED_UNDERLINE: Self = Self(1 << 6);
    pub const DASHED_UNDERLINE: Self = Self(1 << 7);
    pub const BLINK: Self = Self(1 << 8);
    pub const INVERSE: Self = Self(1 << 9);
    pub const HIDDEN: Self = Self(1 << 10);
    pub const STRIKETHROUGH: Self = Self(1 << 11);
    pub const WIDE_CHAR: Self = Self(1 << 12);
    pub const WIDE_CHAR_SPACER: Self = Self(1 << 13);
    pub const WRAPLINE: Self = Self(1 << 14);
    pub const HYPERLINK: Self = Self(1 << 15);

    /// Mask covering all underline styles.
    pub const ALL_UNDERLINES: Self = Self(
        Self::UNDERLINE.0
            | Self::DOUBLE_UNDERLINE.0
            | Self::CURLY_UNDERLINE.0
            | Self::DOTTED_UNDERLINE.0
            | Self::DASHED_UNDERLINE.0,
    );

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    #[inline]
    pub const fn insert(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub const fn remove(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Extra cell data for rare attributes. Boxed to keep common Cell size small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellExtra {
    /// Colored underline (SGR 58/59).
    pub underline_color: Option<Color>,
    /// Hyperlink URI (OSC 8).
    pub hyperlink: Option<String>,
}

/// A single terminal cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed in this cell. '\0' for empty, ' ' for blank.
    /// Wide characters store the char in the first cell; the second cell is a
    /// spacer with `c: ' '` and `WIDE_CHAR_SPACER` flag.
    pub c: char,
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Attribute flags.
    pub flags: CellFlags,
    /// Rare attributes (underline color, hyperlink). None for most cells.
    pub extra: Option<Box<CellExtra>>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: Color::Default,
            bg: Color::Default,
            flags: CellFlags::NONE,
            extra: None,
        }
    }
}

impl Cell {
    /// Create a cell with a character and default colors.
    pub fn new(c: char) -> Self {
        Self {
            c,
            ..Default::default()
        }
    }

    /// Reset this cell to a blank space with default attributes.
    pub fn reset(&mut self) {
        self.c = ' ';
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.flags = CellFlags::NONE;
        self.extra = None;
    }

    /// Reset this cell to a blank space but preserve the given background color.
    /// Used for erasing with the current SGR background.
    pub fn reset_with_bg(&mut self, bg: Color) {
        self.c = ' ';
        self.fg = Color::Default;
        self.bg = bg;
        self.flags = CellFlags::NONE;
        self.extra = None;
    }

    /// Whether this cell is visually empty (space with default colors, no styling).
    pub fn is_empty(&self) -> bool {
        self.c == ' '
            && self.fg == Color::Default
            && self.bg == Color::Default
            && self.flags.is_empty()
    }

    /// Whether this cell is a wide character (first cell of a double-width char).
    pub fn is_wide(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CHAR)
    }

    /// Whether this cell is a spacer for a wide character (second cell).
    pub fn is_wide_spacer(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CHAR_SPACER)
    }

    /// Get the underline color, if set via SGR 58.
    pub fn underline_color(&self) -> Option<Color> {
        self.extra.as_ref().and_then(|e| e.underline_color)
    }

    /// Set the underline color (SGR 58). Allocates CellExtra if needed.
    pub fn set_underline_color(&mut self, color: Color) {
        self.extra
            .get_or_insert_with(|| {
                Box::new(CellExtra {
                    underline_color: None,
                    hyperlink: None,
                })
            })
            .underline_color = Some(color);
    }

    /// Set a hyperlink URI (OSC 8). Allocates CellExtra if needed.
    pub fn set_hyperlink(&mut self, uri: String) {
        self.flags = self.flags.insert(CellFlags::HYPERLINK);
        self.extra
            .get_or_insert_with(|| {
                Box::new(CellExtra {
                    underline_color: None,
                    hyperlink: None,
                })
            })
            .hyperlink = Some(uri);
    }
}

/// Template for new cells — stores the "current SGR state" that gets applied
/// to each character as it's printed.
#[derive(Debug, Clone)]
pub struct CellTemplate {
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
    pub underline_color: Option<Color>,
}

impl Default for CellTemplate {
    fn default() -> Self {
        Self {
            fg: Color::Default,
            bg: Color::Default,
            flags: CellFlags::NONE,
            underline_color: None,
        }
    }
}

impl CellTemplate {
    /// Apply this template to a cell, setting its character.
    pub fn apply(&self, cell: &mut Cell, c: char) {
        cell.c = c;
        cell.fg = self.fg;
        cell.bg = self.bg;
        cell.flags = self.flags;
        if let Some(color) = self.underline_color {
            cell.set_underline_color(color);
        } else {
            // Clear extra if no special attributes
            if cell.extra.as_ref().is_some_and(|e| e.hyperlink.is_none()) {
                cell.extra = None;
            }
        }
    }

    /// Reset to default SGR state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_empty() {
        let cell = Cell::default();
        assert!(cell.is_empty());
        assert_eq!(cell.c, ' ');
    }

    #[test]
    fn cell_flags_operations() {
        let flags = CellFlags::BOLD.insert(CellFlags::ITALIC);
        assert!(flags.contains(CellFlags::BOLD));
        assert!(flags.contains(CellFlags::ITALIC));
        assert!(!flags.contains(CellFlags::UNDERLINE));

        let flags = flags.remove(CellFlags::BOLD);
        assert!(!flags.contains(CellFlags::BOLD));
        assert!(flags.contains(CellFlags::ITALIC));
    }

    #[test]
    fn underline_mask() {
        let flags = CellFlags::CURLY_UNDERLINE;
        assert!(flags.intersects(CellFlags::ALL_UNDERLINES));
        assert!(!CellFlags::BOLD.intersects(CellFlags::ALL_UNDERLINES));
    }

    #[test]
    fn cell_extra_lazy_allocation() {
        let mut cell = Cell::default();
        assert!(cell.extra.is_none());

        cell.set_underline_color(Color::Rgb(crate::color::Rgb::new(255, 0, 0)));
        assert!(cell.extra.is_some());
        assert_eq!(
            cell.underline_color(),
            Some(Color::Rgb(crate::color::Rgb::new(255, 0, 0)))
        );
    }

    #[test]
    fn cell_template_apply() {
        let mut template = CellTemplate::default();
        template.fg = Color::Named(crate::color::NamedColor::Red);
        template.flags = CellFlags::BOLD;

        let mut cell = Cell::default();
        template.apply(&mut cell, 'A');

        assert_eq!(cell.c, 'A');
        assert_eq!(cell.fg, Color::Named(crate::color::NamedColor::Red));
        assert!(cell.flags.contains(CellFlags::BOLD));
    }
}
