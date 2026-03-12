//! NSApplication setup, delegates, and main event loop.
//!
//! Wires together: NSApplication → AppDelegate → Window → MetalView
//! + CAMetalDisplayLink → RenderDelegate → Renderer + Terminal + PTY.
//!
//! All mutable state lives in a thread-local `APP` cell, accessed from both
//! the MetalView (keyboard events) and the RenderDelegate (vsync rendering).
//! This is safe because both run on the main thread (enforced by MainThreadOnly).

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSEventModifierFlags,
    NSMenu, NSMenuItem,
};
use objc2_foundation::{
    NSDefaultRunLoopMode, NSNotification, NSObjectProtocol, NSRunLoop, NSString, ns_string,
};
use objc2_metal::MTLCreateSystemDefaultDevice;
use objc2_quartz_core::{
    CAMetalDisplayLink, CAMetalDisplayLinkDelegate, CAMetalDisplayLinkUpdate, CAMetalDrawable,
    CAMetalLayer,
};

use volt_core::{TermSize, Terminal};
use volt_pty::reader::PtyRead;
use volt_pty::{PtyConfig, PtyHandle, PtySize};
use volt_renderer::Renderer;

use crate::config::VoltConfig;
use crate::window;

/// Minimum font size (points).
const MIN_FONT_SIZE: f32 = 8.0;
/// Maximum font size (points).
const MAX_FONT_SIZE: f32 = 72.0;
/// Font size step for zoom in/out.
const FONT_SIZE_STEP: f32 = 1.0;

/// All mutable application state. Lives in a thread-local because both the
/// view's keyboard handler and the display link's render callback need it,
/// and both run on the main thread.
struct AppState {
    terminal: Terminal,
    renderer: Renderer,
    pty_handle: PtyHandle,
    window: Retained<objc2_app_kit::NSWindow>,
    metal_layer: Retained<CAMetalLayer>,
    // Prevent deallocation — display link delegate is weak-referenced by CAMetalDisplayLink
    _display_link: Retained<CAMetalDisplayLink>,
    _render_delegate: Retained<RenderDelegate>,
    bg_color: [f32; 3],
    scale: f64,
    /// Current font size in points (for zoom).
    current_font_size: f32,
    /// Default font size from config (for Cmd+0 reset).
    default_font_size: f32,
    /// Line height multiplier from config.
    line_height_multiplier: f32,
    /// Font family name (for re-creating text system on zoom).
    _font_family: Option<String>,
    /// Set to true when app is terminating to avoid double-terminate.
    terminating: bool,
}

thread_local! {
    static APP: RefCell<Option<Box<AppState>>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// Public helpers called from MetalView (view.rs)
// ---------------------------------------------------------------------------

/// Write bytes to the PTY (called from view.rs on key events).
pub fn write_to_pty(bytes: &[u8]) {
    APP.with(|cell| {
        let borrow = cell.borrow();
        if let Some(state) = borrow.as_ref() {
            if let Err(e) = state.pty_handle.write(bytes) {
                tracing::error!("PTY write failed: {e}");
            }
        }
    });
}

/// Handle window resize (called from view.rs on setFrameSize).
pub fn handle_resize(width: f64, height: f64) {
    APP.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            let scale = state.scale;
            let px_w = width * scale;
            let px_h = height * scale;

            let cell_m = state.renderer.cell_metrics();
            let cols = (px_w as f32 / cell_m.width).max(1.0) as u16;
            let rows = (px_h as f32 / cell_m.height).max(1.0) as u16;

            state.terminal.resize(TermSize { cols, rows });
            state.renderer.set_viewport(px_w as f32, px_h as f32);

            // Update CAMetalLayer drawable size (physical pixels)
            state.metal_layer.setDrawableSize(objc2_foundation::NSSize {
                width: px_w,
                height: px_h,
            });

            // Tell the shell about the new size
            let _ = state.pty_handle.resize(PtySize {
                rows,
                cols,
                pixel_width: px_w as u16,
                pixel_height: px_h as u16,
            });

            tracing::debug!("resized to {cols}x{rows} ({px_w}x{px_h}px)");
        }
    });
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts — Cmd+key handling (iTerm2-style)
// ---------------------------------------------------------------------------

/// Handle a Cmd+key shortcut. Returns `true` if the key was handled.
///
/// Called from MetalView::keyDown before falling through to translate_key.
pub fn handle_command_key(key_char: &str, has_shift: bool) -> bool {
    tracing::debug!("handle_command_key: char={key_char:?} shift={has_shift}");
    match key_char {
        "v" => {
            paste_from_clipboard();
            true
        }
        "c" => {
            copy_to_clipboard();
            true
        }
        "k" => {
            clear_scrollback();
            true
        }
        "l" => {
            // Cmd+L: send Ctrl+L (form feed) to clear the screen
            write_to_pty(&[0x0C]);
            true
        }
        "w" => {
            close_window();
            true
        }
        "=" | "+" => {
            zoom_in();
            true
        }
        "-" => {
            zoom_out();
            true
        }
        "0" => {
            zoom_reset();
            true
        }
        "\r" => {
            // Cmd+Enter: toggle fullscreen (iTerm2 style)
            toggle_fullscreen();
            true
        }
        "n" => {
            // Cmd+N: new window (placeholder for Phase 2)
            tracing::info!("Cmd+N: new window not yet implemented");
            true
        }
        "t" => {
            // Cmd+T: new tab (placeholder for Phase 2)
            tracing::info!("Cmd+T: new tab not yet implemented");
            true
        }
        "f" => {
            // Cmd+F: find (placeholder for Phase 2)
            tracing::info!("Cmd+F: find not yet implemented");
            true
        }
        "d" => {
            if has_shift {
                // Cmd+Shift+D: split horizontally (placeholder)
                tracing::info!("Cmd+Shift+D: horizontal split not yet implemented");
            } else {
                // Cmd+D: split vertically (placeholder)
                tracing::info!("Cmd+D: vertical split not yet implemented");
            }
            true
        }
        "," => {
            // Cmd+,: preferences (placeholder)
            tracing::info!("Cmd+,: preferences not yet implemented");
            true
        }
        _ => false,
    }
}

/// Paste system clipboard contents into the PTY.
///
/// Wraps in bracketed paste sequences if the terminal has bracketed paste mode enabled.
fn paste_from_clipboard() {
    // Read clipboard text via msg_send! to avoid type mismatches with
    // NSPasteboard::stringForType in objc2-app-kit bindings.
    let text: Option<String> = unsafe {
        let pb: &objc2_foundation::NSObject =
            msg_send![objc2::class!(NSPasteboard), generalPasteboard];
        let ns_str: Option<&NSString> = msg_send![
            pb,
            stringForType: ns_string!("public.utf8-plain-text")
        ];
        ns_str.map(|s| s.to_string())
    };

    let Some(text) = text else {
        tracing::debug!("clipboard empty or no string type");
        return;
    };
    if text.is_empty() {
        return;
    }

    tracing::debug!("pasting {} bytes", text.len());

    APP.with(|cell| {
        let borrow = cell.borrow();
        let Some(state) = borrow.as_ref() else { return };

        let bracketed = state.terminal.modes().bracketed_paste;
        if bracketed {
            let _ = state.pty_handle.write(b"\x1b[200~");
        }
        let _ = state.pty_handle.write(text.as_bytes());
        if bracketed {
            let _ = state.pty_handle.write(b"\x1b[201~");
        }
    });
}

/// Copy terminal selection to system clipboard.
///
/// Currently a placeholder — selection is not yet implemented (Phase 2).
fn copy_to_clipboard() {
    // TODO: implement when selection support is added
    tracing::debug!("Cmd+C: copy (no selection yet)");
}

/// Clear the terminal scrollback buffer and visible screen.
fn clear_scrollback() {
    APP.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            state.terminal.clear_scrollback();
            // Also send Ctrl+L to redraw the prompt
            let _ = state.pty_handle.write(&[0x0C]);
            tracing::debug!("cleared scrollback");
        }
    });
}

/// Close the current window.
fn close_window() {
    APP.with(|cell| {
        let borrow = cell.borrow();
        if let Some(state) = borrow.as_ref() {
            state.window.performClose(None);
        }
    });
}

/// Toggle native macOS fullscreen.
fn toggle_fullscreen() {
    APP.with(|cell| {
        let borrow = cell.borrow();
        if let Some(state) = borrow.as_ref() {
            state.window.toggleFullScreen(None);
        }
    });
}

/// Increase font size by one step.
fn zoom_in() {
    change_font_size(FONT_SIZE_STEP);
}

/// Decrease font size by one step.
fn zoom_out() {
    change_font_size(-FONT_SIZE_STEP);
}

/// Reset font size to default.
fn zoom_reset() {
    APP.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            state.current_font_size = state.default_font_size;
            apply_font_size(state);
        }
    });
}

/// Change font size by a delta and recompute terminal grid.
fn change_font_size(delta: f32) {
    APP.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            let new_size = (state.current_font_size + delta).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
            if (new_size - state.current_font_size).abs() < 0.01 {
                return;
            }
            state.current_font_size = new_size;
            apply_font_size(state);
        }
    });
}

/// Apply the current font size to the renderer and recompute grid dimensions.
fn apply_font_size(state: &mut AppState) {
    // Scale point size to physical pixels for Retina
    let physical_size = state.current_font_size * state.scale as f32;
    let physical_line_height = physical_size * state.line_height_multiplier;
    state
        .renderer
        .set_font_size(physical_size, physical_line_height);

    // Recompute terminal grid from current drawable size
    let drawable_size = state.metal_layer.drawableSize();
    let cell_m = state.renderer.cell_metrics();
    let cols = (drawable_size.width as f32 / cell_m.width).max(1.0) as u16;
    let rows = (drawable_size.height as f32 / cell_m.height).max(1.0) as u16;

    state.terminal.resize(TermSize { cols, rows });
    let _ = state.pty_handle.resize(PtySize {
        rows,
        cols,
        pixel_width: drawable_size.width as u16,
        pixel_height: drawable_size.height as u16,
    });

    tracing::info!(
        "font size: {:.0}pt → {}x{} grid",
        state.current_font_size,
        cols,
        rows
    );
}

// ---------------------------------------------------------------------------
// RenderDelegate — CAMetalDisplayLink vsync callback
// ---------------------------------------------------------------------------

define_class!(
    /// Display link delegate that drives rendering at vsync.
    ///
    /// SAFETY: NSObject has no subclassing requirements.
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VoltRenderDelegate"]
    struct RenderDelegate;

    unsafe impl NSObjectProtocol for RenderDelegate {}

    unsafe impl CAMetalDisplayLinkDelegate for RenderDelegate {
        #[unsafe(method(metalDisplayLink:needsUpdate:))]
        fn metal_display_link_needs_update(
            &self,
            _link: &CAMetalDisplayLink,
            update: &CAMetalDisplayLinkUpdate,
        ) {
            render_frame(update);
        }
    }
);

impl RenderDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Drain PTY output, feed terminal, render to the display link's drawable.
fn render_frame(update: &CAMetalDisplayLinkUpdate) {
    APP.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(state) = borrow.as_mut() else { return };

        if state.terminating {
            return;
        }

        // 1. Drain PTY output → feed terminal (parse eagerly)
        let mut title_changed = None;
        loop {
            match state.pty_handle.rx().try_recv() {
                Ok(PtyRead::Data(bytes)) => {
                    let events = state.terminal.feed(&bytes);
                    for event in events {
                        match event {
                            volt_core::parser::TerminalEvent::TitleChanged(t) => {
                                title_changed = Some(t);
                            }
                            volt_core::parser::TerminalEvent::Bell => {
                                // TODO: visual bell / system beep
                            }
                            _ => {}
                        }
                    }
                }
                Ok(PtyRead::Closed) => {
                    if state.terminating {
                        return;
                    }
                    state.terminating = true;
                    tracing::info!("shell exited, terminating");
                    // Drop the borrow before calling terminate, which triggers
                    // AppKit callbacks that may need to borrow APP.
                    drop(borrow);
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    let app = NSApplication::sharedApplication(mtm);
                    app.terminate(None);
                    return;
                }
                Ok(PtyRead::Error(e)) => {
                    tracing::error!("PTY read error: {e}");
                    break;
                }
                Err(_) => break, // Channel empty
            }
        }

        // 2. Update window title if changed
        if let Some(title) = title_changed {
            state.window.setTitle(&NSString::from_str(&title));
        }

        // 3. Render to the display link's drawable
        let drawable = update.drawable();
        let texture = drawable.texture();

        let render_pass_desc = objc2_metal::MTLRenderPassDescriptor::new();
        let color_attachment = unsafe {
            render_pass_desc
                .colorAttachments()
                .objectAtIndexedSubscript(0)
        };
        color_attachment.setTexture(Some(&texture));
        color_attachment.setLoadAction(objc2_metal::MTLLoadAction::Clear);
        color_attachment.setStoreAction(objc2_metal::MTLStoreAction::Store);

        let bg = state.bg_color;
        color_attachment.setClearColor(objc2_metal::MTLClearColor {
            red: bg[0] as f64,
            green: bg[1] as f64,
            blue: bg[2] as f64,
            alpha: 1.0,
        });

        // Convert CAMetalDrawable → MTLDrawable for presentDrawable.
        // SAFETY: CAMetalDrawable conforms to MTLDrawable; same ObjC object.
        let mtl_drawable: &ProtocolObject<dyn objc2_metal::MTLDrawable> = unsafe {
            let ptr = &*drawable as *const ProtocolObject<dyn objc2_quartz_core::CAMetalDrawable>
                as *const ProtocolObject<dyn objc2_metal::MTLDrawable>;
            &*ptr
        };

        state
            .renderer
            .draw(&state.terminal, &render_pass_desc, Some(mtl_drawable));
    });
}

// ---------------------------------------------------------------------------
// AppDelegate — NSApplication lifecycle
// ---------------------------------------------------------------------------

define_class!(
    /// Application delegate handling lifecycle events.
    ///
    /// SAFETY: NSObject has no subclassing requirements.
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VoltAppDelegate"]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            tracing::info!("applicationDidFinishLaunching");
            // Window is already shown before app.run() — nothing to do here.
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _sender: &NSApplication) -> bool {
            true
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

fn setup_menu(app: &NSApplication, mtm: MainThreadMarker) {
    let menu_bar = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));

    // --- App menu ---
    let app_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Volt"),
            None,
            ns_string!(""),
        )
    };
    let app_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Volt"));
    unsafe {
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("About Volt"),
            Some(objc2::sel!(orderFrontStandardAboutPanel:)),
            ns_string!(""),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Hide Volt"),
            Some(objc2::sel!(hide:)),
            ns_string!("h"),
        );
        // Cmd+Opt+H: hide others
        let hide_others = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Hide Others"),
            Some(objc2::sel!(hideOtherApplications:)),
            ns_string!("h"),
        );
        hide_others.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Option,
        );
        app_menu.addItem(&hide_others);
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Show All"),
            Some(objc2::sel!(unhideAllApplications:)),
            ns_string!(""),
        );
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Quit Volt"),
            Some(objc2::sel!(terminate:)),
            ns_string!("q"),
        );
    }
    app_menu_item.setSubmenu(Some(&app_menu));
    menu_bar.addItem(&app_menu_item);

    // --- Shell menu ---
    // NOTE: New Window / New Tab use empty key equivalents — shortcuts are
    // handled in keyDown via handle_command_key so the menu system doesn't
    // swallow the events. Only system-backed actions get key equivalents here.
    let shell_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Shell"),
            None,
            ns_string!(""),
        )
    };
    let shell_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Shell"));
    unsafe {
        shell_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("New Window"),
            None,
            ns_string!(""),
        );
        shell_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("New Tab"),
            None,
            ns_string!(""),
        );
        shell_menu.addItem(&NSMenuItem::separatorItem(mtm));
        shell_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Close"),
            Some(objc2::sel!(performClose:)),
            ns_string!("w"),
        );
    }
    shell_menu_item.setSubmenu(Some(&shell_menu));
    menu_bar.addItem(&shell_menu_item);

    // --- Edit menu ---
    // Copy/Paste/SelectAll: handled in keyDown, NOT via responder chain,
    // because our custom NSView doesn't implement the standard copy:/paste:
    // selectors. No key equivalents here — keyDown dispatches them.
    let edit_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Edit"),
            None,
            ns_string!(""),
        )
    };
    let edit_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Edit"));
    unsafe {
        edit_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Copy                              \u{2318}C"),
            None,
            ns_string!(""),
        );
        edit_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Paste                             \u{2318}V"),
            None,
            ns_string!(""),
        );
        edit_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Select All                        \u{2318}A"),
            None,
            ns_string!(""),
        );
        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
        edit_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Clear Screen                      \u{2318}L"),
            None,
            ns_string!(""),
        );
        edit_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Clear Scrollback                  \u{2318}K"),
            None,
            ns_string!(""),
        );
        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
        edit_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Find...                           \u{2318}F"),
            None,
            ns_string!(""),
        );
    }
    edit_menu_item.setSubmenu(Some(&edit_menu));
    menu_bar.addItem(&edit_menu_item);

    // --- View menu ---
    let view_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("View"),
            None,
            ns_string!(""),
        )
    };
    let view_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("View"));
    unsafe {
        view_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Zoom In                           \u{2318}+"),
            None,
            ns_string!(""),
        );
        view_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Zoom Out                          \u{2318}-"),
            None,
            ns_string!(""),
        );
        view_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Actual Size                       \u{2318}0"),
            None,
            ns_string!(""),
        );
        view_menu.addItem(&NSMenuItem::separatorItem(mtm));
        view_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Toggle Full Screen"),
            Some(objc2::sel!(toggleFullScreen:)),
            ns_string!("\r"),
        );
    }
    view_menu_item.setSubmenu(Some(&view_menu));
    menu_bar.addItem(&view_menu_item);

    // --- Window menu ---
    let window_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Window"),
            None,
            ns_string!(""),
        )
    };
    let window_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Window"));
    unsafe {
        window_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Minimize"),
            Some(objc2::sel!(miniaturize:)),
            ns_string!("m"),
        );
        window_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Zoom"),
            Some(objc2::sel!(performZoom:)),
            ns_string!(""),
        );
    }
    window_menu_item.setSubmenu(Some(&window_menu));
    menu_bar.addItem(&window_menu_item);

    app.setMainMenu(Some(&menu_bar));
    app.setWindowsMenu(Some(&window_menu));
}

// ---------------------------------------------------------------------------
// Initialization and run
// ---------------------------------------------------------------------------

/// Initialize all state and run the application event loop.
///
/// This function does not return (NSApplication::run() takes over the thread).
pub fn run_app(config: VoltConfig) {
    let mtm = MainThreadMarker::new().expect("must be called from main thread");

    // 1. Create NSApplication
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    // 2. Create window + view
    let (win, view) = window::create_window(mtm, &config.window);

    // 3. Get the CAMetalLayer from the view and configure it
    let layer = view.layer().expect("view must have a layer");
    // SAFETY: We know the layer is a CAMetalLayer because makeBackingLayer returns one.
    let metal_layer: Retained<CAMetalLayer> = unsafe {
        let ptr = Retained::as_ptr(&layer) as *mut CAMetalLayer;
        Retained::retain(ptr).expect("metal layer must be non-null")
    };

    let device = MTLCreateSystemDefaultDevice().expect("no Metal GPU available");
    metal_layer.setDevice(Some(&device));
    metal_layer.setPixelFormat(objc2_metal::MTLPixelFormat::BGRA8Unorm);

    // Set Retina scale factor so rendering matches physical pixels
    let scale = win.screen().map(|s| s.backingScaleFactor()).unwrap_or(2.0);
    metal_layer.setContentsScale(scale);

    let drawable_w = config.window.width * scale;
    let drawable_h = config.window.height * scale;
    metal_layer.setDrawableSize(objc2_foundation::NSSize {
        width: drawable_w,
        height: drawable_h,
    });

    // 4. Create renderer with the same Metal device.
    // Scale font to physical pixels so glyphs are crisp on Retina (e.g., 14pt × 2.0 = 28px).
    let physical_font_size = config.font.size * scale as f32;
    let physical_line_height = physical_font_size * config.font.line_height;
    let renderer = Renderer::with_device(
        device,
        config.font.family.as_deref(),
        physical_font_size,
        physical_line_height,
    )
    .expect("failed to create renderer");

    // 5. Compute terminal size from drawable (physical pixels) and cell metrics
    let cell_m = renderer.cell_metrics();
    let cols = (drawable_w as f32 / cell_m.width).max(1.0) as u16;
    let rows = (drawable_h as f32 / cell_m.height).max(1.0) as u16;

    tracing::info!(
        "terminal size: {cols}x{rows} (cell: {:.1}x{:.1})",
        cell_m.width,
        cell_m.height
    );

    // 6. Create terminal
    let terminal = Terminal::new(TermSize { cols, rows });

    // 7. Spawn PTY
    let pty_handle = PtyHandle::spawn(PtyConfig {
        shell: config.behavior.shell.clone(),
        env: vec![
            ("TERM".into(), "xterm-256color".into()),
            ("COLORTERM".into(), "truecolor".into()),
        ],
        working_dir: config
            .behavior
            .working_directory
            .clone()
            .or_else(|| std::env::var("HOME").ok().map(std::path::PathBuf::from)),
        size: PtySize {
            rows,
            cols,
            pixel_width: config.window.width as u16,
            pixel_height: config.window.height as u16,
        },
    })
    .expect("failed to spawn PTY");

    tracing::info!("PTY spawned, child pid: {}", pty_handle.child_pid());

    // 8. Set up CAMetalDisplayLink for vsync-driven rendering
    let render_delegate = RenderDelegate::new(mtm);
    let display_link =
        CAMetalDisplayLink::initWithMetalLayer(CAMetalDisplayLink::alloc(), &metal_layer);
    display_link.setDelegate(Some(ProtocolObject::from_ref(&*render_delegate)));
    unsafe {
        display_link.addToRunLoop_forMode(&NSRunLoop::currentRunLoop(), NSDefaultRunLoopMode);
    }

    // 9. Store all state in thread-local
    let font_size = config.font.size;
    let line_height_multiplier = config.font.line_height;
    let font_family = config.font.family.clone();
    APP.with(|cell| {
        *cell.borrow_mut() = Some(Box::new(AppState {
            terminal,
            renderer,
            pty_handle,
            window: win,
            metal_layer,
            _display_link: display_link,
            _render_delegate: render_delegate,
            bg_color: config.colors.background,
            scale,
            current_font_size: font_size,
            default_font_size: font_size,
            line_height_multiplier,
            _font_family: font_family,
            terminating: false,
        }));
    });

    // 10. Set up menu and delegate
    setup_menu(&app, mtm);

    let delegate = AppDelegate::new(mtm);
    let proto: &ProtocolObject<dyn NSApplicationDelegate> = ProtocolObject::from_ref(&*delegate);
    app.setDelegate(Some(proto));

    // 11. Show window, enter fullscreen, activate, and run (does not return).
    // Clone the window Retained and DROP the borrow before calling toggleFullScreen,
    // because fullscreen triggers setFrameSize → handle_resize → borrow_mut,
    // which would panic if we still hold an immutable borrow.
    let window_ref = APP.with(|cell| cell.borrow().as_ref().map(|state| state.window.clone()));
    if let Some(win) = window_ref {
        win.makeKeyAndOrderFront(None);
        win.toggleFullScreen(None);
    }
    app.activate();
    app.run();

    // Unreachable, but keep delegate alive for the compiler
    drop(delegate);
}
