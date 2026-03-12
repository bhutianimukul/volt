//! Event handling — keyboard input, key binding resolution, and input dispatch.
//!
//! Translates macOS NSEvent key events into byte sequences for the PTY.
//! Handles special keys (arrows, function keys), Ctrl+key control codes,
//! and Option-as-Meta (ESC prefix) behavior.

use objc2_app_kit::NSEventModifierFlags;

// macOS virtual key codes (hardware-independent)
const KEY_RETURN: u16 = 36;
const KEY_TAB: u16 = 48;
const KEY_SPACE: u16 = 49;
const KEY_DELETE: u16 = 51; // Backspace
const KEY_ESCAPE: u16 = 53;
const KEY_FORWARD_DELETE: u16 = 117;
const KEY_HOME: u16 = 115;
const KEY_END: u16 = 119;
const KEY_PAGE_UP: u16 = 116;
const KEY_PAGE_DOWN: u16 = 121;
const KEY_LEFT: u16 = 123;
const KEY_RIGHT: u16 = 124;
const KEY_DOWN: u16 = 125;
const KEY_UP: u16 = 126;
const KEY_F1: u16 = 122;
const KEY_F2: u16 = 120;
const KEY_F3: u16 = 99;
const KEY_F4: u16 = 118;
const KEY_F5: u16 = 96;
const KEY_F6: u16 = 97;
const KEY_F7: u16 = 98;
const KEY_F8: u16 = 100;
const KEY_F9: u16 = 101;
const KEY_F10: u16 = 109;
const KEY_F11: u16 = 103;
const KEY_F12: u16 = 111;

/// Translate a macOS key event into bytes to write to the PTY.
///
/// - `key_code`: Hardware virtual key code from NSEvent
/// - `characters`: The character(s) produced by this key event (with modifiers applied)
/// - `chars_no_modifiers`: Characters without modifier translation (for Option-as-Meta)
/// - `modifiers`: Current modifier flags
/// - `option_as_meta`: Whether Option key should act as Meta (ESC prefix)
pub fn translate_key(
    key_code: u16,
    characters: Option<&str>,
    chars_no_modifiers: Option<&str>,
    modifiers: NSEventModifierFlags,
) -> Option<Vec<u8>> {
    let has_ctrl = modifiers.contains(NSEventModifierFlags::Control);
    let has_option = modifiers.contains(NSEventModifierFlags::Option);
    let has_cmd = modifiers.contains(NSEventModifierFlags::Command);

    // Cmd+key combos are handled by the menu system, not the terminal
    if has_cmd {
        return None;
    }

    // Special keys by key code
    if let Some(bytes) = translate_special_key(key_code, has_ctrl) {
        return Some(bytes);
    }

    // Ctrl+key → control codes
    if has_ctrl {
        if let Some(base) = chars_no_modifiers.and_then(|s| s.chars().next()) {
            if let Some(code) = ctrl_code(base) {
                return Some(vec![code]);
            }
        }
    }

    // Option-as-Meta: prepend ESC to the base character
    if has_option {
        if let Some(base) = chars_no_modifiers.and_then(|s| s.chars().next()) {
            let mut bytes = vec![0x1B];
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(base.encode_utf8(&mut buf).as_bytes());
            return Some(bytes);
        }
    }

    // Regular character input
    let chars = characters?;
    if chars.is_empty() {
        return None;
    }
    Some(chars.as_bytes().to_vec())
}

/// Translate special keys (arrows, function keys, etc.) to escape sequences.
fn translate_special_key(key_code: u16, _has_ctrl: bool) -> Option<Vec<u8>> {
    match key_code {
        KEY_RETURN => Some(b"\r".to_vec()),
        KEY_TAB => Some(b"\t".to_vec()),
        KEY_DELETE => Some(vec![0x7F]),
        KEY_ESCAPE => Some(vec![0x1B]),
        KEY_SPACE => None, // Let regular character handling deal with space

        // Arrow keys (normal mode — application mode uses ESC O A/B/C/D)
        KEY_UP => Some(b"\x1b[A".to_vec()),
        KEY_DOWN => Some(b"\x1b[B".to_vec()),
        KEY_RIGHT => Some(b"\x1b[C".to_vec()),
        KEY_LEFT => Some(b"\x1b[D".to_vec()),

        // Navigation
        KEY_HOME => Some(b"\x1b[H".to_vec()),
        KEY_END => Some(b"\x1b[F".to_vec()),
        KEY_PAGE_UP => Some(b"\x1b[5~".to_vec()),
        KEY_PAGE_DOWN => Some(b"\x1b[6~".to_vec()),
        KEY_FORWARD_DELETE => Some(b"\x1b[3~".to_vec()),

        // Function keys
        KEY_F1 => Some(b"\x1bOP".to_vec()),
        KEY_F2 => Some(b"\x1bOQ".to_vec()),
        KEY_F3 => Some(b"\x1bOR".to_vec()),
        KEY_F4 => Some(b"\x1bOS".to_vec()),
        KEY_F5 => Some(b"\x1b[15~".to_vec()),
        KEY_F6 => Some(b"\x1b[17~".to_vec()),
        KEY_F7 => Some(b"\x1b[18~".to_vec()),
        KEY_F8 => Some(b"\x1b[19~".to_vec()),
        KEY_F9 => Some(b"\x1b[20~".to_vec()),
        KEY_F10 => Some(b"\x1b[21~".to_vec()),
        KEY_F11 => Some(b"\x1b[23~".to_vec()),
        KEY_F12 => Some(b"\x1b[24~".to_vec()),

        _ => None,
    }
}

/// Convert a character to its Ctrl+key control code (0x01–0x1A).
fn ctrl_code(c: char) -> Option<u8> {
    match c {
        'a'..='z' => Some(c as u8 - b'a' + 1),
        'A'..='Z' => Some(c as u8 - b'A' + 1),
        '[' | '3' => Some(0x1B), // Ctrl+[ = ESC
        '\\' | '4' => Some(0x1C),
        ']' | '5' => Some(0x1D),
        '^' | '6' => Some(0x1E),
        '_' | '7' => Some(0x1F),
        ' ' | '2' => Some(0x00), // Ctrl+Space = NUL
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_mods() -> NSEventModifierFlags {
        NSEventModifierFlags::empty()
    }

    #[test]
    fn regular_character() {
        let result = translate_key(0, Some("a"), Some("a"), no_mods());
        assert_eq!(result, Some(b"a".to_vec()));
    }

    #[test]
    fn return_key() {
        let result = translate_key(KEY_RETURN, Some("\r"), Some("\r"), no_mods());
        assert_eq!(result, Some(b"\r".to_vec()));
    }

    #[test]
    fn arrow_keys() {
        assert_eq!(
            translate_key(KEY_UP, None, None, no_mods()),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            translate_key(KEY_DOWN, None, None, no_mods()),
            Some(b"\x1b[B".to_vec())
        );
    }

    #[test]
    fn ctrl_c() {
        let result = translate_key(
            8, // 'c' key code
            Some("\x03"),
            Some("c"),
            NSEventModifierFlags::Control,
        );
        assert_eq!(result, Some(vec![0x03]));
    }

    #[test]
    fn option_as_meta() {
        let result = translate_key(
            0,
            Some("å"), // Option+a on US keyboard
            Some("a"),
            NSEventModifierFlags::Option,
        );
        // Should send ESC + 'a'
        assert_eq!(result, Some(vec![0x1B, b'a']));
    }

    #[test]
    fn cmd_key_returns_none() {
        let result = translate_key(
            8, // 'c' key
            Some("c"),
            Some("c"),
            NSEventModifierFlags::Command,
        );
        assert_eq!(result, None);
    }
}
