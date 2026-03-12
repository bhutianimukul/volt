//! NSView subclass hosting the CAMetalLayer for terminal rendering.
//!
//! `VoltMetalView` is an NSView that:
//! - Creates a CAMetalLayer as its backing layer (for Metal rendering)
//! - Accepts first responder to receive keyboard events
//! - Translates key events and writes them to the PTY
//! - Handles resize by updating terminal dimensions
//!
//! Rendering is driven externally by CAMetalDisplayLink (see `app.rs`).

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSView};
use objc2_foundation::{NSObjectProtocol, NSSize};
use objc2_quartz_core::{CALayer, CAMetalLayer};

use crate::app;
use crate::event;

define_class!(
    /// Terminal view backed by a CAMetalLayer.
    ///
    /// SAFETY: NSView does not have subclassing requirements beyond
    /// implementing methods correctly, which we do.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "VoltMetalView"]
    pub(crate) struct MetalView;

    unsafe impl NSObjectProtocol for MetalView {}

    impl MetalView {
        /// Return a CAMetalLayer as the backing layer instead of a plain CALayer.
        #[unsafe(method_id(makeBackingLayer))]
        fn make_backing_layer(&self) -> Retained<CALayer> {
            let layer = CAMetalLayer::new();
            // SAFETY: CAMetalLayer is a subclass of CALayer, upcast is safe.
            Retained::into_super(layer)
        }

        /// Use layer-backed rendering (required for Metal).
        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            true
        }

        /// Accept first responder to receive key events.
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        /// Handle key press: check for Cmd+key shortcuts first, then translate to PTY bytes.
        #[unsafe(method(keyDown:))]
        fn key_down(&self, ns_event: &NSEvent) {
            let key_code = ns_event.keyCode();
            let characters = ns_event.characters().map(|s| s.to_string());
            let chars_no_mods = ns_event
                .charactersIgnoringModifiers()
                .map(|s| s.to_string());
            let modifiers = ns_event.modifierFlags();

            // Intercept Cmd+key shortcuts before passing to terminal
            if modifiers.contains(NSEventModifierFlags::Command) {
                if let Some(key_char) = chars_no_mods.as_deref() {
                    let has_shift = modifiers.contains(NSEventModifierFlags::Shift);
                    if app::handle_command_key(key_char, has_shift) {
                        return;
                    }
                }
            }

            if let Some(bytes) = event::translate_key(
                key_code,
                characters.as_deref(),
                chars_no_mods.as_deref(),
                modifiers,
            ) {
                app::write_to_pty(&bytes);
            }
        }

        /// Suppress system beep for unhandled keys.
        #[unsafe(method(keyUp:))]
        fn key_up(&self, _event: &NSEvent) {}

        /// Track modifier key changes (Shift, Ctrl, Option, Cmd).
        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, _event: &NSEvent) {
            // Modifier-only events don't generate PTY input in v0.1.
        }

        /// Handle resize: update terminal grid and renderer viewport.
        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, new_size: NSSize) {
            // SAFETY: Calling super's setFrameSize to maintain NSView contract.
            let _: () = unsafe { msg_send![super(self), setFrameSize: new_size] };

            app::handle_resize(new_size.width, new_size.height);
        }
    }
);

impl MetalView {
    /// Create a new MetalView with the given frame.
    pub fn new(mtm: MainThreadMarker, frame: objc2_foundation::NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view.setWantsLayer(true);
        view
    }
}
