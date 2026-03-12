//! NSWindow management and delegate.
//!
//! Creates the terminal window with appropriate style mask, title bar,
//! and content view (MetalView with CAMetalLayer).

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::config::WindowConfig;
use crate::view::MetalView;

/// Create the main terminal window with a MetalView as its content view.
///
/// The window is created but NOT shown — caller must call `makeKeyAndOrderFront`.
pub fn create_window(
    mtm: MainThreadMarker,
    window_config: &WindowConfig,
) -> (Retained<NSWindow>, Retained<MetalView>) {
    let content_rect = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(window_config.width, window_config.height),
    );

    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;

    // defer: true — don't create window server resources until needed (prevents premature display)
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            content_rect,
            style,
            NSBackingStoreType::Buffered,
            true,
        )
    };

    window.setTitle(&NSString::from_str(&window_config.title));
    window.center();
    // SAFETY: We hold a Retained<NSWindow>, so the window won't be prematurely released.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setAcceptsMouseMovedEvents(true);
    // Disable macOS window state restoration (prevents ghost windows on relaunch)
    window.setRestorable(false);

    // Create the Metal-backed view as the content view
    let view = MetalView::new(mtm, content_rect);
    window.setContentView(Some(&view));

    // NOTE: Do NOT call toggleFullScreen here — APP state isn't stored yet,
    // so the resize handler in app::handle_resize would be a no-op.
    // Fullscreen is triggered in run_app() after state initialization.

    (window, view)
}
