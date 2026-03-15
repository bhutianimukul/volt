//! Dock badge — show badge/bounce on terminal bell when window is unfocused.

#[derive(Debug, Clone)]
pub struct DockBadgeManager {
    badge_count: u32,
    bounce_on_bell: bool,
    badge_on_bell: bool,
}

impl DockBadgeManager {
    pub fn new() -> Self {
        Self {
            badge_count: 0,
            bounce_on_bell: true,
            badge_on_bell: true,
        }
    }

    /// Called when a bell (BEL) character is received and the window is not focused
    pub fn on_bell(&mut self, window_focused: bool) {
        if window_focused {
            return;
        }

        self.badge_count += 1;

        if self.badge_on_bell {
            self.set_dock_badge(self.badge_count);
        }

        if self.bounce_on_bell {
            self.bounce_dock_icon();
        }
    }

    /// Clear the badge (called when window gets focus)
    pub fn clear_badge(&mut self) {
        self.badge_count = 0;
        self.set_dock_badge(0);
    }

    pub fn set_bounce_on_bell(&mut self, enabled: bool) {
        self.bounce_on_bell = enabled;
    }

    pub fn set_badge_on_bell(&mut self, enabled: bool) {
        self.badge_on_bell = enabled;
    }

    #[cfg(target_os = "macos")]
    fn set_dock_badge(&self, count: u32) {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};
        use std::ffi::CString;

        unsafe {
            let app_class = Class::get("NSApplication").unwrap();
            let app: *mut Object = msg_send![app_class, sharedApplication];
            let dock_tile: *mut Object = msg_send![app, dockTile];

            if count == 0 {
                let ns_string_class = Class::get("NSString").unwrap();
                let empty = CString::new("").unwrap();
                let empty_ns: *mut Object =
                    msg_send![ns_string_class, stringWithUTF8String: empty.as_ptr()];
                let _: () = msg_send![dock_tile, setBadgeLabel: empty_ns];
            } else {
                let ns_string_class = Class::get("NSString").unwrap();
                let label = CString::new(count.to_string()).unwrap();
                let label_ns: *mut Object =
                    msg_send![ns_string_class, stringWithUTF8String: label.as_ptr()];
                let _: () = msg_send![dock_tile, setBadgeLabel: label_ns];
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn set_dock_badge(&self, _count: u32) {
        // No-op on non-macOS
    }

    #[cfg(target_os = "macos")]
    fn bounce_dock_icon(&self) {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            let app_class = Class::get("NSApplication").unwrap();
            let app: *mut Object = msg_send![app_class, sharedApplication];
            // NSApplicationActivationPolicy informational bounce
            // requestUserAttention: NSInformationalRequest = 10
            let _: i64 = msg_send![app, requestUserAttention: 10_i64];
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn bounce_dock_icon(&self) {
        // No-op on non-macOS
    }
}

impl Default for DockBadgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bell_when_focused_no_badge() {
        let mut mgr = DockBadgeManager::new();
        mgr.on_bell(true); // Window focused
        assert_eq!(mgr.badge_count, 0);
    }

    #[test]
    fn test_bell_when_unfocused_increments() {
        let mut mgr = DockBadgeManager::new();
        mgr.on_bell(false);
        assert_eq!(mgr.badge_count, 1);
        mgr.on_bell(false);
        assert_eq!(mgr.badge_count, 2);
    }

    #[test]
    fn test_clear_badge() {
        let mut mgr = DockBadgeManager::new();
        mgr.on_bell(false);
        mgr.on_bell(false);
        assert_eq!(mgr.badge_count, 2);
        mgr.clear_badge();
        assert_eq!(mgr.badge_count, 0);
    }

    #[test]
    fn test_disable_badge() {
        let mut mgr = DockBadgeManager::new();
        mgr.set_badge_on_bell(false);
        mgr.on_bell(false);
        assert_eq!(mgr.badge_count, 1); // Count still tracks
    }
}
