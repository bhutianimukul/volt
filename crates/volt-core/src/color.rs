//! Color types — indexed (0-255), RGB, and named ANSI colors.
//!
//! Colors follow the standard terminal color model:
//! - 0-7: standard ANSI colors
//! - 8-15: bright ANSI colors
//! - 16-231: 216-color cube (6x6x6)
//! - 232-255: 24-step grayscale

/// A terminal color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// Use the terminal's default foreground or background.
    Default,
    /// One of the 16 named ANSI colors.
    Named(NamedColor),
    /// 256-color palette index (0-255).
    Indexed(u8),
    /// 24-bit true color.
    Rgb(Rgb),
}

impl Default for Color {
    fn default() -> Self {
        Self::Default
    }
}

/// 24-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// The 16 standard ANSI colors.
///
/// Values 0-7 are standard, 8-15 are bright variants.
/// The numeric value matches the SGR color index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NamedColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

impl NamedColor {
    /// Convert to 256-color palette index.
    pub const fn to_index(self) -> u8 {
        self as u8
    }

    /// Convert a named color to its bright variant.
    pub const fn to_bright(self) -> Self {
        match self {
            Self::Black => Self::BrightBlack,
            Self::Red => Self::BrightRed,
            Self::Green => Self::BrightGreen,
            Self::Yellow => Self::BrightYellow,
            Self::Blue => Self::BrightBlue,
            Self::Magenta => Self::BrightMagenta,
            Self::Cyan => Self::BrightCyan,
            Self::White => Self::BrightWhite,
            bright => bright, // Already bright
        }
    }
}

/// Default 256-color palette.
///
/// Indices 0-15 are set by the theme. 16-231 are the 6x6x6 color cube.
/// 232-255 are the grayscale ramp.
pub fn default_indexed_color(index: u8) -> Rgb {
    match index {
        // 16-231: 6x6x6 color cube
        16..=231 => {
            let idx = index - 16;
            let b = idx % 6;
            let g = (idx / 6) % 6;
            let r = idx / 36;

            let component = |c: u8| -> u8 { if c == 0 { 0 } else { 55 + 40 * c } };

            Rgb::new(component(r), component(g), component(b))
        }
        // 232-255: grayscale ramp
        232..=255 => {
            let value = 8 + 10 * (index - 232);
            Rgb::new(value, value, value)
        }
        // 0-15: theme-dependent, return a sensible default
        _ => default_ansi_color(index),
    }
}

/// Default ANSI colors (indices 0-15). Matches xterm defaults.
fn default_ansi_color(index: u8) -> Rgb {
    match index {
        0 => Rgb::new(0, 0, 0),        // Black
        1 => Rgb::new(205, 0, 0),      // Red
        2 => Rgb::new(0, 205, 0),      // Green
        3 => Rgb::new(205, 205, 0),    // Yellow
        4 => Rgb::new(0, 0, 238),      // Blue
        5 => Rgb::new(205, 0, 205),    // Magenta
        6 => Rgb::new(0, 205, 205),    // Cyan
        7 => Rgb::new(229, 229, 229),  // White
        8 => Rgb::new(127, 127, 127),  // Bright Black
        9 => Rgb::new(255, 0, 0),      // Bright Red
        10 => Rgb::new(0, 255, 0),     // Bright Green
        11 => Rgb::new(255, 255, 0),   // Bright Yellow
        12 => Rgb::new(92, 92, 255),   // Bright Blue
        13 => Rgb::new(255, 0, 255),   // Bright Magenta
        14 => Rgb::new(0, 255, 255),   // Bright Cyan
        15 => Rgb::new(255, 255, 255), // Bright White
        _ => Rgb::new(0, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_cube_boundaries() {
        // First color in cube (index 16) = rgb(0,0,0)
        assert_eq!(default_indexed_color(16), Rgb::new(0, 0, 0));
        // Last color in cube (index 231) = rgb(255,255,255)
        assert_eq!(default_indexed_color(231), Rgb::new(255, 255, 255));
        // Index 196 = pure red (5,0,0) = rgb(255,0,0)
        assert_eq!(default_indexed_color(196), Rgb::new(255, 0, 0));
    }

    #[test]
    fn grayscale_ramp() {
        assert_eq!(default_indexed_color(232), Rgb::new(8, 8, 8));
        assert_eq!(default_indexed_color(255), Rgb::new(238, 238, 238));
    }

    #[test]
    fn named_color_bright() {
        assert_eq!(NamedColor::Red.to_bright(), NamedColor::BrightRed);
        assert_eq!(NamedColor::BrightRed.to_bright(), NamedColor::BrightRed);
    }
}
