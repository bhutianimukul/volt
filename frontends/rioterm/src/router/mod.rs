pub mod routes;
mod window;
use crate::event::EventProxy;
use crate::router::window::{configure_window, create_window_builder};
use crate::screen::{Screen, ScreenWindowProperties};
use assistant::Assistant;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use rio_backend::clipboard::Clipboard;
use rio_backend::config::Config as RioConfig;
use rio_backend::error::{RioError, RioErrorLevel, RioErrorType};

use rio_window::event_loop::ActiveEventLoop;
use rio_window::keyboard::{Key, NamedKey};
#[cfg(not(any(target_os = "macos", windows)))]
use rio_window::platform::startup_notify::{
    self, EventLoopExtStartupNotify, WindowAttributesExtStartupNotify,
};
use rio_window::window::{Window, WindowId};
use routes::{assistant, RoutePath};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

// 𜱭𜱭 unicode is not available yet for all OS
// https://www.unicode.org/charts/PDF/Unicode-16.0/U160-1CC00.pdf
// #[cfg(any(target_os = "macos", target_os = "windows"))]
// const RIO_TITLE: &str = "𜱭𜱭";
// #[cfg(not(any(target_os = "macos", target_os = "windows")))]
const RIO_TITLE: &str = "▲";

pub struct Route<'a> {
    pub assistant: assistant::Assistant,
    pub path: RoutePath,
    pub window: RouteWindow<'a>,
    pub settings_editor: crate::settings_editor::SettingsEditor,
    /// Cached tmux sessions for the picker: (id, name, attached)
    pub tmux_sessions: Vec<(String, String, bool)>,
    /// Currently selected index in the tmux picker
    pub tmux_selected: usize,
    /// Scroll offset for the environment viewer
    pub env_scroll: usize,
    /// Currently selected index in the environment viewer
    pub env_selected: usize,
    /// Scroll offset for the history viewer
    pub history_scroll: usize,
    /// Currently selected index in the history viewer
    pub history_selected: usize,
    /// Scroll offset for the bookmarks viewer
    pub bookmarks_scroll: usize,
    /// Currently selected index in the bookmarks viewer
    pub bookmarks_selected: usize,
    /// Cached bookmarks for the viewer (loaded once when opening)
    pub bookmarks_cache: Vec<crate::bookmarks::Bookmark>,
    /// Currently selected index in the connections viewer
    pub connections_selected: usize,
    /// Cached connections list: (name, type_name, host_info, command)
    pub connections_list: Vec<(String, String, String, String)>,
    /// Scroll offset for the slash commands viewer
    pub slash_commands_scroll: usize,
    /// Currently selected index in the layouts viewer
    pub layouts_selected: usize,
    /// Currently selected category index in the settings sidebar
    pub settings_category: usize,
    /// Whether focus is on the settings sidebar (true) or settings panel (false)
    pub settings_in_sidebar: bool,
    /// Currently selected category in the help sidebar
    pub help_category: usize,
    /// Currently selected item in the help panel
    pub help_selected: usize,
    /// Whether focus is on the help sidebar (true) or help panel (false)
    pub help_in_sidebar: bool,
    /// Session sharing state
    pub sharing_state: crate::router::routes::session_sharing::SharingState,
    /// Selected action in session sharing (0=Host, 1=Connect)
    pub sharing_selected: usize,
    /// Selected format in session export
    pub export_selected: usize,
    /// Last export result message
    pub export_result: Option<crate::router::routes::session_export::ExportResult>,
    /// Selected index in time travel view
    pub time_travel_selected: usize,
    /// Scroll offset in time travel view
    pub time_travel_scroll: usize,
}

impl Route<'_> {
    /// Create a performer.
    #[inline]
    pub fn new(
        assistant: assistant::Assistant,
        path: RoutePath,
        window: RouteWindow,
    ) -> Route {
        Route {
            assistant,
            path,
            window,
            settings_editor: crate::settings_editor::SettingsEditor::new(),
            tmux_sessions: Vec::new(),
            tmux_selected: 0,
            env_scroll: 0,
            env_selected: 0,
            history_scroll: 0,
            history_selected: 0,
            bookmarks_scroll: 0,
            bookmarks_selected: 0,
            bookmarks_cache: Vec::new(),
            connections_selected: 0,
            connections_list: Vec::new(),
            slash_commands_scroll: 0,
            layouts_selected: 0,
            settings_category: 0,
            settings_in_sidebar: true,
            help_category: 0,
            help_selected: 0,
            help_in_sidebar: true,
            sharing_state: crate::router::routes::session_sharing::SharingState::default(
            ),
            sharing_selected: 0,
            export_selected: 0,
            export_result: None,
            time_travel_selected: 0,
            time_travel_scroll: 0,
        }
    }
}

impl Route<'_> {
    #[inline]
    pub fn request_redraw(&mut self) {
        self.window.winit_window.request_redraw();
    }

    #[inline]
    pub fn schedule_redraw(
        &mut self,
        scheduler: &mut crate::scheduler::Scheduler,
        route_id: usize,
    ) {
        #[cfg(target_os = "macos")]
        {
            // On macOS, use direct redraw as CVDisplayLink handles VSync
            let _ = (scheduler, route_id); // Suppress warnings
            self.request_redraw();
        }

        #[cfg(not(target_os = "macos"))]
        {
            use crate::event::{EventPayload, RioEvent, RioEventType};
            use crate::scheduler::{TimerId, Topic};

            // Windows and Linux use the frame scheduler with refresh rate timing
            let timer_id = TimerId::new(Topic::Render, route_id);
            let event = EventPayload::new(
                RioEventType::Rio(RioEvent::Render),
                self.window.winit_window.id(),
            );

            // Schedule a render if not already scheduled
            // Use vblank_interval for proper frame timing
            if !scheduler.scheduled(timer_id) {
                scheduler.schedule(event, self.window.vblank_interval, false, timer_id);
            }
        }
    }

    #[inline]
    pub fn begin_render(&mut self) {
        self.window.render_timestamp = Instant::now();

        // // Track frame count for performance monitoring
        // use std::collections::HashMap;
        // use std::sync::Mutex;

        // static FRAME_COUNTERS: Mutex<
        //     Option<HashMap<rio_window::window::WindowId, (u64, std::time::Instant)>>,
        // > = Mutex::new(None);
        // static LAST_LOG: Mutex<Option<std::time::Instant>> = Mutex::new(None);

        // let window_id = self.window.winit_window.id();

        // {
        //     // Use try_lock to avoid blocking other windows during performance logging
        //     let mut counters = match FRAME_COUNTERS.try_lock() {
        //         Ok(guard) => guard,
        //         Err(_) => return, // Skip performance logging if another window is using it
        //     };
        //     if counters.is_none() {
        //         *counters = Some(HashMap::new());
        //     }

        //     let mut last_log = match LAST_LOG.try_lock() {
        //         Ok(guard) => guard,
        //         Err(_) => return, // Skip performance logging if another window is using it
        //     };
        //     if last_log.is_none() {
        //         *last_log = Some(std::time::Instant::now());
        //     }

        //     if let (Some(ref mut counters_map), Some(ref mut last_log_time)) =
        //         (counters.as_mut(), last_log.as_mut())
        //     {
        //         let entry = counters_map
        //             .entry(window_id)
        //             .or_insert((0, std::time::Instant::now()));
        //         entry.0 += 1;

        //         // Log performance stats every 5 seconds
        //         if last_log_time.elapsed().as_secs() >= 5 {
        //             let total_windows = counters_map.len();
        //             if total_windows > 1 {
        //                 tracing::warn!(
        //                     "[PERF] Multi-window performance stats ({} windows):",
        //                     total_windows
        //                 );
        //                 let mut sorted_windows: Vec<_> = counters_map.iter().collect();
        //                 sorted_windows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by frame count descending

        //                 for (i, (id, (frames, start_time))) in
        //                     sorted_windows.iter().enumerate()
        //                 {
        //                     let fps = *frames as f64 / start_time.elapsed().as_secs_f64();
        //                     let priority = if i == 0 { "HIGH" } else { "LOW" };
        //                     tracing::warn!(
        //                         "[PERF]   Window {:?}: {:.1} FPS ({} frames) [{}]",
        //                         id,
        //                         fps,
        //                         frames,
        //                         priority
        //                     );
        //                 }

        //                 // Check for significant FPS differences
        //                 if sorted_windows.len() >= 2 {
        //                     let highest_fps = sorted_windows[0].1 .0 as f64
        //                         / sorted_windows[0].1 .1.elapsed().as_secs_f64();
        //                     let lowest_fps = sorted_windows.last().unwrap().1 .0 as f64
        //                         / sorted_windows
        //                             .last()
        //                             .unwrap()
        //                             .1
        //                              .1
        //                             .elapsed()
        //                             .as_secs_f64();
        //                     if highest_fps > lowest_fps * 2.0 {
        //                         tracing::error!("[PERF] SIGNIFICANT FPS DIFFERENCE: {:.1} vs {:.1} FPS - window prioritization detected!", highest_fps, lowest_fps);
        //                     }
        //                 }
        //             }
        //             **last_log_time = std::time::Instant::now();
        //             // Reset counters
        //             for (_, (frames, start_time)) in counters_map.iter_mut() {
        //                 *frames = 0;
        //                 *start_time = std::time::Instant::now();
        //             }
        //         }
        //     }
        // }
    }

    #[inline]
    pub fn update_config(
        &mut self,
        config: &RioConfig,
        db: &rio_backend::sugarloaf::font::FontLibrary,
        should_update_font: bool,
    ) {
        self.window
            .screen
            .update_config(config, db, should_update_font);
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn set_window_subtitle(&mut self, subtitle: &str) {
        #[cfg(target_os = "macos")]
        self.window.winit_window.set_subtitle(subtitle);
    }

    #[inline]
    pub fn set_window_title(&mut self, title: &str) {
        self.window.winit_window.set_title(title);
    }

    #[inline]
    pub fn report_error(&mut self, error: &RioError) {
        if error.report == RioErrorType::ConfigurationNotFound {
            self.path = RoutePath::Welcome;
            return;
        }

        self.assistant.set(error.to_owned());
        self.path = RoutePath::Assistant;
    }

    #[inline]
    pub fn clear_errors(&mut self) {
        self.assistant.clear();
        self.path = RoutePath::Terminal;
    }

    #[inline]
    pub fn confirm_quit(&mut self) {
        self.path = RoutePath::ConfirmQuit;
    }

    #[inline]
    pub fn quit(&mut self) {
        std::process::exit(0);
    }

    #[inline]
    pub fn has_key_wait(&mut self, key_event: &rio_window::event::KeyEvent) -> bool {
        if self.path == RoutePath::Terminal {
            return false;
        }

        let is_enter = key_event.logical_key == Key::Named(NamedKey::Enter);
        if self.path == RoutePath::Assistant {
            if self.assistant.is_warning() && is_enter {
                self.assistant.clear();
                self.path = RoutePath::Terminal;
            } else {
                return true;
            }
        }

        if self.path == RoutePath::ConfirmQuit {
            if key_event.logical_key == Key::Named(NamedKey::Escape) {
                self.path = RoutePath::Terminal;
            } else if is_enter {
                self.quit();

                return true;
            }
        }

        if self.path == RoutePath::Welcome && is_enter {
            rio_backend::config::create_config_file(None);
            self.path = RoutePath::Terminal;
        }

        if self.path == RoutePath::Settings {
            // Only handle key press events, not releases
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }

            let editor = &mut self.settings_editor;

            if editor.editing {
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        editor.cancel_edit();
                    }
                    Key::Named(NamedKey::Enter) => {
                        editor.confirm_edit();
                    }
                    Key::Named(NamedKey::Backspace) => {
                        editor.backspace();
                    }
                    Key::Character(c) => {
                        for ch in c.chars() {
                            editor.type_char(ch);
                        }
                    }
                    Key::Named(NamedKey::Space) => {
                        editor.type_char(' ');
                    }
                    _ => {}
                }
            } else if editor.searching {
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        editor.toggle_search();
                    }
                    Key::Named(NamedKey::Enter) => {
                        // Stop searching, keep filter active
                        editor.searching = false;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        editor.backspace();
                    }
                    Key::Character(c) => {
                        for ch in c.chars() {
                            editor.type_char(ch);
                        }
                    }
                    Key::Named(NamedKey::Space) => {
                        editor.type_char(' ');
                    }
                    _ => {}
                }
            } else if self.settings_in_sidebar {
                // Sidebar mode: navigate categories
                let categories = editor.categories();
                let cat_count = categories.len();
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.path = RoutePath::Terminal;
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if self.settings_category > 0 {
                            self.settings_category -= 1;
                            // Reset panel selection when category changes
                            editor.selected_index = 0;
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        if cat_count > 0 && self.settings_category + 1 < cat_count {
                            self.settings_category += 1;
                            editor.selected_index = 0;
                        }
                    }
                    Key::Named(NamedKey::Enter) | Key::Named(NamedKey::ArrowRight) => {
                        self.settings_in_sidebar = false;
                    }
                    Key::Named(NamedKey::Tab) => {
                        self.settings_in_sidebar = false;
                    }
                    Key::Character(c) if c.as_str() == "/" => {
                        editor.toggle_search();
                    }
                    Key::Character(c) if c.as_str() == "i" => {
                        if let Some((source, imported)) =
                            crate::config_import::auto_import()
                        {
                            let toml = imported.to_volt_toml();
                            let config_path = rio_backend::config::config_file_path();
                            let _ = std::fs::write(&config_path, &toml);
                            tracing::info!("Imported config from {}", source.name());
                        }
                    }
                    _ => {}
                }
            } else {
                // Panel mode: navigate settings within category
                let categories = editor.categories();
                let current_cat = categories
                    .get(self.settings_category)
                    .cloned()
                    .unwrap_or_default();
                let cat_items = editor.items_for_category(&current_cat);
                let item_count = cat_items.len();
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.path = RoutePath::Terminal;
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if editor.selected_index > 0 {
                            editor.selected_index -= 1;
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        if item_count > 0 && editor.selected_index + 1 < item_count {
                            editor.selected_index += 1;
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.settings_in_sidebar = true;
                    }
                    Key::Named(NamedKey::Tab) => {
                        self.settings_in_sidebar = true;
                    }
                    Key::Named(NamedKey::Enter) => {
                        // Find the real index into editor.items for this category item
                        if let Some(real_item) = cat_items.get(editor.selected_index) {
                            let real_idx = editor
                                .items
                                .iter()
                                .position(|it| it.key == real_item.key);
                            if let Some(ri) = real_idx {
                                if editor.items[ri].key == "window.background-image" {
                                    // Open native image picker instead of text editor
                                    if let Some(path) = crate::image_picker::pick_image()
                                    {
                                        editor.items[ri].value =
                                            crate::settings_editor::SettingValue::String(
                                                path.clone(),
                                            );
                                        editor.save_background_image(&path);
                                    }
                                } else if editor.items[ri].value.is_bool() {
                                    editor.items[ri].value = match &editor.items[ri].value
                                    {
                                        crate::settings_editor::SettingValue::Bool(v) => {
                                            crate::settings_editor::SettingValue::Bool(!v)
                                        }
                                        other => other.clone(),
                                    };
                                    editor.save_setting_by_index(ri);
                                } else {
                                    editor.edit_buffer = editor.items[ri].value.display();
                                    editor.editing = true;
                                    // Store the real index for editing
                                    editor.editing_real_index = Some(ri);
                                }
                            }
                        }
                    }
                    Key::Character(c) if c.as_str() == "/" => {
                        editor.toggle_search();
                    }
                    Key::Character(c) if c.as_str() == "i" => {
                        if let Some((source, imported)) =
                            crate::config_import::auto_import()
                        {
                            let toml = imported.to_volt_toml();
                            let config_path = rio_backend::config::config_file_path();
                            let _ = std::fs::write(&config_path, &toml);
                            tracing::info!("Imported config from {}", source.name());
                        }
                    }
                    _ => {}
                }
            }
            return true;
        }

        if self.path == RoutePath::Help {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::Tab) => {
                    self.help_in_sidebar = !self.help_in_sidebar;
                    if self.help_in_sidebar {
                        // Reset panel selection when moving to sidebar
                    } else {
                        self.help_selected = 0;
                    }
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if self.help_in_sidebar {
                        if self.help_category > 0 {
                            self.help_category -= 1;
                            self.help_selected = 0;
                        }
                    } else if self.help_selected > 0 {
                        self.help_selected -= 1;
                    }
                }
                Key::Named(NamedKey::ArrowDown) => {
                    if self.help_in_sidebar {
                        if self.help_category + 1
                            < crate::router::routes::help::HELP_CATEGORIES.len()
                        {
                            self.help_category += 1;
                            self.help_selected = 0;
                        }
                    } else {
                        let max = crate::router::routes::help::category_item_count(
                            self.help_category,
                        );
                        if max > 0 && self.help_selected + 1 < max {
                            self.help_selected += 1;
                        }
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    if self.help_in_sidebar {
                        self.help_in_sidebar = false;
                        self.help_selected = 0;
                    } else if self.help_category == 2 {
                        // Actions category — open the feature
                        match crate::router::routes::help::action_route(
                            self.help_selected,
                        ) {
                            Some("ai") => self.path = RoutePath::Assistant,
                            Some("history") => self.path = RoutePath::History,
                            Some("env") => self.path = RoutePath::EnvViewer,
                            Some("bookmarks") => self.path = RoutePath::Bookmarks,
                            Some("connections") => self.path = RoutePath::Connections,
                            Some("tmux") => self.path = RoutePath::TmuxPicker,
                            Some("slash") => self.path = RoutePath::SlashCommands,
                            Some("layouts") => self.path = RoutePath::Layouts,
                            Some("sharing") => self.path = RoutePath::SessionSharing,
                            Some("export") => self.path = RoutePath::SessionExport,
                            Some("timetravel") => self.path = RoutePath::TimeTravel,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::History {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.history_scroll = 0;
                    self.history_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    let max = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(50)
                        .len();
                    if max > 0 && self.history_selected + 1 < max {
                        self.history_selected += 1;
                    }
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.history_selected = self.history_selected.saturating_sub(1);
                }
                Key::Named(NamedKey::PageDown) => {
                    let max = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(50)
                        .len();
                    self.history_selected =
                        (self.history_selected + 20).min(max.saturating_sub(1));
                }
                Key::Named(NamedKey::PageUp) => {
                    self.history_selected = self.history_selected.saturating_sub(20);
                }
                Key::Named(NamedKey::Enter) => {
                    // Paste selected command text into terminal (without executing)
                    // and also copy to clipboard
                    let recent = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(50);
                    let display_order: Vec<_> = recent.into_iter().rev().collect();
                    if let Some(entry) = display_order.get(self.history_selected) {
                        if !entry.command.is_empty() {
                            // Copy to clipboard
                            self.window.screen.clipboard.borrow_mut().set(
                                rio_backend::clipboard::ClipboardType::Clipboard,
                                entry.command.clone(),
                            );
                            // Write to PTY without trailing \r so user can review before running
                            let cmd = entry.command.clone();
                            self.window
                                .screen
                                .ctx_mut()
                                .current_mut()
                                .messenger
                                .send_write(cmd.into_bytes());
                        }
                    }
                    self.history_scroll = 0;
                    self.history_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Character(c) if c.as_str() == "e" || c.as_str() == "E" => {
                    // Export session as .cast file to Desktop
                    let entries = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(500);
                    let mut recording =
                        crate::session_export::SessionRecording::new(80, 24);
                    let mut ts = 0.0_f64;
                    for entry in entries.iter().rev() {
                        recording.add_input(ts, format!("{}\r\n", entry.command));
                        ts += 0.5;
                        if !entry.output_preview.is_empty() {
                            recording.add_output(ts, entry.output_preview.clone());
                            ts += 0.3;
                        }
                    }
                    let desktop = dirs::desktop_dir().unwrap_or_else(|| {
                        std::path::PathBuf::from(
                            std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
                        )
                        .join("Desktop")
                    });
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let filename = format!("volt-session-{}.cast", timestamp);
                    let path = desktop.join(&filename);
                    match recording.save_to_file(&path) {
                        Ok(()) => {
                            tracing::info!("Session exported to {}", path.display());
                        }
                        Err(e) => {
                            tracing::error!("Failed to export session: {}", e);
                        }
                    }
                }
                Key::Character(c) if c.as_str() == "b" || c.as_str() == "B" => {
                    // Bookmark the selected command (use same source as display)
                    let recent = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(50);
                    let entries: Vec<_> = recent.into_iter().rev().collect();
                    if let Some(entry) = entries.get(self.history_selected) {
                        let mut store = crate::bookmarks::BookmarkStore::load();
                        store.add(
                            entry.command.clone(),
                            entry.output_preview.clone(),
                            entry.working_dir.clone(),
                            entry.exit_code,
                        );
                        let _ = store.save();
                        tracing::info!("Bookmarked: {}", entry.command);
                    }
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::EnvViewer {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.env_scroll = 0;
                    self.env_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    let max = crate::env_inspector::get_all_env_vars().len();
                    if max > 0 && self.env_selected + 1 < max {
                        self.env_selected += 1;
                    }
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.env_selected = self.env_selected.saturating_sub(1);
                }
                Key::Named(NamedKey::PageDown) => {
                    let max = crate::env_inspector::get_all_env_vars().len();
                    self.env_selected =
                        (self.env_selected + 20).min(max.saturating_sub(1));
                }
                Key::Named(NamedKey::PageUp) => {
                    self.env_selected = self.env_selected.saturating_sub(20);
                }
                Key::Named(NamedKey::Enter) => {
                    // Copy the selected variable's KEY=VALUE to the PTY
                    let all_vars = crate::env_inspector::get_all_env_vars();
                    if let Some(var) = all_vars.get(self.env_selected) {
                        let text = format!("{}={}", var.key, var.value);
                        self.window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_write(text.into_bytes());
                        self.env_scroll = 0;
                        self.env_selected = 0;
                        self.path = RoutePath::Terminal;
                    }
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::Bookmarks {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.bookmarks_scroll = 0;
                    self.bookmarks_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    let max = self.bookmarks_cache.len();
                    if max > 0 && self.bookmarks_selected + 1 < max {
                        self.bookmarks_selected += 1;
                    }
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.bookmarks_selected = self.bookmarks_selected.saturating_sub(1);
                }
                Key::Named(NamedKey::PageDown) => {
                    let max = self.bookmarks_cache.len();
                    self.bookmarks_selected =
                        (self.bookmarks_selected + 20).min(max.saturating_sub(1));
                }
                Key::Named(NamedKey::PageUp) => {
                    self.bookmarks_selected = self.bookmarks_selected.saturating_sub(20);
                }
                Key::Named(NamedKey::Enter) => {
                    // Paste selected bookmark command into terminal
                    if let Some(bm) = self.bookmarks_cache.get(self.bookmarks_selected) {
                        if !bm.command.is_empty() {
                            let cmd = bm.command.clone();
                            self.window
                                .screen
                                .ctx_mut()
                                .current_mut()
                                .messenger
                                .send_write(cmd.into_bytes());
                        }
                    }
                    self.bookmarks_scroll = 0;
                    self.bookmarks_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Character(c) if c.as_str() == "d" || c.as_str() == "D" => {
                    // Delete selected bookmark
                    if self.bookmarks_selected < self.bookmarks_cache.len() {
                        self.bookmarks_cache.remove(self.bookmarks_selected);
                        let mut store = crate::bookmarks::BookmarkStore::load();
                        store.remove(self.bookmarks_selected);
                        let _ = store.save();
                        if self.bookmarks_selected >= self.bookmarks_cache.len()
                            && self.bookmarks_selected > 0
                        {
                            self.bookmarks_selected -= 1;
                        }
                    }
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::Connections {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.connections_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if self.connections_selected > 0 {
                        self.connections_selected -= 1;
                    }
                }
                Key::Named(NamedKey::ArrowDown) => {
                    if !self.connections_list.is_empty()
                        && self.connections_selected < self.connections_list.len() - 1
                    {
                        self.connections_selected += 1;
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    if let Some((_name, _type_name, _host_info, command)) =
                        self.connections_list.get(self.connections_selected)
                    {
                        let cmd = format!("{}\r", command);
                        self.window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_write(cmd.into_bytes());
                        self.connections_selected = 0;
                        self.path = RoutePath::Terminal;
                    }
                }
                Key::Character(c) if c.as_str() == "n" || c.as_str() == "N" => {
                    #[cfg(target_os = "macos")]
                    {
                        use objc::runtime::{Class, Object};
                        use objc::{msg_send, sel, sel_impl};
                        use std::ffi::CString;

                        unsafe {
                            let ns_string_class = Class::get("NSString").unwrap();
                            let alert_class = Class::get("NSAlert").unwrap();
                            let text_field_class = Class::get("NSTextField").unwrap();

                            // Helper to show input dialog
                            let ask = |title: &str, prompt: &str| -> Option<String> {
                                let alert: *mut Object = msg_send![alert_class, new];
                                let title_ns: *mut Object = msg_send![ns_string_class,
                                    stringWithUTF8String: CString::new(title).unwrap().as_ptr()];
                                let _: () = msg_send![alert, setMessageText: title_ns];
                                let info_ns: *mut Object = msg_send![ns_string_class,
                                    stringWithUTF8String: CString::new(prompt).unwrap().as_ptr()];
                                let _: () = msg_send![alert, setInformativeText: info_ns];
                                let ok_ns: *mut Object = msg_send![ns_string_class,
                                    stringWithUTF8String: CString::new("OK").unwrap().as_ptr()];
                                let _: () = msg_send![alert, addButtonWithTitle: ok_ns];
                                let cancel_ns: *mut Object = msg_send![ns_string_class,
                                    stringWithUTF8String: CString::new("Cancel").unwrap().as_ptr()];
                                let _: () =
                                    msg_send![alert, addButtonWithTitle: cancel_ns];

                                let frame: ((f64, f64), (f64, f64)) =
                                    ((0.0, 0.0), (300.0, 24.0));
                                let input: *mut Object =
                                    msg_send![text_field_class, alloc];
                                let input: *mut Object =
                                    msg_send![input, initWithFrame: frame];
                                let _: () = msg_send![alert, setAccessoryView: input];
                                let window: *mut Object = msg_send![alert, window];
                                let _: () =
                                    msg_send![window, setInitialFirstResponder: input];

                                let response: i64 = msg_send![alert, runModal];
                                if response == 1000 {
                                    let value: *mut Object =
                                        msg_send![input, stringValue];
                                    let utf8: *const std::ffi::c_char =
                                        msg_send![value, UTF8String];
                                    let s = std::ffi::CStr::from_ptr(utf8)
                                        .to_string_lossy()
                                        .to_string();
                                    if !s.is_empty() {
                                        Some(s)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };

                            // Collect connection details
                            if let Some(name) = ask("New Connection", "Connection name:")
                            {
                                if let Some(conn_type) = ask("Connection Type", "Type (ssh, mysql, postgres, redis, kubectl, docker):") {
                                    if let Some(host) = ask("Host", "Hostname or IP:") {
                                        let user = ask("User (optional)", "Username (leave empty to skip):");

                                        // Build TOML entry using proper serialization
                                        let conn_path = crate::connections::connection_config_path();
                                        let file_content = std::fs::read_to_string(&conn_path).unwrap_or_default();
                                        let mut doc: toml::Table = file_content.parse().unwrap_or_default();

                                        let connections_table = doc
                                            .entry("connections")
                                            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                                        if let toml::Value::Table(conns) = connections_table {
                                            let mut entry = toml::Table::new();
                                            entry.insert("type".to_string(), toml::Value::String(conn_type.clone()));
                                            entry.insert("host".to_string(), toml::Value::String(host.clone()));
                                            if let Some(u) = user {
                                                entry.insert("user".to_string(), toml::Value::String(u));
                                            }
                                            conns.insert(name.clone(), toml::Value::Table(entry));
                                        }

                                        if let Ok(output) = toml::to_string_pretty(&doc) {
                                            let _ = std::fs::write(&conn_path, output);
                                        }
                                        tracing::info!("Connection '{}' created", name);

                                        // Refresh the list
                                        match crate::connections::load_connections() {
                                            Ok(config) => {
                                                self.connections_list = config.connections.iter()
                                                    .map(|(cname, conn)| (cname.clone(), conn.type_name().to_string(), conn.to_command(), conn.to_command()))
                                                    .collect();
                                            }
                                            Err(e) => tracing::error!("Failed to reload connections: {}", e),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Key::Character(c) if c.as_str() == "d" || c.as_str() == "D" => {
                    if let Some((name, _, _, _)) =
                        self.connections_list.get(self.connections_selected)
                    {
                        let conn_path = crate::connections::connection_config_path();
                        if let Ok(content) = std::fs::read_to_string(&conn_path) {
                            // Remove the [connections.NAME] section
                            let section_header = format!("[connections.{}]", name);
                            let lines: Vec<&str> = content.lines().collect();
                            let mut remove_start = None;
                            let mut remove_end = None;
                            for (i, line) in lines.iter().enumerate() {
                                if line.trim() == section_header {
                                    remove_start = Some(i);
                                }
                                if remove_start.is_some()
                                    && i > remove_start.unwrap()
                                    && line.starts_with('[')
                                {
                                    remove_end = Some(i);
                                    break;
                                }
                            }
                            if let Some(start) = remove_start {
                                let end = remove_end.unwrap_or(lines.len());
                                let mut lines = lines;
                                lines.drain(start..end);
                                let _ = std::fs::write(&conn_path, lines.join("\n"));
                            }
                        }
                        // Refresh
                        match crate::connections::load_connections() {
                            Ok(config) => {
                                self.connections_list = config
                                    .connections
                                    .iter()
                                    .map(|(cname, conn)| {
                                        (
                                            cname.clone(),
                                            conn.type_name().to_string(),
                                            conn.to_command(),
                                            conn.to_command(),
                                        )
                                    })
                                    .collect();
                            }
                            Err(e) => {
                                tracing::error!("Failed to reload connections: {}", e)
                            }
                        }
                        if self.connections_selected >= self.connections_list.len()
                            && self.connections_selected > 0
                        {
                            self.connections_selected -= 1;
                        }
                    }
                }
                Key::Character(c) if c.as_str() == "e" || c.as_str() == "E" => {
                    // Open connections.toml in the default editor
                    let path = crate::connections::connection_config_path();
                    // If file doesn't exist, create with template
                    if !path.exists() {
                        let _ =
                            std::fs::write(&path, crate::connections::default_template());
                    }
                    // Open in editor via $EDITOR or fallback to vi
                    let editor =
                        std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                    let quoted_editor = format!("'{}'", editor.replace('\'', "'\\''"));
                    let quoted_path = format!(
                        "'{}'",
                        path.display().to_string().replace('\'', "'\\''")
                    );
                    let cmd = format!("{} {}\r", quoted_editor, quoted_path);
                    self.window
                        .screen
                        .ctx_mut()
                        .current_mut()
                        .messenger
                        .send_write(cmd.into_bytes());
                    self.connections_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::SlashCommands {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.slash_commands_scroll = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.slash_commands_scroll =
                        self.slash_commands_scroll.saturating_add(1);
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.slash_commands_scroll =
                        self.slash_commands_scroll.saturating_sub(1);
                }
                Key::Named(NamedKey::PageDown) => {
                    self.slash_commands_scroll =
                        self.slash_commands_scroll.saturating_add(20);
                }
                Key::Named(NamedKey::PageUp) => {
                    self.slash_commands_scroll =
                        self.slash_commands_scroll.saturating_sub(20);
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::Layouts {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.layouts_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if self.layouts_selected >= 2 {
                        self.layouts_selected -= 2;
                    }
                }
                Key::Named(NamedKey::ArrowDown) => {
                    if self.layouts_selected < 2 {
                        self.layouts_selected += 2;
                    }
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    if self.layouts_selected % 2 == 1 {
                        self.layouts_selected -= 1;
                    }
                }
                Key::Named(NamedKey::ArrowRight) => {
                    if self.layouts_selected % 2 == 0 && self.layouts_selected < 3 {
                        self.layouts_selected += 1;
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    // Apply the selected layout preset
                    match self.layouts_selected {
                        0 => {
                            // Side by side: one split right
                            self.window.screen.split_right();
                        }
                        1 => {
                            // Dev: split right, then split the right pane down
                            self.window.screen.split_right();
                            self.window.screen.split_down();
                        }
                        2 => {
                            // Quad: 4 equal panes in a 2x2 grid
                            // 1. split_right → [A, *B*] focus on B
                            // 2. split_down  → [A, B-top, *B-bot*] focus on B-bot
                            // 3. prev_split  → [A, *B-top*, B-bot] focus on B-top
                            // 4. prev_split  → [*A*, B-top, B-bot] focus on A
                            // 5. split_down  → [A-top, *A-bot*, B-top, B-bot] focus on A-bot
                            self.window.screen.split_right();
                            self.window.screen.split_down();
                            self.window.screen.context_manager.select_prev_split();
                            self.window.screen.context_manager.select_prev_split();
                            self.window.screen.split_down();
                        }
                        3 => {
                            // Monitoring: wide left + two stacked right
                            // Split right first, then split the right pane down,
                            // then go back to left pane (the wide main pane)
                            self.window.screen.split_right();
                            self.window.screen.split_down();
                            self.window.screen.context_manager.select_prev_split();
                            self.window.screen.context_manager.select_prev_split();
                        }
                        _ => {}
                    }
                    self.layouts_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::SessionSharing {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.sharing_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    if self.sharing_selected > 0 {
                        self.sharing_selected -= 1;
                    }
                }
                Key::Named(NamedKey::ArrowRight) => {
                    if self.sharing_selected < 1 {
                        self.sharing_selected += 1;
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    use crate::router::routes::session_sharing::SharingState;
                    match &self.sharing_state {
                        SharingState::Idle => {
                            if self.sharing_selected == 0 {
                                // Host — placeholder, start hosting on port 9876
                                self.sharing_state = SharingState::Hosting { port: 9876 };
                            } else {
                                // Connect — placeholder
                                self.sharing_state = SharingState::Connecting {
                                    host: "localhost:9876".to_string(),
                                };
                            }
                        }
                        _ => {}
                    }
                }
                Key::Character(c) if c.as_str() == "q" || c.as_str() == "Q" => {
                    self.sharing_state =
                        crate::router::routes::session_sharing::SharingState::Idle;
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::SessionExport {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.export_selected = 0;
                    self.export_result = None;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if self.export_selected > 0 {
                        self.export_selected -= 1;
                    }
                }
                Key::Named(NamedKey::ArrowDown) => {
                    if self.export_selected + 1
                        < crate::router::routes::session_export::EXPORT_FORMATS.len()
                    {
                        self.export_selected += 1;
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    // Perform the export
                    let entries = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(500);
                    let mut recording =
                        crate::session_export::SessionRecording::new(80, 24);
                    let mut ts = 0.0_f64;
                    for entry in entries.iter().rev() {
                        recording.add_output(ts, format!("$ {}\r\n", entry.command));
                        ts += entry.duration_ms.unwrap_or(100) as f64 / 1000.0;
                        if !entry.output_preview.is_empty() {
                            recording.add_output(ts, entry.output_preview.clone());
                            ts += 0.1;
                        }
                    }

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default();
                    let timestamp = now.as_secs();
                    let (ext, content) = match self.export_selected {
                        0 => (".cast", recording.to_asciinema()),
                        1 => (
                            ".txt",
                            self.window
                                .screen
                                .context_manager
                                .session_recorder
                                .export_text(),
                        ),
                        2 => (".html", recording.to_html()),
                        3 => (".json", recording.to_json()),
                        _ => (".txt", String::new()),
                    };

                    let filename = format!("volt-session-{}{}", timestamp, ext);
                    let desktop = dirs::desktop_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let path = desktop.join(&filename);
                    match std::fs::write(&path, &content) {
                        Ok(_) => {
                            self.export_result = Some(
                                crate::router::routes::session_export::ExportResult {
                                    message: format!("Saved to {}", path.display()),
                                    success: true,
                                },
                            );
                            tracing::info!("Exported session to {}", path.display());
                        }
                        Err(e) => {
                            self.export_result = Some(
                                crate::router::routes::session_export::ExportResult {
                                    message: format!("Failed: {}", e),
                                    success: false,
                                },
                            );
                        }
                    }
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::TimeTravel {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.time_travel_selected = 0;
                    self.time_travel_scroll = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    let max = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(100)
                        .len();
                    if max > 0 && self.time_travel_selected + 1 < max {
                        self.time_travel_selected += 1;
                    }
                    // Auto-scroll to keep selection visible
                    if self.time_travel_selected >= self.time_travel_scroll + 30 {
                        self.time_travel_scroll =
                            self.time_travel_selected.saturating_sub(29);
                    }
                }
                Key::Named(NamedKey::ArrowUp) if self.time_travel_selected > 0 => {
                    self.time_travel_selected -= 1;
                    if self.time_travel_selected < self.time_travel_scroll {
                        self.time_travel_scroll = self.time_travel_selected;
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    // Replay: paste the selected command into the terminal
                    let entries = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(100);
                    let display_order: Vec<_> = entries.into_iter().rev().collect();
                    if let Some(entry) = display_order.get(self.time_travel_selected) {
                        if !entry.command.is_empty() {
                            let cmd = entry.command.clone();
                            self.window
                                .screen
                                .ctx_mut()
                                .current_mut()
                                .messenger
                                .send_write(cmd.into_bytes());
                        }
                    }
                    self.time_travel_selected = 0;
                    self.time_travel_scroll = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Character(c) if c.as_str() == "c" || c.as_str() == "C" => {
                    // Copy selected command to clipboard
                    let entries = self
                        .window
                        .screen
                        .context_manager
                        .session_recorder
                        .recent(100);
                    let display_order: Vec<_> = entries.into_iter().rev().collect();
                    if let Some(entry) = display_order.get(self.time_travel_selected) {
                        if !entry.command.is_empty() {
                            self.window.screen.clipboard.borrow_mut().set(
                                rio_backend::clipboard::ClipboardType::Clipboard,
                                entry.command.clone(),
                            );
                        }
                    }
                }
                _ => {}
            }
            return true;
        }

        if self.path == RoutePath::TmuxPicker {
            if key_event.state != rio_window::event::ElementState::Pressed {
                return true;
            }

            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.tmux_selected = 0;
                    self.path = RoutePath::Terminal;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if self.tmux_selected > 0 {
                        self.tmux_selected -= 1;
                    }
                }
                Key::Named(NamedKey::ArrowDown) => {
                    let max = self.tmux_sessions.len();
                    if max > 0 && self.tmux_selected + 1 < max {
                        self.tmux_selected += 1;
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    if let Some((_id, name, _attached)) =
                        self.tmux_sessions.get(self.tmux_selected)
                    {
                        let cmd = format!("tmux attach -t {}\r", name);
                        self.window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_write(cmd.into_bytes());
                        self.path = RoutePath::Terminal;
                    }
                }
                Key::Character(c) if c.as_str() == "n" || c.as_str() == "N" => {
                    let cmd = "tmux new-session\r".to_string();
                    self.window
                        .screen
                        .ctx_mut()
                        .current_mut()
                        .messenger
                        .send_write(cmd.into_bytes());
                    self.path = RoutePath::Terminal;
                }
                // 'd' to detach selected session
                Key::Character(c) if c.as_str() == "d" || c.as_str() == "D" => {
                    if let Some((_id, name, _attached)) =
                        self.tmux_sessions.get(self.tmux_selected)
                    {
                        let cmd = format!("tmux detach-client -s {}\r", name);
                        self.window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_write(cmd.into_bytes());
                    }
                }
                // 'x' to kill selected session
                Key::Character(c) if c.as_str() == "x" || c.as_str() == "X" => {
                    if let Some((_id, name, _attached)) =
                        self.tmux_sessions.get(self.tmux_selected)
                    {
                        let cmd = format!("tmux kill-session -t {}\r", name);
                        self.window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_write(cmd.into_bytes());
                        // Refresh session list
                        self.tmux_sessions =
                            crate::tmux_cc::TmuxController::list_sessions();
                        if self.tmux_selected >= self.tmux_sessions.len()
                            && self.tmux_selected > 0
                        {
                            self.tmux_selected -= 1;
                        }
                    }
                }
                // 'r' to rename selected session
                Key::Character(c) if c.as_str() == "r" || c.as_str() == "R" => {
                    if let Some((_id, name, _attached)) =
                        self.tmux_sessions.get(self.tmux_selected)
                    {
                        // Use tmux rename — for now just send the command
                        let cmd = format!("tmux rename-session -t {} \r", name);
                        self.window
                            .screen
                            .ctx_mut()
                            .current_mut()
                            .messenger
                            .send_write(cmd.into_bytes());
                        self.path = RoutePath::Terminal;
                    }
                }
                _ => {}
            }
            return true;
        }

        false
    }
}

pub struct Router<'a> {
    pub routes: FxHashMap<WindowId, Route<'a>>,
    propagated_report: Option<RioError>,
    pub font_library: Box<rio_backend::sugarloaf::font::FontLibrary>,
    pub config_route: Option<WindowId>,
    pub clipboard: Rc<RefCell<Clipboard>>,
    current_tab_id: u64,
}

impl Router<'_> {
    pub fn new<'b>(
        fonts: rio_backend::sugarloaf::font::SugarloafFonts,
        clipboard: Clipboard,
    ) -> Router<'b> {
        let (font_library, fonts_not_found) =
            rio_backend::sugarloaf::font::FontLibrary::new(fonts);

        let mut propagated_report = None;

        if let Some(err) = fonts_not_found {
            propagated_report = Some(RioError {
                report: RioErrorType::FontsNotFound(err.fonts_not_found),
                level: RioErrorLevel::Warning,
            });
        }

        let clipboard = Rc::new(RefCell::new(clipboard));

        Router {
            routes: FxHashMap::default(),
            propagated_report,
            config_route: None,
            font_library: Box::new(font_library),
            clipboard,
            current_tab_id: 0,
        }
    }

    #[inline]
    pub fn propagate_error_to_next_route(&mut self, error: RioError) {
        self.propagated_report = Some(error);
    }

    #[inline]
    pub fn update_titles(&mut self) {
        for route in self.routes.values_mut() {
            if route.window.is_focused {
                route.window.screen.context_manager.update_titles();
            }
        }
    }

    #[inline]
    pub fn get_focused_route(&self) -> Option<WindowId> {
        self.routes
            .iter()
            .find_map(|(key, val)| {
                if val.window.winit_window.has_focus() {
                    Some(key)
                } else {
                    None
                }
            })
            .copied()
    }

    pub fn open_config_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventProxy,
        config: &RioConfig,
    ) {
        // In case configuration window does exists already
        if let Some(route_id) = self.config_route {
            if let Some(route) = self.routes.get(&route_id) {
                route.window.winit_window.focus_window();
                return;
            }
        }

        let current_config: RioConfig = config.clone();
        let editor = config.editor.clone();
        let mut args = editor.args;
        args.push(
            rio_backend::config::config_file_path()
                .display()
                .to_string(),
        );
        let new_config = RioConfig {
            shell: rio_backend::config::Shell {
                program: editor.program,
                args,
            },
            ..current_config
        };

        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            &new_config,
            &self.font_library,
            "Volt Settings",
            None,
            None,
            self.clipboard.clone(),
            None,
        );
        let id = window.winit_window.id();
        let route = Route::new(Assistant::new(), RoutePath::Terminal, window);
        self.routes.insert(id, route);
        self.config_route = Some(id);
    }

    pub fn open_config_split(&mut self, config: &RioConfig) {
        let current_config: RioConfig = config.clone();
        let editor = config.editor.clone();
        let mut args = editor.args;
        args.push(
            rio_backend::config::config_file_path()
                .display()
                .to_string(),
        );
        let new_config = RioConfig {
            shell: rio_backend::config::Shell {
                program: editor.program,
                args,
            },
            ..current_config
        };

        let window_id = match self.get_focused_route() {
            Some(window_id) => window_id,
            None => return,
        };

        let route = match self.routes.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        route.window.screen.split_right_with_config(new_config);
    }

    #[inline]
    pub fn create_window<'a>(
        &'a mut self,
        event_loop: &'a ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'a rio_backend::config::Config,
        open_url: Option<String>,
        app_id: Option<&str>,
    ) {
        let tab_id = if config.navigation.is_native() {
            let id = self.current_tab_id;
            self.current_tab_id = self.current_tab_id.wrapping_add(1);
            Some(id.to_string())
        } else {
            None
        };

        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            config,
            &self.font_library,
            RIO_TITLE,
            tab_id.as_deref(),
            open_url,
            self.clipboard.clone(),
            app_id,
        );
        let id = window.winit_window.id();

        let mut route = Route {
            window,
            path: RoutePath::Terminal,
            assistant: Assistant::new(),
            settings_editor: crate::settings_editor::SettingsEditor::new(),
            tmux_sessions: Vec::new(),
            tmux_selected: 0,
            env_scroll: 0,
            env_selected: 0,
            history_scroll: 0,
            history_selected: 0,
            bookmarks_scroll: 0,
            bookmarks_selected: 0,
            bookmarks_cache: Vec::new(),
            connections_selected: 0,
            connections_list: Vec::new(),
            slash_commands_scroll: 0,
            layouts_selected: 0,
            settings_category: 0,
            settings_in_sidebar: true,
            help_category: 0,
            help_selected: 0,
            help_in_sidebar: true,
            sharing_state: crate::router::routes::session_sharing::SharingState::default(
            ),
            sharing_selected: 0,
            export_selected: 0,
            export_result: None,
            time_travel_selected: 0,
            time_travel_scroll: 0,
        };

        if let Some(err) = &self.propagated_report {
            route.report_error(err);
            self.propagated_report = None;
        }

        self.routes.insert(id, route);
    }

    #[cfg(target_os = "macos")]
    #[inline]
    pub fn create_native_tab<'a>(
        &'a mut self,
        event_loop: &'a ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'a rio_backend::config::Config,
        tab_id: Option<&str>,
        open_url: Option<String>,
    ) {
        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            config,
            &self.font_library,
            RIO_TITLE,
            tab_id,
            open_url,
            self.clipboard.clone(),
            None,
        );
        self.routes.insert(
            window.winit_window.id(),
            Route {
                window,
                path: RoutePath::Terminal,
                assistant: Assistant::new(),
                settings_editor: crate::settings_editor::SettingsEditor::new(),
                tmux_sessions: Vec::new(),
                tmux_selected: 0,
                env_scroll: 0,
                env_selected: 0,
                history_scroll: 0,
                history_selected: 0,
                bookmarks_scroll: 0,
                bookmarks_selected: 0,
                bookmarks_cache: Vec::new(),
                connections_selected: 0,
                connections_list: Vec::new(),
                slash_commands_scroll: 0,
                layouts_selected: 0,
                settings_category: 0,
                settings_in_sidebar: true,
                help_category: 0,
                help_selected: 0,
                help_in_sidebar: true,
                sharing_state:
                    crate::router::routes::session_sharing::SharingState::default(),
                sharing_selected: 0,
                export_selected: 0,
                export_result: None,
                time_travel_selected: 0,
                time_travel_scroll: 0,
            },
        );
    }
}

pub struct RouteWindow<'a> {
    pub is_focused: bool,
    pub is_occluded: bool,
    pub needs_render_after_occlusion: bool,
    pub render_timestamp: Instant,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub vblank_interval: Duration,
    pub winit_window: Window,
    pub screen: Screen<'a>,

    #[cfg(target_os = "macos")]
    pub is_macos_deadzone: bool,
}

impl<'a> RouteWindow<'a> {
    pub fn configure_window(&mut self, config: &rio_backend::config::Config) {
        configure_window(&self.winit_window, config);
    }

    pub fn wait_until(&self) -> Option<Duration> {
        // If we need to render after occlusion, render immediately
        if self.needs_render_after_occlusion {
            return None;
        }

        // On macOS, CVDisplayLink handles VSync synchronization automatically,
        // so we don't need software-based frame timing calculations
        #[cfg(target_os = "macos")]
        {
            None
        }

        #[cfg(not(target_os = "macos"))]
        {
            let now = Instant::now();
            let elapsed = now.duration_since(self.render_timestamp);
            let vblank = self.vblank_interval;

            // Calculate how many complete frames have elapsed
            let frames_elapsed = elapsed.as_nanos() / vblank.as_nanos();

            // Calculate when the next frame should occur
            let next_frame_time = self.render_timestamp
                + Duration::from_nanos(
                    (frames_elapsed + 1) as u64 * vblank.as_nanos() as u64,
                );

            if next_frame_time > now {
                // Return the time to wait until the next ideal frame time
                Some(next_frame_time.duration_since(now))
            } else {
                // We've missed the target frame time, render immediately
                None
            }
        }
    }

    // TODO: Use it whenever animated cursor is done
    // pub fn request_animation_frame(&mut self) {
    //     if self.config.renderer.strategy.is_event_based() {
    //         // Schedule a render for the next frame time
    //         let route_id = self.window.screen.ctx().current_route();
    //         let timer_id = TimerId::new(Topic::RenderRoute, route_id);
    //         let event = EventPayload::new(
    //             RioEventType::Rio(RioEvent::RenderRoute(route_id)),
    //             self.window.winit_window.id(),
    //         );

    //         // Always schedule at the next vblank interval
    //         self.scheduler.schedule(event, self.window.vblank_interval, false, timer_id);
    //     } else {
    //         // For game loop rendering, the standard redraw is fine
    //         self.request_redraw();
    //     }
    // }

    #[inline]
    pub fn update_vblank_interval(&mut self) {
        // On macOS, CVDisplayLink handles VSync synchronization automatically,
        // so we don't need to calculate vblank intervals
        #[cfg(not(target_os = "macos"))]
        {
            // Always update vblank interval based on monitor refresh rate
            // Get the display refresh rate, default to 60Hz if unavailable
            let refresh_rate_hz = self
                .winit_window
                .current_monitor()
                .and_then(|monitor| monitor.refresh_rate_millihertz())
                .unwrap_or(60_000) as f64
                / 1000.0; // Convert millihertz to Hz

            // Calculate frame time in microseconds (1,000,000 µs / refresh_rate)
            let frame_time_us = (1_000_000.0 / refresh_rate_hz) as u64;
            self.vblank_interval = Duration::from_micros(frame_time_us);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_target<'b>(
        event_loop: &'b ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'b RioConfig,
        font_library: &rio_backend::sugarloaf::font::FontLibrary,
        window_name: &str,
        tab_id: Option<&str>,
        open_url: Option<String>,
        clipboard: Rc<RefCell<Clipboard>>,
        app_id: Option<&str>,
    ) -> RouteWindow<'a> {
        #[allow(unused_mut)]
        let mut window_builder =
            create_window_builder(window_name, config, tab_id, app_id);

        #[cfg(not(any(target_os = "macos", windows)))]
        if let Some(token) = event_loop.read_token_from_env() {
            tracing::debug!("Activating window with token: {token:?}");
            window_builder = window_builder.with_activation_token(token);

            // Remove the token from the env.
            startup_notify::reset_activation_token_env();
        }

        let winit_window = event_loop.create_window(window_builder).unwrap();
        configure_window(&winit_window, config);

        let properties = ScreenWindowProperties {
            size: winit_window.inner_size(),
            scale: winit_window.scale_factor(),
            raw_window_handle: winit_window.window_handle().unwrap().into(),
            raw_display_handle: winit_window.display_handle().unwrap().into(),
            window_id: winit_window.id(),
        };

        let screen = Screen::new(
            properties,
            config,
            event_proxy,
            font_library,
            open_url,
            clipboard,
        )
        .expect("Screen not created");

        #[cfg(target_os = "windows")]
        {
            // On windows cloak (hide) the window initially, we later reveal it after the first draw.
            // This is a workaround to hide the "white flash" that occurs during application startup.
            use rio_window::platform::windows::WindowExtWindows;
            winit_window.set_cloaked(false);
        }

        // Get the display refresh rate and convert to frame interval
        // On macOS, CVDisplayLink handles VSync synchronization automatically,
        // so we don't need to calculate vblank intervals
        #[cfg(target_os = "macos")]
        let monitor_vblank_interval = Duration::from_micros(16667); // Placeholder value, not used

        #[cfg(not(target_os = "macos"))]
        let monitor_vblank_interval = {
            let monitor_refresh_rate_hz = winit_window
                .current_monitor()
                .and_then(|monitor| monitor.refresh_rate_millihertz())
                .unwrap_or(60_000) as f64
                / 1000.0;

            // Convert to microseconds for precise frame timing
            let frame_time_us = (1_000_000.0 / monitor_refresh_rate_hz) as u64;
            Duration::from_micros(frame_time_us)
        };

        Self {
            vblank_interval: monitor_vblank_interval,
            render_timestamp: Instant::now(),
            is_focused: true,
            is_occluded: false,
            needs_render_after_occlusion: false,
            winit_window,
            screen,
            #[cfg(target_os = "macos")]
            is_macos_deadzone: false,
        }
    }
}
