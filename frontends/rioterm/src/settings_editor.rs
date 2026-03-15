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
        let indices = self.filtered_indices();
        if let Some(&idx) = indices.get(self.selected_index) {
            if let Some(new_val) =
                SettingValue::from_input(&self.edit_buffer, &self.items[idx].value)
            {
                self.items[idx].value = new_val;
                self.save_setting(idx);
            }
        }
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
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

        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        let mut doc: toml::Table = content.parse().unwrap_or_default();

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

        if let Ok(output) = toml::to_string_pretty(&doc) {
            let _ = std::fs::write(&config_path, output);
            tracing::info!("Setting saved: {} = {}", item.key, item.value.display());
        }
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
            value: SettingValue::String(format!("{:?}", config.window.decorations)),
            description: "Window chrome style: Enabled, Disabled, Transparent, Buttonless".into(),
        },
        SettingItem {
            key: "window.width".into(),
            label: "Window Width".into(),
            category: "Window".into(),
            value: SettingValue::Integer(config.window.width as i64),
            description: "Default window width in pixels".into(),
        },
        SettingItem {
            key: "window.height".into(),
            label: "Window Height".into(),
            category: "Window".into(),
            value: SettingValue::Integer(config.window.height as i64),
            description: "Default window height in pixels".into(),
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
            key: "navigation.open-config-with-split".into(),
            label: "Open Config in Split".into(),
            category: "Navigation".into(),
            value: SettingValue::Bool(config.navigation.open_config_with_split),
            description: "Open config file in a split pane rather than a new tab".into(),
        },
        // ── Colors ───────────────────────────────────────────
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
            description: "Scroll speed multiplier".into(),
        },
        SettingItem {
            key: "scroll.divider".into(),
            label: "Scroll Divider".into(),
            category: "Scroll".into(),
            value: SettingValue::Float(config.scroll.divider as f32),
            description: "Scroll speed divider".into(),
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
