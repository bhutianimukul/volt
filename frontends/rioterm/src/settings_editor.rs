//! Interactive settings editor — VS Code-style settings panel.
//! Navigate with arrow keys, Enter to edit, Escape to cancel.

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub key: String,
    pub label: String,
    pub category: String,
    pub value: SettingValue,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum SettingValue {
    String(String),
    Float(f32),
    Integer(i64),
    Bool(bool),
}

impl SettingValue {
    pub fn display(&self) -> String {
        match self {
            SettingValue::String(s) => s.clone(),
            SettingValue::Float(f) => format!("{:.1}", f),
            SettingValue::Integer(i) => i.to_string(),
            SettingValue::Bool(b) => {
                if *b {
                    "true"
                } else {
                    "false"
                }
                .to_string()
            }
        }
    }

    pub fn from_input(input: &str, template: &SettingValue) -> Option<SettingValue> {
        match template {
            SettingValue::String(_) => Some(SettingValue::String(input.to_string())),
            SettingValue::Float(_) => input.parse().ok().map(SettingValue::Float),
            SettingValue::Integer(_) => input.parse().ok().map(SettingValue::Integer),
            SettingValue::Bool(_) => match input.to_lowercase().as_str() {
                "true" | "yes" | "1" | "on" => Some(SettingValue::Bool(true)),
                "false" | "no" | "0" | "off" => Some(SettingValue::Bool(false)),
                _ => None,
            },
        }
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, SettingValue::Bool(_))
    }
}

#[derive(Debug)]
pub struct SettingsEditor {
    pub items: Vec<SettingItem>,
    pub selected_index: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub search_query: String,
    pub searching: bool,
    pub scroll_offset: usize,
    pub visible_rows: usize,
    /// When editing from the panel view, this stores the real index into `items`.
    pub editing_real_index: Option<usize>,
}

impl SettingsEditor {
    pub fn new() -> Self {
        Self {
            items: build_settings_list(),
            selected_index: 0,
            editing: false,
            edit_buffer: String::new(),
            search_query: String::new(),
            searching: false,
            scroll_offset: 0,
            visible_rows: 20,
            editing_real_index: None,
        }
    }

    pub fn reload_from_config(&mut self, config: &rio_backend::config::Config) {
        self.items = build_settings_from_config(config);
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
        }
    }

    pub fn move_down(&mut self) {
        let count = self.filtered_items().len();
        if count > 0 && self.selected_index + 1 < count {
            self.selected_index += 1;
            self.ensure_visible();
        }
    }

    pub fn start_editing(&mut self) {
        if let Some(item) = self.filtered_items().get(self.selected_index) {
            self.edit_buffer = item.value.display();
            self.editing = true;
        }
    }

    pub fn toggle_bool(&mut self) {
        let indices = self.filtered_indices();
        if let Some(&idx) = indices.get(self.selected_index) {
            if let SettingValue::Bool(val) = &self.items[idx].value {
                self.items[idx].value = SettingValue::Bool(!val);
                self.save_setting(idx);
            }
        }
    }

    pub fn confirm_edit(&mut self) {
        if !self.editing {
            return;
        }
        // Use editing_real_index if available (panel mode), otherwise fall back to filtered
        let idx = if let Some(ri) = self.editing_real_index {
            Some(ri)
        } else {
            let indices = self.filtered_indices();
            indices.get(self.selected_index).copied()
        };
        if let Some(idx) = idx {
            if let Some(new_val) =
                SettingValue::from_input(&self.edit_buffer, &self.items[idx].value)
            {
                self.items[idx].value = new_val;
                self.save_setting(idx);
            }
        }
        self.editing = false;
        self.edit_buffer.clear();
        self.editing_real_index = None;
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
        self.editing_real_index = None;
    }

    pub fn type_char(&mut self, c: char) {
        if self.editing {
            self.edit_buffer.push(c);
        } else if self.searching {
            self.search_query.push(c);
            self.selected_index = 0;
        }
    }

    pub fn backspace(&mut self) {
        if self.editing {
            self.edit_buffer.pop();
        } else if self.searching {
            self.search_query.pop();
        }
    }

    pub fn toggle_search(&mut self) {
        self.searching = !self.searching;
        if !self.searching {
            self.search_query.clear();
            self.selected_index = 0;
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            (0..self.items.len()).collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.label.to_lowercase().contains(&q)
                        || item.key.to_lowercase().contains(&q)
                        || item.category.to_lowercase().contains(&q)
                        || item.description.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect()
        }
    }

    pub fn filtered_items(&self) -> Vec<&SettingItem> {
        self.filtered_indices()
            .iter()
            .filter_map(|&i| self.items.get(i))
            .collect()
    }

    fn ensure_visible(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.visible_rows {
            self.scroll_offset = self.selected_index - self.visible_rows + 1;
        }
    }

    fn save_setting(&self, idx: usize) {
        let item = &self.items[idx];
        let config_path = rio_backend::config::config_file_path();

        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Cannot read config: {}", e);
                return;
            }
        };
        let mut doc: toml::Table = match content.parse() {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Invalid TOML: {}", e);
                return;
            }
        };

        let parts: Vec<&str> = item.key.splitn(2, '.').collect();
        match parts.len() {
            2 => {
                let section = doc
                    .entry(parts[0])
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                if let toml::Value::Table(table) = section {
                    insert_value(table, parts[1], &item.value);
                }
            }
            1 => {
                insert_value(&mut doc, parts[0], &item.value);
            }
            _ => {}
        }

        match toml::to_string_pretty(&doc) {
            Ok(output) => {
                match std::fs::write(&config_path, &output) {
                    Ok(_) => tracing::info!("Setting saved: {} = {}", item.key, item.value.display()),
                    Err(e) => tracing::error!("Failed to write config: {}", e),
                }
            }
            Err(e) => tracing::error!("Failed to serialize config: {}", e),
        }
    }

    /// Returns the ordered list of unique category names.
    pub fn categories(&self) -> Vec<String> {
        let mut cats = Vec::new();
        for item in &self.items {
            if !cats.contains(&item.category) {
                cats.push(item.category.clone());
            }
        }
        cats
    }

    /// Returns items belonging to a specific category.
    pub fn items_for_category(&self, category: &str) -> Vec<&SettingItem> {
        self.items.iter().filter(|it| it.category == category).collect()
    }

    /// Public wrapper for save_setting so the router can call it.
    pub fn save_setting_by_index(&self, idx: usize) {
        self.save_setting(idx);
    }

    /// Returns true if the selected item is a bool (for toggle-on-Enter behavior).
    pub fn selected_is_bool(&self) -> bool {
        let indices = self.filtered_indices();
        if let Some(&idx) = indices.get(self.selected_index) {
            self.items[idx].value.is_bool()
        } else {
            false
        }
    }
}

fn insert_value(table: &mut toml::Table, key: &str, value: &SettingValue) {
    match value {
        SettingValue::String(s) => {
            table.insert(key.to_string(), toml::Value::String(s.clone()));
        }
        SettingValue::Float(f) => {
            table.insert(key.to_string(), toml::Value::Float(*f as f64));
        }
        SettingValue::Integer(i) => {
            table.insert(key.to_string(), toml::Value::Integer(*i));
        }
        SettingValue::Bool(b) => {
            table.insert(key.to_string(), toml::Value::Boolean(*b));
        }
    }
}

impl Default for SettingsEditor {
    fn default() -> Self {
        Self::new()
    }
}

fn build_settings_list() -> Vec<SettingItem> {
    build_settings_from_config(&rio_backend::config::Config::default())
}

/// Convert a ColorArray ([f32; 4] in 0.0..1.0 range) to a hex string like "#RRGGBB".
fn color_arr_to_hex(c: &[f32; 4]) -> String {
    let r = (c[0] * 255.0).round() as u8;
    let g = (c[1] * 255.0).round() as u8;
    let b = (c[2] * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

pub fn build_settings_from_config(config: &rio_backend::config::Config) -> Vec<SettingItem> {
    vec![
        // ── Font ──────────────────────────────────────────────
        SettingItem {
            key: "fonts.size".into(),
            label: "Font Size".into(),
            category: "Font".into(),
            value: SettingValue::Float(config.fonts.size),
            description: "Terminal font size in points".into(),
        },
        SettingItem {
            key: "fonts.family".into(),
            label: "Font Family".into(),
            category: "Font".into(),
            value: SettingValue::String(
                config.fonts.family.clone().unwrap_or_default(),
            ),
            description: "Font family name (e.g. 'JetBrains Mono', 'Fira Code')".into(),
        },
        SettingItem {
            key: "fonts.hinting".into(),
            label: "Font Hinting".into(),
            category: "Font".into(),
            value: SettingValue::Bool(config.fonts.hinting),
            description: "Enable font hinting for sharper rendering".into(),
        },
        SettingItem {
            key: "line-height".into(),
            label: "Line Height".into(),
            category: "Font".into(),
            value: SettingValue::Float(config.line_height),
            description: "Line height multiplier (1.0 = default)".into(),
        },
        // ── Window ────────────────────────────────────────────
        SettingItem {
            key: "window.width".into(),
            label: "Window Width".into(),
            category: "Window".into(),
            value: SettingValue::Integer(config.window.width as i64),
            description: "Initial window width in pixels".into(),
        },
        SettingItem {
            key: "window.height".into(),
            label: "Window Height".into(),
            category: "Window".into(),
            value: SettingValue::Integer(config.window.height as i64),
            description: "Initial window height in pixels".into(),
        },
        SettingItem {
            key: "window.mode".into(),
            label: "Window Mode".into(),
            category: "Window".into(),
            value: SettingValue::String(format!("{:?}", config.window.mode).to_lowercase()),
            description: "Window mode: windowed, maximized, fullscreen".into(),
        },
        SettingItem {
            key: "window.opacity".into(),
            label: "Window Opacity".into(),
            category: "Window".into(),
            value: SettingValue::Float(config.window.opacity),
            description: "Window transparency (0.0 = transparent, 1.0 = opaque)".into(),
        },
        SettingItem {
            key: "window.blur".into(),
            label: "Background Blur".into(),
            category: "Window".into(),
            value: SettingValue::Bool(config.window.blur),
            description: "Enable background blur behind the terminal window".into(),
        },
        SettingItem {
            key: "window.decorations".into(),
            label: "Window Decorations".into(),
            category: "Window".into(),
            value: SettingValue::String(format!("{:?}", config.window.decorations).to_lowercase()),
            description: "Window chrome style: enabled, disabled, transparent, buttonless".into(),
        },
        SettingItem {
            key: "window.background-image".into(),
            label: "Background Image".into(),
            category: "Window".into(),
            value: SettingValue::String(
                config.window.background_image.as_ref()
                    .map(|img| img.path.clone())
                    .unwrap_or_default(),
            ),
            description: "Path to a background image file (PNG, JPG, etc.)".into(),
        },
        SettingItem {
            key: "window.macos-use-unified-titlebar".into(),
            label: "macOS Unified Titlebar".into(),
            category: "Window".into(),
            value: SettingValue::Bool(config.window.macos_use_unified_titlebar),
            description: "Use unified titlebar style on macOS".into(),
        },
        SettingItem {
            key: "window.macos-use-shadow".into(),
            label: "macOS Window Shadow".into(),
            category: "Window".into(),
            value: SettingValue::Bool(config.window.macos_use_shadow),
            description: "Enable window shadow on macOS".into(),
        },
        SettingItem {
            key: "window.initial-title".into(),
            label: "Initial Window Title".into(),
            category: "Window".into(),
            value: SettingValue::String(
                config.window.initial_title.clone().unwrap_or_default(),
            ),
            description: "Custom initial window title (leave empty for default)".into(),
        },
        SettingItem {
            key: "window.colorspace".into(),
            label: "Colorspace".into(),
            category: "Window".into(),
            value: SettingValue::String(format!("{:?}", config.window.colorspace).to_lowercase()),
            description: "Display colorspace: srgb, display-p3, rec2020".into(),
        },
        SettingItem {
            key: "padding-x".into(),
            label: "Padding X".into(),
            category: "Window".into(),
            value: SettingValue::Float(config.padding_x),
            description: "Horizontal padding in pixels".into(),
        },
        // ── Navigation ───────────────────────────────────────
        SettingItem {
            key: "navigation.mode".into(),
            label: "Tab Mode".into(),
            category: "Navigation".into(),
            value: SettingValue::String(config.navigation.mode.to_string()),
            description: "Tab bar style: TopTab, BottomTab, Bookmark, Plain, NativeTab".into(),
        },
        SettingItem {
            key: "navigation.hide-if-single".into(),
            label: "Hide Tab Bar (single tab)".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.hide_if_single),
            description: "Hide the tab bar when there is only one tab".into(),
        },
        SettingItem {
            key: "navigation.use-split".into(),
            label: "Enable Split Panes".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.use_split),
            description: "Allow splitting the terminal into panes".into(),
        },
        SettingItem {
            key: "navigation.clickable".into(),
            label: "Clickable Navigation".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.clickable),
            description: "Make navigation elements clickable with the mouse".into(),
        },
        SettingItem {
            key: "navigation.use-terminal-title".into(),
            label: "Use Terminal Title".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.use_terminal_title),
            description: "Show the terminal-reported title in tabs".into(),
        },
        SettingItem {
            key: "navigation.current-working-directory".into(),
            label: "Show Current Working Dir".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.current_working_directory),
            description: "Display the current working directory in navigation".into(),
        },
        SettingItem {
            key: "navigation.open-config-with-split".into(),
            label: "Open Config in Split".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.open_config_with_split),
            description: "Open config file in a split pane rather than a new tab".into(),
        },
        SettingItem {
            key: "navigation.unfocused-split-opacity".into(),
            label: "Unfocused Split Opacity".into(),
            category: "Navigation".into(),
            value: SettingValue::Float(config.navigation.unfocused_split_opacity),
            description: "Opacity of unfocused split panes (0.0 to 1.0)".into(),
        },
        // ── Colors — Primary ────────────────────────────────
        SettingItem {
            key: "colors.background".into(),
            label: "Background Color".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.background.0)),
            description: "Terminal background color (hex, e.g. #1E1E2E)".into(),
        },
        SettingItem {
            key: "colors.foreground".into(),
            label: "Foreground Color".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.foreground)),
            description: "Terminal text/foreground color (hex, e.g. #CDD6F4)".into(),
        },
        SettingItem {
            key: "colors.cursor".into(),
            label: "Cursor Color".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.cursor)),
            description: "Cursor color (hex, e.g. #F5E0DC)".into(),
        },
        SettingItem {
            key: "colors.vi-cursor".into(),
            label: "Vi Cursor Color".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.vi_cursor)),
            description: "Vi mode cursor color (hex)".into(),
        },
        // ── Colors — Selection ──────────────────────────────
        SettingItem {
            key: "colors.selection-background".into(),
            label: "Selection Background".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.selection_background)),
            description: "Background color for selected text (hex)".into(),
        },
        SettingItem {
            key: "colors.selection-foreground".into(),
            label: "Selection Foreground".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.selection_foreground)),
            description: "Foreground color for selected text (hex)".into(),
        },
        // ── Colors — Tab Bar ────────────────────────────────
        SettingItem {
            key: "colors.tabs".into(),
            label: "Tabs Background".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.tabs)),
            description: "Inactive tab background color (hex)".into(),
        },
        SettingItem {
            key: "colors.tabs-active".into(),
            label: "Active Tab Background".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.tabs_active)),
            description: "Active tab background color (hex)".into(),
        },
        SettingItem {
            key: "colors.tabs-active-foreground".into(),
            label: "Active Tab Foreground".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.tabs_active_foreground)),
            description: "Active tab text color (hex)".into(),
        },
        SettingItem {
            key: "colors.tabs-foreground".into(),
            label: "Tabs Foreground".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.tabs_foreground)),
            description: "Inactive tab text color (hex)".into(),
        },
        SettingItem {
            key: "colors.tabs-active-highlight".into(),
            label: "Active Tab Highlight".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.tabs_active_highlight)),
            description: "Highlight color on the active tab (hex)".into(),
        },
        SettingItem {
            key: "colors.bar".into(),
            label: "Bar Color".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.bar)),
            description: "Navigation bar background color (hex)".into(),
        },
        SettingItem {
            key: "colors.split".into(),
            label: "Split Divider Color".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.split)),
            description: "Color of the split pane divider (hex)".into(),
        },
        // ── Colors — ANSI Normal ────────────────────────────
        SettingItem {
            key: "colors.black".into(),
            label: "Black (ANSI 0)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.black)),
            description: "ANSI black color (hex)".into(),
        },
        SettingItem {
            key: "colors.red".into(),
            label: "Red (ANSI 1)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.red)),
            description: "ANSI red color (hex)".into(),
        },
        SettingItem {
            key: "colors.green".into(),
            label: "Green (ANSI 2)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.green)),
            description: "ANSI green color (hex)".into(),
        },
        SettingItem {
            key: "colors.yellow".into(),
            label: "Yellow (ANSI 3)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.yellow)),
            description: "ANSI yellow color (hex)".into(),
        },
        SettingItem {
            key: "colors.blue".into(),
            label: "Blue (ANSI 4)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.blue)),
            description: "ANSI blue color (hex)".into(),
        },
        SettingItem {
            key: "colors.magenta".into(),
            label: "Magenta (ANSI 5)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.magenta)),
            description: "ANSI magenta color (hex)".into(),
        },
        SettingItem {
            key: "colors.cyan".into(),
            label: "Cyan (ANSI 6)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.cyan)),
            description: "ANSI cyan color (hex)".into(),
        },
        SettingItem {
            key: "colors.white".into(),
            label: "White (ANSI 7)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.white)),
            description: "ANSI white color (hex)".into(),
        },
        // ── Colors — ANSI Light/Bright ──────────────────────
        SettingItem {
            key: "colors.light-black".into(),
            label: "Light Black (ANSI 8)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_black)),
            description: "ANSI bright black color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-red".into(),
            label: "Light Red (ANSI 9)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_red)),
            description: "ANSI bright red color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-green".into(),
            label: "Light Green (ANSI 10)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_green)),
            description: "ANSI bright green color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-yellow".into(),
            label: "Light Yellow (ANSI 11)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_yellow)),
            description: "ANSI bright yellow color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-blue".into(),
            label: "Light Blue (ANSI 12)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_blue)),
            description: "ANSI bright blue color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-magenta".into(),
            label: "Light Magenta (ANSI 13)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_magenta)),
            description: "ANSI bright magenta color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-cyan".into(),
            label: "Light Cyan (ANSI 14)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_cyan)),
            description: "ANSI bright cyan color (hex)".into(),
        },
        SettingItem {
            key: "colors.light-white".into(),
            label: "Light White (ANSI 15)".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.light_white)),
            description: "ANSI bright white color (hex)".into(),
        },
        // ── Colors — Search ─────────────────────────────────
        SettingItem {
            key: "colors.search-match-background".into(),
            label: "Search Match Background".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.search_match_background)),
            description: "Background color for search matches (hex)".into(),
        },
        SettingItem {
            key: "colors.search-match-foreground".into(),
            label: "Search Match Foreground".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.search_match_foreground)),
            description: "Foreground color for search matches (hex)".into(),
        },
        SettingItem {
            key: "colors.search-focused-match-background".into(),
            label: "Focused Match Background".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.search_focused_match_background)),
            description: "Background color for the focused search match (hex)".into(),
        },
        SettingItem {
            key: "colors.search-focused-match-foreground".into(),
            label: "Focused Match Foreground".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.search_focused_match_foreground)),
            description: "Foreground color for the focused search match (hex)".into(),
        },
        // ── Colors — Hints ──────────────────────────────────
        SettingItem {
            key: "colors.hint-foreground".into(),
            label: "Hint Foreground".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.hint_foreground)),
            description: "Text color for keyboard hints (hex)".into(),
        },
        SettingItem {
            key: "colors.hint-background".into(),
            label: "Hint Background".into(),
            category: "Colors".into(),
            value: SettingValue::String(color_arr_to_hex(&config.colors.hint_background)),
            description: "Background color for keyboard hints (hex)".into(),
        },
        // ── Shell ────────────────────────────────────────────
        SettingItem {
            key: "shell.program".into(),
            label: "Shell Program".into(),
            category: "Shell".into(),
            value: SettingValue::String(config.shell.program.clone()),
            description: "Shell executable path (e.g. /bin/zsh, /usr/bin/fish)".into(),
        },
        SettingItem {
            key: "working-dir".into(),
            label: "Working Directory".into(),
            category: "Shell".into(),
            value: SettingValue::String(
                config.working_dir.clone().unwrap_or_default(),
            ),
            description: "Default working directory for new tabs".into(),
        },
        SettingItem {
            key: "env-vars".into(),
            label: "Environment Variables".into(),
            category: "Shell".into(),
            value: SettingValue::String(config.env_vars.join(", ")),
            description: "Extra environment variables (comma-separated KEY=VALUE pairs)".into(),
        },
        SettingItem {
            key: "editor.program".into(),
            label: "Editor Program".into(),
            category: "Shell".into(),
            value: SettingValue::String(config.editor.program.clone()),
            description: "Editor to use when opening config files (e.g. vi, nano, code)".into(),
        },
        // ── General ──────────────────────────────────────────
        SettingItem {
            key: "confirm-before-quit".into(),
            label: "Confirm Before Quit".into(),
            category: "General".into(),
            value: SettingValue::Bool(config.confirm_before_quit),
            description: "Show confirmation dialog when quitting".into(),
        },
        SettingItem {
            key: "use-fork".into(),
            label: "Use Fork".into(),
            category: "General".into(),
            value: SettingValue::Bool(config.use_fork),
            description: "Use fork() to spawn shell processes".into(),
        },
        SettingItem {
            key: "hide-mouse-cursor-when-typing".into(),
            label: "Hide Mouse When Typing".into(),
            category: "General".into(),
            value: SettingValue::Bool(config.hide_cursor_when_typing),
            description: "Hide mouse cursor while typing".into(),
        },
        SettingItem {
            key: "option-as-alt".into(),
            label: "Option as Alt".into(),
            category: "General".into(),
            value: SettingValue::String(config.option_as_alt.clone()),
            description: "Use Option key as Alt: left, right, both".into(),
        },
        SettingItem {
            key: "ignore-selection-foreground-color".into(),
            label: "Ignore Selection FG Color".into(),
            category: "General".into(),
            value: SettingValue::Bool(config.ignore_selection_fg_color),
            description: "Ignore foreground color in selections".into(),
        },
        SettingItem {
            key: "draw-bold-text-with-light-colors".into(),
            label: "Bold Text Light Colors".into(),
            category: "General".into(),
            value: SettingValue::Bool(config.draw_bold_text_with_light_colors),
            description: "Render bold text with lighter colors".into(),
        },
        SettingItem {
            key: "theme".into(),
            label: "Theme".into(),
            category: "General".into(),
            value: SettingValue::String(config.theme.clone()),
            description: "Theme name (looks for <name>.toml in themes directory)".into(),
        },
        // ── Cursor ───────────────────────────────────────────
        SettingItem {
            key: "cursor.shape".into(),
            label: "Cursor Shape".into(),
            category: "Cursor".into(),
            value: SettingValue::String(format!("{:?}", config.cursor.shape)),
            description: "Cursor shape: Block, Underline, Beam, Hidden".into(),
        },
        SettingItem {
            key: "cursor.blinking".into(),
            label: "Cursor Blinking".into(),
            category: "Cursor".into(),
            value: SettingValue::Bool(config.cursor.blinking),
            description: "Enable cursor blinking animation".into(),
        },
        SettingItem {
            key: "cursor.blinking-interval".into(),
            label: "Cursor Blink Interval".into(),
            category: "Cursor".into(),
            value: SettingValue::Integer(config.cursor.blinking_interval as i64),
            description: "Cursor blink interval in milliseconds".into(),
        },
        // ── Scroll ───────────────────────────────────────────
        SettingItem {
            key: "scroll.multiplier".into(),
            label: "Scroll Multiplier".into(),
            category: "Scroll".into(),
            value: SettingValue::Float(config.scroll.multiplier as f32),
            description: "Scroll speed multiplier (higher = faster scrolling)".into(),
        },
        SettingItem {
            key: "scroll.divider".into(),
            label: "Scroll Divider".into(),
            category: "Scroll".into(),
            value: SettingValue::Float(config.scroll.divider as f32),
            description: "Scroll speed divider (higher = slower scrolling)".into(),
        },
        // ── Renderer ─────────────────────────────────────────
        SettingItem {
            key: "renderer.performance".into(),
            label: "Renderer Performance".into(),
            category: "Renderer".into(),
            value: SettingValue::String(config.renderer.performance.to_string()),
            description: "Renderer performance mode: High, Low".into(),
        },
        SettingItem {
            key: "renderer.backend".into(),
            label: "Renderer Backend".into(),
            category: "Renderer".into(),
            value: SettingValue::String(config.renderer.backend.to_string()),
            description: "GPU backend: Automatic, Vulkan, GL, DX12, Metal".into(),
        },
        SettingItem {
            key: "renderer.strategy".into(),
            label: "Renderer Strategy".into(),
            category: "Renderer".into(),
            value: SettingValue::String(format!("{:?}", config.renderer.strategy).to_lowercase()),
            description: "Rendering strategy: events (redraw on change), game (continuous redraw)".into(),
        },
        SettingItem {
            key: "renderer.disable-unfocused-render".into(),
            label: "Disable Unfocused Render".into(),
            category: "Renderer".into(),
            value: SettingValue::Bool(config.renderer.disable_unfocused_render),
            description: "Stop rendering when the window loses focus".into(),
        },
        SettingItem {
            key: "renderer.disable-occluded-render".into(),
            label: "Disable Occluded Render".into(),
            category: "Renderer".into(),
            value: SettingValue::Bool(config.renderer.disable_occluded_render),
            description: "Stop rendering when the window is fully occluded".into(),
        },
        // ── Keyboard ────────────────────────────────────────
        SettingItem {
            key: "keyboard.disable-ctlseqs-alt".into(),
            label: "Disable Ctrl Seqs Alt".into(),
            category: "Keyboard".into(),
            value: SettingValue::Bool(config.keyboard.disable_ctlseqs_alt),
            description: "Disable control sequences triggered by Alt key combinations".into(),
        },
        SettingItem {
            key: "keyboard.ime-cursor-positioning".into(),
            label: "IME Cursor Positioning".into(),
            category: "Keyboard".into(),
            value: SettingValue::Bool(config.keyboard.ime_cursor_positioning),
            description: "Position IME input popup at the cursor location".into(),
        },
        // ── Title ───────────────────────────────────────────
        SettingItem {
            key: "title.placeholder".into(),
            label: "Title Placeholder".into(),
            category: "Title".into(),
            value: SettingValue::String(
                config.title.placeholder.clone().unwrap_or_default(),
            ),
            description: "Placeholder text for the window title when no title is set".into(),
        },
        SettingItem {
            key: "title.content".into(),
            label: "Title Content".into(),
            category: "Title".into(),
            value: SettingValue::String(config.title.content.clone()),
            description: "Window title content (leave empty for default behavior)".into(),
        },
        // ── Bell ────────────────────────────────────────────
        SettingItem {
            key: "bell.visual".into(),
            label: "Visual Bell".into(),
            category: "Bell".into(),
            value: SettingValue::Bool(config.bell.visual),
            description: "Flash the screen on bell character".into(),
        },
        SettingItem {
            key: "bell.audio".into(),
            label: "Audio Bell".into(),
            category: "Bell".into(),
            value: SettingValue::Bool(config.bell.audio),
            description: "Play a sound on bell character".into(),
        },
        // ── Hints ───────────────────────────────────────────
        SettingItem {
            key: "hints.alphabet".into(),
            label: "Hints Alphabet".into(),
            category: "Hints".into(),
            value: SettingValue::String(config.hints.alphabet.clone()),
            description: "Characters used for generating hint labels".into(),
        },
        // ── Developer ────────────────────────────────────────
        SettingItem {
            key: "developer.log-level".into(),
            label: "Log Level".into(),
            category: "Developer".into(),
            value: SettingValue::String(config.developer.log_level.clone()),
            description: "Logging level: OFF, ERROR, WARN, INFO, DEBUG, TRACE".into(),
        },
        SettingItem {
            key: "developer.enable-log-file".into(),
            label: "Enable Log File".into(),
            category: "Developer".into(),
            value: SettingValue::Bool(config.developer.enable_log_file),
            description: "Write logs to ~/.config/volt/log/volt.log".into(),
        },
        SettingItem {
            key: "developer.enable-fps-counter".into(),
            label: "FPS Counter".into(),
            category: "Developer".into(),
            value: SettingValue::Bool(config.developer.enable_fps_counter),
            description: "Show FPS counter overlay".into(),
        },
    ]
}
