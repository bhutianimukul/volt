use crate::event::{ClickState, EventPayload, EventProxy, RioEvent, RioEventType};
use crate::ime::Preedit;
use crate::renderer::utils::update_colors_based_on_theme;
use crate::router::{routes::RoutePath, Router};
use crate::scheduler::{Scheduler, TimerId, Topic};
use crate::screen::touch::on_touch;
use crate::watcher::configuration_file_updates;
#[cfg(all(
    feature = "audio",
    not(target_os = "macos"),
    not(target_os = "windows")
))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use raw_window_handle::HasDisplayHandle;
use rio_backend::clipboard::{Clipboard, ClipboardType};
use rio_backend::config::colors::{ColorRgb, NamedColor};
use rio_window::application::ApplicationHandler;
use rio_window::event::{
    ElementState, Ime, MouseButton, MouseScrollDelta, StartCause, TouchPhase, WindowEvent,
};
use rio_window::event_loop::ActiveEventLoop;
use rio_window::event_loop::ControlFlow;
use rio_window::event_loop::{DeviceEvents, EventLoop};
#[cfg(target_os = "macos")]
use rio_window::platform::macos::ActiveEventLoopExtMacOS;
#[cfg(target_os = "macos")]
use rio_window::platform::macos::WindowExtMacOS;
use rio_window::window::WindowId;
use rio_window::window::{CursorIcon, Fullscreen};
use std::error::Error;
use std::time::{Duration, Instant};

pub struct Application<'a> {
    config: rio_backend::config::Config,
    event_proxy: EventProxy,
    router: Router<'a>,
    scheduler: Scheduler,
    app_id: Option<String>,
}

impl Application<'_> {
    pub fn new<'app>(
        config: rio_backend::config::Config,
        config_error: Option<rio_backend::config::ConfigError>,
        event_loop: &EventLoop<EventPayload>,
        app_id: Option<String>,
    ) -> Application<'app> {
        // SAFETY: Since this takes a pointer to the winit event loop, it MUST be dropped first,
        // which is done in `loop_exiting`.
        let clipboard =
            unsafe { Clipboard::new(event_loop.display_handle().unwrap().as_raw()) };

        let mut router = Router::new(config.fonts.to_owned(), clipboard);
        if let Some(error) = config_error {
            router.propagate_error_to_next_route(error.into());
        }

        let proxy = event_loop.create_proxy();
        let event_proxy = EventProxy::new(proxy.clone());
        let _ = configuration_file_updates(
            rio_backend::config::config_dir_path(),
            event_proxy.clone(),
        );
        let scheduler = Scheduler::new(proxy);
        event_loop.listen_device_events(DeviceEvents::Never);

        #[cfg(target_os = "macos")]
        event_loop.set_confirm_before_quit(config.confirm_before_quit);

        Application {
            config,
            event_proxy,
            router,
            scheduler,
            app_id,
        }
    }

    fn skip_window_event(event: &WindowEvent) -> bool {
        matches!(
            event,
            WindowEvent::KeyboardInput {
                is_synthetic: true,
                ..
            } | WindowEvent::ActivationTokenDone { .. }
                | WindowEvent::DoubleTapGesture { .. }
                | WindowEvent::TouchpadPressure { .. }
                | WindowEvent::RotationGesture { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::AxisMotion { .. }
                | WindowEvent::PanGesture { .. }
                | WindowEvent::HoveredFileCancelled
                | WindowEvent::Destroyed
                | WindowEvent::HoveredFile(_)
                | WindowEvent::Moved(_)
        )
    }

    fn handle_visual_bell(&mut self, window_id: WindowId) {
        if let Some(route) = self.router.routes.get_mut(&window_id) {
            route.window.screen.renderer.trigger_visual_bell();

            // Mark content as dirty to ensure render happens
            route
                .window
                .screen
                .ctx_mut()
                .current_mut()
                .renderable_content
                .pending_update
                .set_dirty();

            // Force immediate render to show the bell
            route.request_redraw();

            // Schedule a render after the bell duration to clear it
            let timer_id =
                TimerId::new(Topic::Render, route.window.screen.ctx().current_route());
            let event = EventPayload::new(RioEventType::Rio(RioEvent::Render), window_id);

            // Schedule render to clear bell effect after visual bell duration
            self.scheduler.schedule(
                event,
                crate::constants::BELL_DURATION,
                false,
                timer_id,
            );
        }
    }

    fn handle_audio_bell(&mut self) {
        #[cfg(target_os = "macos")]
        {
            // Use system bell sound on macOS
            unsafe {
                #[link(name = "AppKit", kind = "framework")]
                extern "C" {
                    fn NSBeep();
                }
                NSBeep();
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Use MessageBeep on Windows with MB_OK (0x00000000) for default beep
            unsafe {
                windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(0x00000000);
            }
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            #[cfg(feature = "audio")]
            {
                std::thread::spawn(|| {
                    if let Err(e) = play_bell_sound() {
                        tracing::warn!("Failed to play bell sound: {}", e);
                    }
                });
            }
            #[cfg(not(feature = "audio"))]
            {
                tracing::debug!("Audio bell requested but audio feature is not enabled");
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn set_dock_icon() {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};
        use std::ffi::CString;

        // Write icon to a temp file and load from path (most reliable method)
        let icon_data: &[u8] = include_bytes!("../../../misc/volt-icon.png");
        let icon_path = std::env::temp_dir().join("volt-dock-icon.png");
        if std::fs::write(&icon_path, icon_data).is_err() {
            tracing::warn!("Failed to write dock icon to temp file");
            return;
        }

        unsafe {
            let ns_string_class = match Class::get("NSString") {
                Some(c) => c,
                None => return,
            };
            let ns_image_class = match Class::get("NSImage") {
                Some(c) => c,
                None => return,
            };

            let path_str = CString::new(icon_path.to_string_lossy().as_bytes()).unwrap();
            let ns_path: *mut Object = msg_send![ns_string_class,
                stringWithUTF8String: path_str.as_ptr()];

            let ns_image: *mut Object = msg_send![ns_image_class, alloc];
            let ns_image: *mut Object = msg_send![ns_image, initWithContentsOfFile: ns_path];

            if ns_image.is_null() {
                tracing::warn!("Failed to create NSImage from icon file");
                return;
            }

            let ns_app_class = Class::get("NSApplication").unwrap();
            let ns_app: *mut Object = msg_send![ns_app_class, sharedApplication];
            let _: () = msg_send![ns_app, setApplicationIconImage: ns_image];
            tracing::info!("Dock icon set from {}", icon_path.display());
        }
    }

    pub fn run(
        &mut self,
        event_loop: EventLoop<EventPayload>,
    ) -> Result<(), Box<dyn Error>> {
        let result = event_loop.run_app(self);
        result.map_err(Into::into)
    }
}

impl ApplicationHandler<EventPayload> for Application<'_> {
    fn resumed(&mut self, _active_event_loop: &ActiveEventLoop) {}

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause != StartCause::Init
            && cause != StartCause::CreateWindow
            && cause != StartCause::MacOSReopen
        {
            return;
        }

        if cause == StartCause::MacOSReopen && !self.router.routes.is_empty() {
            return;
        }

        #[cfg(target_os = "macos")]
        {
            // Set the dock icon programmatically for cargo run (no .app bundle)
            static ICON_SET: std::sync::Once = std::sync::Once::new();
            ICON_SET.call_once(|| {
                Self::set_dock_icon();
            });
        }

        update_colors_based_on_theme(&mut self.config, event_loop.system_theme());

        self.router.create_window(
            event_loop,
            self.event_proxy.clone(),
            &self.config,
            None,
            self.app_id.as_deref(),
        );

        // Schedule title updates every 2s
        let timer_id = TimerId::new(Topic::UpdateTitles, 0);
        if !self.scheduler.scheduled(timer_id) {
            self.scheduler.schedule(
                EventPayload::new(RioEventType::Rio(RioEvent::UpdateTitles), unsafe {
                    rio_window::window::WindowId::dummy()
                }),
                Duration::from_secs(2),
                true,
                timer_id,
            );
        }

        tracing::info!("Initialisation complete");
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: EventPayload) {
        let window_id = event.window_id;
        match event.payload {
            RioEventType::Rio(RioEvent::Render) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    // Skip rendering for unfocused windows if configured
                    if self.config.renderer.disable_unfocused_render
                        && !route.window.is_focused
                    {
                        return;
                    }

                    // Skip rendering for occluded windows if configured, unless we need to render after occlusion
                    if self.config.renderer.disable_occluded_render
                        && route.window.is_occluded
                        && !route.window.needs_render_after_occlusion
                    {
                        return;
                    }

                    // Clear the one-time render flag if it was set
                    if route.window.needs_render_after_occlusion {
                        route.window.needs_render_after_occlusion = false;
                    }

                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::RenderRoute(route_id)) => {
                if self.config.renderer.strategy.is_event_based() {
                    if let Some(route) = self.router.routes.get_mut(&window_id) {
                        // Skip rendering for unfocused windows if configured
                        if self.config.renderer.disable_unfocused_render
                            && !route.window.is_focused
                        {
                            return;
                        }

                        // Skip rendering for occluded windows if configured, unless we need to render after occlusion
                        if self.config.renderer.disable_occluded_render
                            && route.window.is_occluded
                            && !route.window.needs_render_after_occlusion
                        {
                            return;
                        }

                        // Clear the one-time render flag if it was set
                        if route.window.needs_render_after_occlusion {
                            route.window.needs_render_after_occlusion = false;
                        }

                        // Mark the renderable content as needing to render
                        if let Some(ctx_item) =
                            route.window.screen.ctx_mut().get_mut(route_id)
                        {
                            ctx_item.val.renderable_content.pending_update.set_dirty();
                        }

                        // Check if we need to throttle based on timing
                        if let Some(wait_duration) = route.window.wait_until() {
                            // We need to wait before rendering again
                            let timer_id = TimerId::new(Topic::RenderRoute, route_id);
                            let event = EventPayload::new(
                                RioEventType::Rio(RioEvent::Render),
                                window_id,
                            );

                            // Only schedule if not already scheduled
                            if !self.scheduler.scheduled(timer_id) {
                                self.scheduler.schedule(
                                    event,
                                    wait_duration,
                                    false,
                                    timer_id,
                                );
                            }
                        } else {
                            // We can render immediately
                            route.request_redraw();
                        }
                    }
                }
            }

            RioEventType::Rio(RioEvent::Wakeup(route_id)) => {
                if self.config.renderer.strategy.is_event_based() {
                    if let Some(route) = self.router.routes.get_mut(&window_id) {
                        // Skip rendering for unfocused windows if configured
                        if self.config.renderer.disable_unfocused_render
                            && !route.window.is_focused
                        {
                            tracing::trace!("Wakeup: Skipping unfocused window");
                            return;
                        }

                        // Skip rendering for occluded windows if configured
                        if self.config.renderer.disable_occluded_render
                            && route.window.is_occluded
                            && !route.window.needs_render_after_occlusion
                        {
                            tracing::trace!("Wakeup: Skipping occluded window");
                            return;
                        }

                        tracing::trace!(
                            "Wakeup: Marking route {} for damage check",
                            route_id
                        );

                        // Mark the renderable content as needing to check for damage
                        // The actual damage retrieval will happen during render
                        if let Some(ctx_item) =
                            route.window.screen.ctx_mut().get_mut(route_id)
                        {
                            ctx_item.val.renderable_content.pending_update.set_dirty();
                            route.schedule_redraw(&mut self.scheduler, route_id);
                        }
                    }
                }
            }
            RioEventType::Rio(RioEvent::UpdateGraphics { route_id, queues }) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    // Process graphics directly in sugarloaf
                    let sugarloaf = &mut route.window.screen.sugarloaf;

                    for graphic_data in queues.pending {
                        sugarloaf.graphics.insert(graphic_data);
                    }

                    for graphic_data in queues.remove_queue {
                        sugarloaf.graphics.remove(&graphic_data);
                    }

                    // Request a redraw to display the updated graphics
                    route.schedule_redraw(&mut self.scheduler, route_id);
                }
            }
            RioEventType::Rio(RioEvent::PrepareUpdateConfig) => {
                let timer_id = TimerId::new(Topic::UpdateConfig, 0);
                let event = EventPayload::new(
                    RioEventType::Rio(RioEvent::UpdateConfig),
                    window_id,
                );

                if !self.scheduler.scheduled(timer_id) {
                    self.scheduler.schedule(
                        event,
                        Duration::from_millis(250),
                        false,
                        timer_id,
                    );
                }
            }
            RioEventType::Rio(RioEvent::ReportToAssistant(error)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.report_error(&error);
                }
            }
            RioEventType::Rio(RioEvent::UpdateConfig) => {
                let (config, config_error) = match rio_backend::config::Config::try_load()
                {
                    Ok(config) => (config, None),
                    Err(error) => (rio_backend::config::Config::default(), Some(error)),
                };

                let has_font_updates = self.config.fonts != config.fonts;

                let font_library_errors = if has_font_updates {
                    let new_font_library = rio_backend::sugarloaf::font::FontLibrary::new(
                        config.fonts.to_owned(),
                    );
                    self.router.font_library = Box::new(new_font_library.0);
                    new_font_library.1
                } else {
                    None
                };

                self.config = config;

                let mut has_checked_adaptive_colors = false;
                for (_id, route) in self.router.routes.iter_mut() {
                    // Apply system theme to ensure colors are consistent
                    if !has_checked_adaptive_colors {
                        let system_theme = route.window.winit_window.theme();
                        update_colors_based_on_theme(&mut self.config, system_theme);
                        has_checked_adaptive_colors = true;
                    }

                    if has_font_updates {
                        if let Some(ref err) = font_library_errors {
                            route
                                .window
                                .screen
                                .context_manager
                                .report_error_fonts_not_found(
                                    err.fonts_not_found.clone(),
                                );
                        }
                    }

                    route.update_config(
                        &self.config,
                        &self.router.font_library,
                        has_font_updates,
                    );
                    route.window.configure_window(&self.config);

                    if let Some(error) = &config_error {
                        route.report_error(&error.to_owned().into());
                    } else {
                        route.clear_errors();
                    }
                }
            }
            RioEventType::Rio(RioEvent::Exit) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if cfg!(target_os = "macos") && self.config.confirm_before_quit {
                        route.confirm_quit();
                        route.request_redraw();
                    } else {
                        route.quit();
                    }
                }
            }
            RioEventType::Rio(RioEvent::CloseTerminal(route_id)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route
                        .window
                        .screen
                        .context_manager
                        .should_close_context_manager(route_id)
                    {
                        self.router.routes.remove(&window_id);

                        // Unschedule pending events.
                        self.scheduler.unschedule_window(route_id);

                        if self.router.routes.is_empty() {
                            event_loop.exit();
                        }
                    } else {
                        let size = route.window.screen.context_manager.len();
                        route.window.screen.resize_top_or_bottom_line(size);
                    }
                }
            }
            RioEventType::Rio(RioEvent::CursorBlinkingChange) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::CursorBlinkingChangeOnRoute(route_id)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route_id == route.window.screen.ctx().current_route() {
                        // Get cursor position for damage
                        let cursor_line = {
                            let terminal = route
                                .window
                                .screen
                                .ctx_mut()
                                .current_mut()
                                .terminal
                                .lock();
                            terminal.cursor().pos.row.0 as usize
                        };

                        // Set UI damage for cursor line
                        route
                            .window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .renderable_content
                            .pending_update
                            .set_ui_damage(rio_backend::event::TerminalDamage::Partial(
                                [rio_backend::crosswords::LineDamage::new(
                                    cursor_line,
                                    true,
                                )]
                                .into_iter()
                                .collect(),
                            ));

                        route.request_redraw();
                    }
                }
            }
            RioEventType::Rio(RioEvent::Bell) => {
                // Handle visual bell
                if self.config.bell.visual {
                    self.handle_visual_bell(window_id);
                }

                // Handle audio bell
                if self.config.bell.audio {
                    self.handle_audio_bell();
                }

                // Handle dock badge for unfocused windows
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    let is_focused = route.window.is_focused;
                    route
                        .window
                        .screen
                        .ctx_mut()
                        .dock_badge
                        .on_bell(is_focused);
                }
            }
            RioEventType::Rio(RioEvent::PrepareRender(millis)) => {
                if let Some(route) = self.router.routes.get(&window_id) {
                    let timer_id = TimerId::new(
                        Topic::Render,
                        route.window.screen.ctx().current_route(),
                    );
                    let event =
                        EventPayload::new(RioEventType::Rio(RioEvent::Render), window_id);

                    if !self.scheduler.scheduled(timer_id) {
                        self.scheduler.schedule(
                            event,
                            Duration::from_millis(millis),
                            false,
                            timer_id,
                        );
                    }
                }
            }
            RioEventType::Rio(RioEvent::PrepareRenderOnRoute(millis, route_id)) => {
                let timer_id = TimerId::new(Topic::RenderRoute, route_id);
                let event = EventPayload::new(
                    RioEventType::Rio(RioEvent::RenderRoute(route_id)),
                    window_id,
                );

                if !self.scheduler.scheduled(timer_id) {
                    self.scheduler.schedule(
                        event,
                        Duration::from_millis(millis),
                        false,
                        timer_id,
                    );
                }
            }
            RioEventType::Rio(RioEvent::BlinkCursor(millis, route_id)) => {
                let timer_id = TimerId::new(Topic::CursorBlinking, route_id);
                let event = EventPayload::new(
                    RioEventType::Rio(RioEvent::CursorBlinkingChangeOnRoute(route_id)),
                    window_id,
                );

                if !self.scheduler.scheduled(timer_id) {
                    self.scheduler.schedule(
                        event,
                        Duration::from_millis(millis),
                        false,
                        timer_id,
                    );
                }
            }
            RioEventType::Rio(RioEvent::Title(title)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.set_window_title(&title);
                }
            }
            RioEventType::Rio(RioEvent::TitleWithSubtitle(title, subtitle)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.set_window_title(&title);
                    route.set_window_subtitle(&subtitle);
                }
            }
            RioEventType::Rio(RioEvent::UpdateTitles) => {
                self.router.update_titles();
            }
            RioEventType::Rio(RioEvent::MouseCursorDirty) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.window.screen.reset_mouse();
                }
            }
            RioEventType::Rio(RioEvent::Scroll(scroll)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    let mut terminal = route
                        .window
                        .screen
                        .context_manager
                        .current_mut()
                        .terminal
                        .lock();
                    terminal.scroll_display(scroll);
                    drop(terminal);
                }
            }
            RioEventType::Rio(RioEvent::ClipboardLoad(clipboard_type, format)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.window.is_focused {
                        let text = format(
                            self.router
                                .clipboard
                                .borrow_mut()
                                .get(clipboard_type)
                                .as_str(),
                        );
                        route
                            .window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_bytes(text.into_bytes());
                    }
                }
            }
            RioEventType::Rio(RioEvent::ClipboardStore(clipboard_type, content)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.window.is_focused {
                        self.router
                            .clipboard
                            .borrow_mut()
                            .set(clipboard_type, content);
                    }
                }
            }
            RioEventType::Rio(RioEvent::PtyWrite(text)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route
                        .window
                        .screen
                        .ctx_mut()
                        .current_mut()
                        .messenger
                        .send_bytes(text.into_bytes());
                }
            }
            RioEventType::Rio(RioEvent::TextAreaSizeRequest(format)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    let dimension =
                        route.window.screen.context_manager.current().dimension;
                    let text =
                        format(crate::renderer::utils::terminal_dimensions(&dimension));
                    route
                        .window
                        .screen
                        .ctx_mut()
                        .current_mut()
                        .messenger
                        .send_bytes(text.into_bytes());
                }
            }
            RioEventType::Rio(RioEvent::ColorRequest(index, format)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    let terminal = route
                        .window
                        .screen
                        .context_manager
                        .current()
                        .terminal
                        .lock();
                    let color: ColorRgb = match terminal.colors()[index] {
                        Some(color) => ColorRgb::from_color_arr(color),
                        // Ignore cursor color requests unless it was changed.
                        None if index
                            == crate::crosswords::NamedColor::Cursor as usize =>
                        {
                            return
                        }
                        None => ColorRgb::from_color_arr(
                            route.window.screen.renderer.colors[index],
                        ),
                    };

                    drop(terminal);

                    route
                        .window
                        .screen
                        .ctx_mut()
                        .current_mut()
                        .messenger
                        .send_bytes(format(color).into_bytes());
                }
            }
            RioEventType::Rio(RioEvent::CreateWindow) => {
                self.router.create_window(
                    event_loop,
                    self.event_proxy.clone(),
                    &self.config,
                    None,
                    self.app_id.as_deref(),
                );
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::CreateNativeTab(working_dir_overwrite)) => {
                if let Some(route) = self.router.routes.get(&window_id) {
                    // This case happens only for native tabs
                    // every time that a new tab is created through context
                    // it also reaches for the foreground process path if
                    // config.use_current_path is true
                    // For these case we need to make a workaround
                    let config = if working_dir_overwrite.is_some() {
                        rio_backend::config::Config {
                            working_dir: working_dir_overwrite,
                            ..self.config.clone()
                        }
                    } else {
                        self.config.clone()
                    };

                    self.router.create_native_tab(
                        event_loop,
                        self.event_proxy.clone(),
                        &config,
                        Some(&route.window.winit_window.tabbing_identifier()),
                        None,
                    );
                }
            }
            RioEventType::Rio(RioEvent::CreateConfigEditor) => {
                if self.config.navigation.open_config_with_split {
                    self.router.open_config_split(&self.config);
                } else {
                    self.router.open_config_window(
                        event_loop,
                        self.event_proxy.clone(),
                        &self.config,
                    );
                }
            }
            RioEventType::Rio(RioEvent::ToggleSettings) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::Settings {
                        route.path = RoutePath::Terminal;
                    } else {
                        route.settings_editor.reload_from_config(&self.config);
                        route.path = RoutePath::Settings;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleHelp) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::Help {
                        route.path = RoutePath::Terminal;
                    } else {
                        route.path = RoutePath::Help;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleHistory) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::History {
                        route.path = RoutePath::Terminal;
                    } else {
                        route.path = RoutePath::History;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleTmuxPicker) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::TmuxPicker {
                        route.path = RoutePath::Terminal;
                    } else {
                        use crate::tmux_cc::TmuxController;
                        route.tmux_sessions = TmuxController::list_sessions();
                        route.tmux_selected = 0;
                        route.path = RoutePath::TmuxPicker;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleEnvViewer) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::EnvViewer {
                        route.path = RoutePath::Terminal;
                    } else {
                        route.path = RoutePath::EnvViewer;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleBookmarks) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::Bookmarks {
                        route.path = RoutePath::Terminal;
                    } else {
                        // Load bookmarks once when opening the viewer
                        let store = crate::bookmarks::BookmarkStore::load();
                        route.bookmarks_cache = store.list().into_iter().cloned().collect();
                        route.bookmarks_scroll = 0;
                        route.path = RoutePath::Bookmarks;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleSlashCommands) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::SlashCommands {
                        route.path = RoutePath::Terminal;
                    } else {
                        route.slash_commands_scroll = 0;
                        route.path = RoutePath::SlashCommands;
                    }
                    route.request_redraw();
                }
            }
            RioEventType::Rio(RioEvent::ToggleLayouts) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    if route.path == RoutePath::Layouts {
                        route.path = RoutePath::Terminal;
                    } else {
                        route.layouts_selected = 0;
                        route.path = RoutePath::Layouts;
                    }
                    route.request_redraw();
                }
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::CloseWindow) => {
                self.router.routes.remove(&window_id);
                if self.router.routes.is_empty() && !self.config.confirm_before_quit {
                    event_loop.exit();
                }
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::SelectNativeTabByIndex(tab_index)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.window.winit_window.select_tab_at_index(tab_index);
                }
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::SelectNativeTabLast) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route
                        .window
                        .winit_window
                        .select_tab_at_index(route.window.winit_window.num_tabs() - 1);
                }
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::SelectNativeTabNext) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.window.winit_window.select_next_tab();
                }
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::SelectNativeTabPrev) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.window.winit_window.select_previous_tab();
                }
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::Hide) => {
                event_loop.hide_application();
            }
            #[cfg(target_os = "macos")]
            RioEventType::Rio(RioEvent::HideOtherApplications) => {
                event_loop.hide_other_applications();
            }
            RioEventType::Rio(RioEvent::Minimize(set_minimize)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.window.winit_window.set_minimized(set_minimize);
                }
            }
            RioEventType::Rio(RioEvent::ToggleFullScreen) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    match route.window.winit_window.fullscreen() {
                        None => route
                            .window
                            .winit_window
                            .set_fullscreen(Some(Fullscreen::Borderless(None))),
                        _ => route.window.winit_window.set_fullscreen(None),
                    }
                }
            }
            RioEventType::Rio(RioEvent::ColorChange(route_id, index, color)) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    let screen = &mut route.window.screen;
                    // Background color is index 1 relative to NamedColor::Foreground
                    if index == NamedColor::Foreground as usize + 1 {
                        let grid = screen.context_manager.current_grid_mut();
                        if let Some(context_item) = grid.get_mut(route_id) {
                            use crate::context::renderable::BackgroundState;
                            context_item.context_mut().renderable_content.background =
                                Some(match color {
                                    Some(c) => BackgroundState::Set(c.to_wgpu()),
                                    None => BackgroundState::Reset,
                                });
                        }
                    }
                }
            }
            RioEventType::Rio(RioEvent::ShellPromptStart { row }) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route
                        .window
                        .screen
                        .context_manager
                        .shell_profiler
                        .on_first_prompt();
                    route
                        .window
                        .screen
                        .context_manager
                        .block_manager
                        .on_prompt_start(row);
                }
            }
            RioEventType::Rio(RioEvent::ShellCommandStart { row }) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route
                        .window
                        .screen
                        .context_manager
                        .block_manager
                        .on_command_start(row);
                }
            }
            RioEventType::Rio(RioEvent::ShellOutputStart { row }) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route
                        .window
                        .screen
                        .context_manager
                        .block_manager
                        .on_output_start(row, String::new());
                    route
                        .window
                        .screen
                        .context_manager
                        .notification_manager
                        .command_started(String::new());
                    route
                        .window
                        .screen
                        .context_manager
                        .audit_logger
                        .log_command("", "");
                    route
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .record(String::new(), std::path::PathBuf::from("."));
                }
            }
            RioEventType::Rio(RioEvent::ShellCommandFinish { row, exit_code }) => {
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route
                        .window
                        .screen
                        .context_manager
                        .block_manager
                        .on_command_finish(row, exit_code);
                    let is_focused = route.window.is_focused;
                    if !is_focused {
                        route
                            .window
                            .screen
                            .context_manager
                            .notification_manager
                            .command_finished(exit_code);
                    }
                    route
                        .window
                        .screen
                        .context_manager
                        .audit_logger
                        .log(
                            crate::audit_log::AuditEvent::CommandCompleted {
                                command: String::new(),
                                exit_code,
                                duration_ms: 0,
                            }
                        );
                    if let Some(last_id) = route.window.screen.context_manager.session_recorder.all().back().map(|e| e.id) {
                        route.window.screen.context_manager.session_recorder.complete(last_id, exit_code, 0, String::new());
                    }
                }
            }
            _ => {}
        }
    }

    #[cfg(target_os = "macos")]
    fn open_urls(&mut self, active_event_loop: &ActiveEventLoop, urls: Vec<String>) {
        if !self.config.navigation.is_native() {
            let config = &self.config;
            for url in urls {
                self.router.create_window(
                    active_event_loop,
                    self.event_proxy.clone(),
                    config,
                    Some(url),
                    self.app_id.as_deref(),
                );
            }
            return;
        }

        let mut tab_id = None;

        // In case only have one window
        for (_, route) in self.router.routes.iter() {
            if tab_id.is_none() {
                tab_id = Some(route.window.winit_window.tabbing_identifier());
            }

            if route.window.is_focused {
                tab_id = Some(route.window.winit_window.tabbing_identifier());
                break;
            }
        }

        if tab_id.is_some() {
            let config = &self.config;
            for url in urls {
                self.router.create_native_tab(
                    active_event_loop,
                    self.event_proxy.clone(),
                    config,
                    tab_id.as_deref(),
                    Some(url),
                );
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Ignore all events we do not care about.
        if Self::skip_window_event(&event) {
            return;
        }

        let route = match self.router.routes.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                // MacOS doesn't exit the loop
                if cfg!(target_os = "macos") && self.config.confirm_before_quit {
                    self.router.routes.remove(&window_id);
                    return;
                }

                if self.config.confirm_before_quit {
                    route.confirm_quit();
                    route.request_redraw();
                    return;
                } else {
                    self.router.routes.remove(&window_id);
                }

                if self.router.routes.is_empty() {
                    event_loop.exit();
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                route.window.screen.set_modifiers(modifiers);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if route.path != RoutePath::Terminal {
                    return;
                }

                if self.config.hide_cursor_when_typing {
                    route.window.winit_window.set_cursor_visible(true);
                }

                match button {
                    MouseButton::Left => {
                        route.window.screen.mouse.left_button_state = state
                    }
                    MouseButton::Middle => {
                        route.window.screen.mouse.middle_button_state = state
                    }
                    MouseButton::Right => {
                        route.window.screen.mouse.right_button_state = state
                    }
                    _ => (),
                }

                // Check nav button clicks and tab clicks BEFORE macOS deadzone
                if state == ElementState::Pressed && button == MouseButton::Left {
                    let mx = route.window.screen.mouse.x as f64;
                    let my = route.window.screen.mouse.y as f64;
                    let scale = route.window.screen.sugarloaf.scale_factor() as f64;
                    let lx = mx / scale;
                    let ly = my / scale;

                    let visible_w = route.window.screen.sugarloaf.window_size().width
                        / route.window.screen.sugarloaf.scale_factor();
                    let win_h = route.window.screen.sugarloaf.window_size().height
                        / route.window.screen.sugarloaf.scale_factor();

                    // Check nav buttons [Help] [Settings] — in tab bar region
                    // For TopTab: tab bar is at y=[0, 22]
                    // For BottomTab: tab bar is at y=[h-22-20, h-20]
                    let nav_mode = route.window.screen.renderer.navigation.navigation.mode;
                    let hide_single = route.window.screen.renderer.navigation.navigation.hide_if_single;
                    let num_tabs = route.window.screen.context_manager.len();
                    let tabs_hidden = hide_single && num_tabs <= 1;
                    let in_tab_bar = if tabs_hidden {
                        false
                    } else if nav_mode == rio_backend::config::navigation::NavigationMode::TopTab {
                        ly < 22.0
                    } else if nav_mode == rio_backend::config::navigation::NavigationMode::BottomTab {
                        let tab_bar_top = (win_h - 22.0 - 20.0) as f64;
                        let tab_bar_bottom = (win_h - 20.0) as f64;
                        ly >= tab_bar_top && ly <= tab_bar_bottom
                    } else {
                        false
                    };
                    if in_tab_bar {
                        if let Some(btn) = crate::renderer::navigation::nav_button_at_position(
                            lx as f32, visible_w,
                        ) {
                            use crate::renderer::navigation::NavButton;
                            match btn {
                                NavButton::Help => {
                                    route.window.screen.context_manager.toggle_help();
                                }
                                NavButton::Settings => {
                                    route.window.screen.context_manager.toggle_settings();
                                }
                                _ => {}
                            }
                            route.request_redraw();
                            return;
                        }
                    }

                    // Check bottom status bar buttons
                    // Only when the tab bar is visible (status bar renders with it)
                    {
                        let num_tabs = route.window.screen.context_manager.len();
                        let hide_single = route.window.screen.renderer.navigation.navigation.hide_if_single;
                        let is_tab_mode = nav_mode == rio_backend::config::navigation::NavigationMode::TopTab
                            || nav_mode == rio_backend::config::navigation::NavigationMode::BottomTab;
                        let tab_bar_visible = is_tab_mode && !(hide_single && num_tabs <= 1);

                        // Status bar clicks — ALWAYS check, even with hidden tab bar
                        if is_tab_mode {
                            if let Some(btn) = crate::renderer::navigation::status_button_at_position(
                                lx as f32, ly as f32, win_h, visible_w,
                            ) {
                                use crate::renderer::navigation::NavButton;
                                match btn {
                                    NavButton::TmuxConnect => {
                                        route.window.screen.context_manager.toggle_tmux_picker();
                                    }
                                    NavButton::AiAssistant => {
                                        if crate::ai_assistant::is_claude_available() {
                                            route.window.screen.split_right();
                                            let bytes = b"claude\r".to_vec();
                                            route.window.screen.ctx_mut().current_mut().messenger.send_write(bytes);
                                        }
                                    }
                                    NavButton::History => {
                                        route.window.screen.context_manager.toggle_history();
                                    }
                                    NavButton::EnvViewer => {
                                        route.window.screen.context_manager.toggle_env_viewer();
                                    }
                                    NavButton::Bookmarks => {
                                        route.window.screen.context_manager.toggle_bookmarks();
                                    }
                                    NavButton::Connections => {
                                        if route.path == RoutePath::Connections {
                                            route.path = RoutePath::Terminal;
                                        } else {
                                            // Load connections
                                            match crate::connections::load_connections() {
                                                Ok(config) => {
                                                    let mut list: Vec<(String, String, String, String)> = config
                                                        .connections
                                                        .iter()
                                                        .map(|(name, conn)| {
                                                            let host_info = conn.to_command();
                                                            (
                                                                name.clone(),
                                                                conn.type_name().to_string(),
                                                                host_info.clone(),
                                                                host_info,
                                                            )
                                                        })
                                                        .collect();
                                                    list.sort_by(|a, b| a.0.cmp(&b.0));
                                                    route.connections_list = list;
                                                    route.connections_selected = 0;
                                                    route.path = RoutePath::Connections;
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to load connections: {}", e);
                                                    route.connections_list = Vec::new();
                                                    route.connections_selected = 0;
                                                    route.path = RoutePath::Connections;
                                                }
                                            }
                                        }
                                    }
                                    NavButton::SlashCommands => {
                                        route.window.screen.context_manager.toggle_slash_commands();
                                    }
                                    NavButton::Layouts => {
                                        route.window.screen.context_manager.toggle_layouts();
                                    }
                                    _ => {}
                                }
                                route.request_redraw();
                                return;
                            }
                        }
                    }

                    // Check tab clicks
                    if let Some(tab_idx) =
                        route.window.screen.tab_index_at_position(mx, my)
                    {
                        let current = route.window.screen.context_manager.current_index();
                        let now = std::time::Instant::now();
                        let elapsed =
                            now - route.window.screen.mouse.last_click_timestamp;
                        let same_tab = route.window.screen.mouse.last_click_tab
                            == Some(tab_idx);
                        let is_double_click =
                            same_tab && elapsed < Duration::from_millis(400);

                        if tab_idx == current && is_double_click {
                            route.window.screen.prompt_rename_tab();
                        } else if tab_idx != current {
                            route.window.screen.context_manager.select_tab(tab_idx);
                            route.window.screen.render();
                        }
                        route.window.screen.mouse.last_click_timestamp = now;
                        route.window.screen.mouse.last_click_tab = Some(tab_idx);
                        route.request_redraw();
                        return;
                    }
                }

                #[cfg(target_os = "macos")]
                {
                    if route.window.is_macos_deadzone {
                        return;
                    }
                }

                match state {
                    ElementState::Pressed => {
                        // Check if click is on a split pane divider
                        if button == MouseButton::Left {
                            let mx = route.window.screen.mouse.x as f32;
                            let my = route.window.screen.mouse.y as f32;
                            if let Some(hit) = route
                                .window
                                .screen
                                .context_manager
                                .divider_at_position(mx, my)
                            {
                                use crate::mouse::DividerDragState;
                                route.window.screen.mouse.divider_drag =
                                    Some(DividerDragState {
                                        hit,
                                        last_x: mx,
                                        last_y: my,
                                    });
                                use crate::context::grid::DividerOrientation;
                                let cursor = match hit.orientation {
                                    DividerOrientation::Vertical => CursorIcon::ColResize,
                                    DividerOrientation::Horizontal => {
                                        CursorIcon::RowResize
                                    }
                                };
                                route.window.winit_window.set_cursor(cursor);
                                return;
                            }
                        }

                        // In case need to switch grid current
                        route.window.screen.select_current_based_on_mouse();

                        if route.window.screen.trigger_hyperlink() {
                            return;
                        }

                        // Process mouse press before bindings to update the `click_state`.
                        if !route.window.screen.modifiers.state().shift_key()
                            && route.window.screen.mouse_mode()
                        {
                            route.window.screen.mouse.click_state = ClickState::None;

                            let code = match button {
                                MouseButton::Left => 0,
                                MouseButton::Middle => 1,
                                MouseButton::Right => 2,
                                // Can't properly report more than three buttons..
                                MouseButton::Back
                                | MouseButton::Forward
                                | MouseButton::Other(_) => return,
                            };

                            route
                                .window
                                .screen
                                .mouse_report(code, ElementState::Pressed);

                            route.window.screen.process_mouse_bindings(button);
                        } else {
                            // Calculate time since the last click to handle double/triple clicks.
                            let now = Instant::now();
                            let elapsed =
                                now - route.window.screen.mouse.last_click_timestamp;
                            route.window.screen.mouse.last_click_timestamp = now;

                            let threshold = Duration::from_millis(300);
                            let mouse = &route.window.screen.mouse;
                            route.window.screen.mouse.click_state = match mouse
                                .click_state
                            {
                                // Reset click state if button has changed.
                                _ if button != mouse.last_click_button => {
                                    route.window.screen.mouse.last_click_button = button;
                                    ClickState::Click
                                }
                                ClickState::Click if elapsed < threshold => {
                                    ClickState::DoubleClick
                                }
                                ClickState::DoubleClick if elapsed < threshold => {
                                    ClickState::TripleClick
                                }
                                _ => ClickState::Click,
                            };

                            // Load mouse point, treating message bar and padding as the closest square.
                            let display_offset = route.window.screen.display_offset();

                            if let MouseButton::Left = button {
                                let pos =
                                    route.window.screen.mouse_position(display_offset);
                                route.window.screen.on_left_click(pos);
                            }

                            route.request_redraw();
                        }
                        route.window.screen.process_mouse_bindings(button);
                    }
                    ElementState::Released => {
                        // End divider drag if active
                        if button == MouseButton::Left
                            && route.window.screen.mouse.divider_drag.is_some()
                        {
                            route.window.screen.mouse.divider_drag = None;
                            route.window.winit_window.set_cursor(CursorIcon::Text);
                            return;
                        }

                        if !route.window.screen.modifiers.state().shift_key()
                            && route.window.screen.mouse_mode()
                        {
                            let code = match button {
                                MouseButton::Left => 0,
                                MouseButton::Middle => 1,
                                MouseButton::Right => 2,
                                // Can't properly report more than three buttons.
                                MouseButton::Back
                                | MouseButton::Forward
                                | MouseButton::Other(_) => return,
                            };
                            route
                                .window
                                .screen
                                .mouse_report(code, ElementState::Released);
                            return;
                        }

                        // Trigger hints highlighted by the mouse
                        if button == MouseButton::Left
                            && route.window.screen.trigger_hint()
                        {
                            return;
                        }

                        if let MouseButton::Left | MouseButton::Right = button {
                            // Copy selection on release, to prevent flooding the display server.
                            route.window.screen.copy_selection(ClipboardType::Selection);
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if self.config.hide_cursor_when_typing {
                    route.window.winit_window.set_cursor_visible(true);
                }

                if route.path != RoutePath::Terminal {
                    route.window.winit_window.set_cursor(CursorIcon::Default);
                    return;
                }

                let x = position.x;
                let y = position.y;

                // Handle active divider drag
                if let Some(drag_state) = route.window.screen.mouse.divider_drag {
                    let cur_x = x as f32;
                    let cur_y = y as f32;
                    let delta_x = cur_x - drag_state.last_x;
                    let delta_y = cur_y - drag_state.last_y;

                    let hit = drag_state.hit;
                    if route
                        .window
                        .screen
                        .context_manager
                        .drag_divider(&hit, delta_x, delta_y)
                    {
                        route.window.screen.mouse.divider_drag =
                            Some(crate::mouse::DividerDragState {
                                hit,
                                last_x: cur_x,
                                last_y: cur_y,
                            });
                        route.window.screen.render();
                        route.request_redraw();
                    } else {
                        // Update position even if drag didn't succeed (e.g. at min size)
                        route.window.screen.mouse.divider_drag =
                            Some(crate::mouse::DividerDragState {
                                hit,
                                last_x: cur_x,
                                last_y: cur_y,
                            });
                    }
                    return;
                }

                let lmb_pressed =
                    route.window.screen.mouse.left_button_state == ElementState::Pressed;
                let rmb_pressed =
                    route.window.screen.mouse.right_button_state == ElementState::Pressed;

                let has_selection = !route.window.screen.selection_is_empty();

                // Always update mouse position first (needed for tab click detection)
                let layout = route.window.screen.sugarloaf.window_size();
                let mx = x.clamp(0.0, (layout.width as i32 - 1).into()) as usize;
                let my = y.clamp(0.0, (layout.height as i32 - 1).into()) as usize;
                route.window.screen.mouse.x = mx;
                route.window.screen.mouse.y = my;

                #[cfg(target_os = "macos")]
                {
                    use rio_backend::config::navigation::NavigationMode;
                    let nav_mode =
                        route.window.screen.renderer.navigation.navigation.mode;
                    let is_tab_bar_mode = nav_mode == NavigationMode::TopTab
                        || nav_mode == NavigationMode::BottomTab;

                    // Dead zone for MacOS only — but NOT when using TopTab/BottomTab
                    // (tab bar occupies the deadzone area in those modes)
                    if !is_tab_bar_mode
                        && !has_selection
                        && !route.window.screen.context_manager.config.is_native
                        && route.window.screen.is_macos_deadzone(y)
                    {
                        route.window.winit_window.set_cursor(CursorIcon::Default);

                        route.window.is_macos_deadzone = true;
                        return;
                    }

                    route.window.is_macos_deadzone = false;
                }

                if has_selection && (lmb_pressed || rmb_pressed) {
                    route.window.screen.update_selection_scrolling(y);
                }

                let display_offset = route.window.screen.display_offset();
                let old_point = route.window.screen.mouse_position(display_offset);

                // Rebind x, y as usize for downstream code
                let x = mx;
                let y = my;

                let point = route.window.screen.mouse_position(display_offset);

                let square_changed = old_point != point;

                let inside_text_area = route.window.screen.contains_point(x, y);
                let square_side = route.window.screen.side_by_pos(x);

                // If the mouse hasn't changed cells, do nothing.
                if !square_changed
                    && route.window.screen.mouse.square_side == square_side
                    && route.window.screen.mouse.inside_text_area == inside_text_area
                {
                    // Even if the cell hasn't changed, update divider hover cursor
                    if !lmb_pressed {
                        if let Some(hit) = route
                            .window
                            .screen
                            .context_manager
                            .divider_at_position(x as f32, y as f32)
                        {
                            use crate::context::grid::DividerOrientation;
                            let cursor = match hit.orientation {
                                DividerOrientation::Vertical => CursorIcon::ColResize,
                                DividerOrientation::Horizontal => CursorIcon::RowResize,
                            };
                            route.window.winit_window.set_cursor(cursor);
                        }
                    }
                    return;
                }

                // Check if hovering over a divider (only when not dragging)
                if !lmb_pressed && !rmb_pressed {
                    if let Some(hit) = route
                        .window
                        .screen
                        .context_manager
                        .divider_at_position(x as f32, y as f32)
                    {
                        use crate::context::grid::DividerOrientation;
                        let cursor = match hit.orientation {
                            DividerOrientation::Vertical => CursorIcon::ColResize,
                            DividerOrientation::Horizontal => CursorIcon::RowResize,
                        };
                        route.window.winit_window.set_cursor(cursor);
                        route.window.screen.mouse.inside_text_area = inside_text_area;
                        route.window.screen.mouse.square_side = square_side;
                        return;
                    }
                }

                if route.window.screen.update_highlighted_hints() {
                    route.window.winit_window.set_cursor(CursorIcon::Pointer);
                    route.window.screen.context_manager.request_render();
                } else {
                    let cursor_icon =
                        if !route.window.screen.modifiers.state().shift_key()
                            && route.window.screen.mouse_mode()
                        {
                            CursorIcon::Default
                        } else {
                            CursorIcon::Text
                        };

                    route.window.winit_window.set_cursor(cursor_icon);

                    // In case hyperlink range has cleaned trigger one more render
                    if route
                        .window
                        .screen
                        .context_manager
                        .current()
                        .has_hyperlink_range()
                    {
                        route
                            .window
                            .screen
                            .context_manager
                            .current_mut()
                            .set_hyperlink_range(None);
                        route.window.screen.context_manager.request_render();
                    }
                }

                route.window.screen.mouse.inside_text_area = inside_text_area;
                route.window.screen.mouse.square_side = square_side;

                if (lmb_pressed || rmb_pressed)
                    && (route.window.screen.modifiers.state().shift_key()
                        || !route.window.screen.mouse_mode())
                {
                    route.window.screen.update_selection(point, square_side);
                    route.window.screen.context_manager.request_render();
                } else if square_changed
                    && route.window.screen.has_mouse_motion_and_drag()
                {
                    if lmb_pressed {
                        route.window.screen.mouse_report(32, ElementState::Pressed);
                    } else if route.window.screen.mouse.middle_button_state
                        == ElementState::Pressed
                    {
                        route.window.screen.mouse_report(33, ElementState::Pressed);
                    } else if route.window.screen.mouse.right_button_state
                        == ElementState::Pressed
                    {
                        route.window.screen.mouse_report(34, ElementState::Pressed);
                    } else if route.window.screen.has_mouse_motion() {
                        route.window.screen.mouse_report(35, ElementState::Pressed);
                    }
                }
            }

            WindowEvent::MouseWheel { delta, phase, .. } => {
                if route.path != RoutePath::Terminal {
                    return;
                }

                if self.config.hide_cursor_when_typing {
                    route.window.winit_window.set_cursor_visible(true);
                }

                // If mouse is over the tab bar, scroll tabs horizontally
                {
                    let mx = route.window.screen.mouse.x as f64;
                    let my = route.window.screen.mouse.y as f64;
                    if route.window.screen.tab_index_at_position(mx, my).is_some() || {
                        // Also check if mouse is in the tab bar Y range (even between tabs)
                        let scale = route.window.screen.sugarloaf.scale_factor() as f64;
                        let ly = my / scale;
                        ly < 22.0 // PADDING_Y_BOTTOM_TABS
                    } {
                        let scroll_delta = match delta {
                            MouseScrollDelta::LineDelta(cols, _) => cols * 30.0,
                            MouseScrollDelta::PixelDelta(pos) => pos.x as f32,
                        };
                        let num_tabs = route.window.screen.context_manager.len();
                        let visible_width = {
                            let ws = route.window.screen.sugarloaf.window_size();
                            let s = route.window.screen.sugarloaf.scale_factor();
                            ws.width / s
                        };
                        route.window.screen.renderer.navigation.scroll_tabs(
                            -scroll_delta,
                            num_tabs,
                            visible_width,
                        );
                        route.window.screen.render();
                        route.request_redraw();
                        return;
                    }
                }

                match delta {
                    MouseScrollDelta::LineDelta(columns, lines) => {
                        let layout = route.window.screen.sugarloaf.rich_text_layout(&0);
                        let new_scroll_px_x = columns * layout.font_size;
                        let new_scroll_px_y = lines * layout.font_size;
                        route
                            .window
                            .screen
                            .scroll(new_scroll_px_x as f64, new_scroll_px_y as f64);
                    }
                    MouseScrollDelta::PixelDelta(mut lpos) => {
                        match phase {
                            TouchPhase::Started => {
                                // Reset offset to zero.
                                route.window.screen.mouse.accumulated_scroll =
                                    Default::default();
                            }
                            TouchPhase::Moved => {
                                // When the angle between (x, 0) and (x, y) is lower than ~25 degrees
                                // (cosine is larger that 0.9) we consider this scrolling as horizontal.
                                if lpos.x.abs() / lpos.x.hypot(lpos.y) > 0.9 {
                                    lpos.y = 0.;
                                } else {
                                    lpos.x = 0.;
                                }

                                route.window.screen.scroll(lpos.x, lpos.y);
                            }
                            _ => (),
                        }
                    }
                }
            }

            WindowEvent::KeyboardInput {
                is_synthetic: false,
                event: key_event,
                ..
            } => {
                if route.has_key_wait(&key_event) {
                    if route.path != RoutePath::Terminal
                        && key_event.state == ElementState::Released
                    {
                        // Scheduler must be cleaned after leave the terminal route
                        self.scheduler.unschedule(TimerId::new(
                            Topic::Render,
                            route.window.screen.ctx().current_route(),
                        ));
                    }
                    // Overlay routes (Settings, Help, History, etc.) need a
                    // redraw after handling key input so the UI updates.
                    route.request_redraw();
                    return;
                }

                route.window.screen.context_manager.set_last_typing();
                route.window.screen.process_key_event(&key_event);

                if key_event.state == ElementState::Released
                    && self.config.hide_cursor_when_typing
                {
                    route.window.winit_window.set_cursor_visible(false);
                }
            }

            WindowEvent::Ime(ime) => {
                if route.path != RoutePath::Terminal {
                    return;
                }

                match ime {
                    Ime::Commit(text) => {
                        // Don't use bracketed paste for single char input.
                        route.window.screen.paste(&text, text.chars().count() > 1);
                    }
                    Ime::Preedit(text, cursor_offset) => {
                        let preedit = if text.is_empty() {
                            None
                        } else {
                            Some(Preedit::new(text, cursor_offset.map(|offset| offset.0)))
                        };

                        if route.window.screen.context_manager.current().ime.preedit()
                            != preedit.as_ref()
                        {
                            route
                                .window
                                .screen
                                .context_manager
                                .current_mut()
                                .ime
                                .set_preedit(preedit);
                            route.request_redraw();
                        }
                    }
                    Ime::Enabled => {
                        route
                            .window
                            .screen
                            .context_manager
                            .current_mut()
                            .ime
                            .set_enabled(true);
                    }
                    Ime::Disabled => {
                        route
                            .window
                            .screen
                            .context_manager
                            .current_mut()
                            .ime
                            .set_enabled(false);
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                on_touch(route, touch);
            }

            WindowEvent::Focused(focused) => {
                if self.config.hide_cursor_when_typing {
                    route.window.winit_window.set_cursor_visible(true);
                }

                let has_regained_focus = !route.window.is_focused && focused;
                route.window.is_focused = focused;

                if has_regained_focus {
                    route.request_redraw();
                }

                // Clear dock badge when window regains focus
                if focused {
                    route.window.screen.ctx_mut().dock_badge.clear_badge();
                }

                route.window.screen.on_focus_change(focused);
            }

            WindowEvent::Occluded(occluded) => {
                let was_occluded = route.window.is_occluded;
                route.window.is_occluded = occluded;

                // If window was occluded and is now visible, mark for one-time render
                if was_occluded && !occluded {
                    route.window.needs_render_after_occlusion = true;
                }
            }

            WindowEvent::ThemeChanged(new_theme) => {
                update_colors_based_on_theme(&mut self.config, Some(new_theme));
                route.window.screen.update_config(
                    &self.config,
                    &self.router.font_library,
                    false,
                );
                route.window.configure_window(&self.config);
            }

            WindowEvent::DroppedFile(path) => {
                if route.path != RoutePath::Terminal {
                    return;
                }

                let path: String = path.to_string_lossy().into();
                route.window.screen.paste(&(path + " "), true);
            }

            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    return;
                }

                route.window.screen.resize(new_size);
            }

            WindowEvent::ScaleFactorChanged {
                inner_size_writer: _,
                scale_factor,
            } => {
                let scale = scale_factor as f32;
                route
                    .window
                    .screen
                    .set_scale(scale, route.window.winit_window.inner_size());
                route.window.update_vblank_interval();
            }

            WindowEvent::RedrawRequested => {
                // let start = std::time::Instant::now();
                route.window.winit_window.pre_present_notify();

                route.begin_render();

                match route.path {
                    RoutePath::Assistant => {
                        route.window.screen.render_assistant(&route.assistant);
                    }
                    RoutePath::Welcome => {
                        route.window.screen.render_welcome();
                    }
                    RoutePath::Terminal => {
                        if let Some(window_update) = route.window.screen.render() {
                            use crate::context::renderable::{
                                BackgroundState, WindowUpdate,
                            };
                            match window_update {
                                WindowUpdate::Background(bg_state) => {
                                    // for now setting this as allowed because it fails on linux builds
                                    #[allow(unused_variables)]
                                    let bg_color = match bg_state {
                                        BackgroundState::Set(color) => color,
                                        BackgroundState::Reset => {
                                            self.config.colors.background.1
                                        }
                                    };

                                    #[cfg(target_os = "macos")]
                                    {
                                        route.window.winit_window.set_background_color(
                                            bg_color.r, bg_color.g, bg_color.b,
                                            bg_color.a,
                                        );
                                    }

                                    #[cfg(target_os = "windows")]
                                    {
                                        use rio_window::platform::windows::WindowExtWindows;
                                        route
                                            .window
                                            .winit_window
                                            .set_title_bar_background_color(
                                                bg_color.r, bg_color.g, bg_color.b,
                                                bg_color.a,
                                            );
                                    }
                                }
                            }
                        }

                        // Update IME cursor position after rendering to ensure it's current
                        route.window.screen.update_ime_cursor_position_if_needed(
                            &route.window.winit_window,
                        );
                    }
                    RoutePath::ConfirmQuit => {
                        route.window.screen.render_dialog(
                            "Quit Volt?",
                            "Continue -> press escape key",
                            "Quit -> press enter key",
                        );
                    }
                    RoutePath::Settings => {
                        route
                            .window
                            .screen
                            .render_settings(&route.settings_editor, route.settings_category, route.settings_in_sidebar);
                    }
                    RoutePath::Help => {
                        route.window.screen.render_help();
                    }
                    RoutePath::TmuxPicker => {
                        route.window.screen.render_tmux_picker(
                            &route.tmux_sessions,
                            route.tmux_selected,
                        );
                    }
                    RoutePath::EnvViewer => {
                        route.window.screen.render_env_viewer(route.env_scroll, route.env_selected);
                    }
                    RoutePath::Bookmarks => {
                        route.window.screen.render_bookmarks(route.bookmarks_scroll, &route.bookmarks_cache);
                    }
                    RoutePath::History => {
                        route.window.screen.render_history(
                            route.history_scroll,
                            route.history_selected,
                        );
                    }
                    RoutePath::Connections => {
                        route.window.screen.render_connections(
                            &route.connections_list,
                            route.connections_selected,
                        );
                    }
                    RoutePath::SlashCommands => {
                        route.window.screen.render_slash_commands(route.slash_commands_scroll);
                    }
                    RoutePath::Layouts => {
                        route.window.screen.render_layouts(route.layouts_selected);
                    }
                }

                // let duration = start.elapsed();
                // println!("Time elapsed in render() is: {:?}", duration);
                // }

                if self.config.renderer.strategy.is_game() {
                    route.request_redraw();
                } else if route
                    .window
                    .screen
                    .ctx()
                    .current()
                    .renderable_content
                    .pending_update
                    .is_dirty()
                {
                    route.schedule_redraw(
                        &mut self.scheduler,
                        route.window.screen.ctx().current_route(),
                    );
                }

                event_loop.set_control_flow(ControlFlow::Wait);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let control_flow = match self.scheduler.update() {
            Some(instant) => ControlFlow::WaitUntil(instant),
            None => ControlFlow::Wait,
        };
        event_loop.set_control_flow(control_flow);
    }

    fn open_config(&mut self, event_loop: &ActiveEventLoop) {
        if self.config.navigation.open_config_with_split {
            self.router.open_config_split(&self.config);
        } else {
            self.router.open_config_window(
                event_loop,
                self.event_proxy.clone(),
                &self.config,
            );
        }
    }

    fn hook_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        key: &rio_window::event::KeyEvent,
        modifiers: &rio_window::event::Modifiers,
    ) {
        let window_id = match self.router.get_focused_route() {
            Some(window_id) => window_id,
            None => return,
        };

        let route = match self.router.routes.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        // For menu-triggered events, we need to temporarily set the correct modifiers
        // since menu events don't trigger ModifiersChanged events.
        let original_modifiers = route.window.screen.modifiers;

        // Use the modifiers passed from the menu action
        route.window.screen.set_modifiers(*modifiers);

        // Process the key event
        route.window.screen.process_key_event(key);

        // Restore the original modifiers
        route.window.screen.set_modifiers(original_modifiers);
    }

    // Emitted when the event loop is being shut down.
    // This is irreversible - if this event is emitted, it is guaranteed to be the last event that gets emitted.
    // You generally want to treat this as an “do on quit” event.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Save session history to disk before exiting
        for route in self.router.routes.values() {
            route.window.screen.context_manager.session_recorder.save_to_disk();
        }

        // Save window state before exiting
        let mut state = crate::window_state::WindowState::new();
        for route in self.router.routes.values() {
            let win = &route.window.winit_window;
            let pos = win.outer_position().unwrap_or_default();
            let size = win.inner_size();
            let is_fullscreen = win.fullscreen().is_some();
            state.add_window(
                pos.x as f64,
                pos.y as f64,
                size.width as f64,
                size.height as f64,
                is_fullscreen,
            );
        }
        if let Err(e) = state.save() {
            tracing::warn!("Failed to save window state: {e}");
        }

        // Ensure that all the windows are dropped, so the destructors for
        // Renderer and contexts ran.
        self.router.routes.clear();

        // SAFETY: The clipboard must be dropped before the event loop, so use the nop clipboard
        // as a safe placeholder.
        self.router.clipboard =
            std::rc::Rc::new(std::cell::RefCell::new(Clipboard::new_nop()));

        std::process::exit(0);
    }
}

#[cfg(all(
    feature = "audio",
    not(target_os = "macos"),
    not(target_os = "windows")
))]
fn play_bell_sound() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;

    let config = device.default_output_config()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => run_bell::<f32>(&device, &config.into()),
        cpal::SampleFormat::I16 => run_bell::<i16>(&device, &config.into()),
        cpal::SampleFormat::U16 => run_bell::<u16>(&device, &config.into()),
        _ => Err("Unsupported sample format".into()),
    }
}

#[cfg(all(
    feature = "audio",
    not(target_os = "macos"),
    not(target_os = "windows")
))]
fn run_bell<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
) -> Result<(), Box<dyn Error>>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;
    let duration_secs = crate::constants::BELL_DURATION.as_secs_f32();
    let total_samples = (sample_rate * duration_secs) as usize;

    let mut sample_clock = 0f32;
    let mut samples_played = 0usize;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                if samples_played >= total_samples {
                    for sample in frame.iter_mut() {
                        *sample = T::from_sample(0.0);
                    }
                } else {
                    let value = (sample_clock * 440.0 * 2.0 * std::f32::consts::PI
                        / sample_rate)
                        .sin()
                        * 0.2;
                    for sample in frame.iter_mut() {
                        *sample = T::from_sample(value);
                    }
                    sample_clock += 1.0;
                    samples_played += 1;
                }
            }
        },
        |err| tracing::error!("Audio stream error: {}", err),
        None,
    )?;

    stream.play()?;
    std::thread::sleep(crate::constants::BELL_DURATION);

    Ok(())
}
